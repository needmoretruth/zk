//! A ledger and the wallets that use it, and the one path every shielded transaction takes:
//! plan, prove, verify, decide, record, deliver.

use std::collections::BTreeMap;
use std::time::Instant;

use zk_circuit::{Assignment, ZkField};
use zk_core::{Control, FieldBytes};

use crate::element::{from_hex, note_value, random, signed, to_hex};
use crate::error::PoolError;
use crate::keys::{Keys, micros};
use crate::ledger::Ledger;
use crate::plan::{Input, Output, Plan};
use crate::receipt::{
    Activity, InputNote, NoteDelivery, OutputNote, PrivatePart, PublicPart, Receipt,
    ShieldedPublic, SpendCircuit,
};
use crate::view::WalletView;
use crate::wallet::Received;

/// Who an output pays and how much; `None` pays a random address nobody holds.
pub(crate) type Pay<'a> = (Option<&'a str>, i64);

/// A ledger with its wallets. The pool's own world is saved to disk; an attack's is a scratch world
/// that lives only in memory.
#[derive(Clone, Debug)]
pub(crate) struct World<F> {
    /// The circuit this world's ledger verifies against.
    pub(crate) circuit: SpendCircuit,
    /// The public ledger.
    pub(crate) ledger: Ledger<F>,
    /// Every wallet, by name.
    pub(crate) wallets: BTreeMap<String, WalletView>,
}

/// A spend of an existing note, described by who holds what.
pub(crate) struct NoteSpend<'a> {
    /// The activity the receipt names.
    pub(crate) activity: Activity,
    /// Whose wallet holds the note.
    pub(crate) owner: &'a str,
    /// The note's index in that wallet.
    pub(crate) note: usize,
    /// Whose key the prover uses; the owner's, except in a theft.
    pub(crate) key_holder: &'a str,
    /// The two outputs.
    pub(crate) pay: [Pay<'a>; 2],
    /// Public value taken out to `key_holder`'s transparent account.
    pub(crate) v_pub_out: u16,
    /// Publish a random nullifier instead of the note's own.
    pub(crate) forge_nullifier: bool,
}

/// A planned transaction and what happens to the wallets if the ledger accepts it.
pub(crate) struct Spend<F> {
    activity: Activity,
    plan: Plan<F>,
    account: Option<String>,
    input: InputNote,
    spends: Option<(String, usize)>,
    pay: [(Option<String>, i64); 2],
}

/// What executing a spend produced.
pub(crate) struct Executed<F> {
    /// The receipt for the screen.
    pub(crate) receipt: Receipt,
    /// The values proved, so an attack can show which assertions they break.
    pub(crate) assignment: Assignment<F>,
    /// Wallets whose files changed.
    pub(crate) touched: Vec<String>,
}

impl<F: ZkField> World<F> {
    /// An empty world verified by `system` against `circuit`.
    pub(crate) fn new(system: &str, circuit: SpendCircuit) -> Self {
        Self { circuit, ledger: Ledger::new(system, circuit), wallets: BTreeMap::new() }
    }

    /// The wallet called `name`.
    pub(crate) fn wallet(&self, name: &str) -> Result<&WalletView, PoolError> {
        self.wallets.get(name).ok_or_else(|| PoolError::UnknownWallet(name.to_string()))
    }

    fn wallet_mut(&mut self, name: &str) -> Result<&mut WalletView, PoolError> {
        self.wallets.get_mut(name).ok_or_else(|| PoolError::UnknownWallet(name.to_string()))
    }

    /// A shield: a dummy input, `amount` of public value in from `name`'s transparent account, a
    /// note to `name` and a zero-value padding note.
    pub(crate) fn shield(&self, name: &str, amount: u16) -> Result<Spend<F>, PoolError> {
        let pay = [(Some(name), i64::from(amount)), (None, 0)];
        let plan = Plan {
            input: Input::Dummy { sk: random()?, rcm: random()? },
            outputs: self.outputs(pay)?,
            v_pub_in: amount,
            v_pub_out: 0,
            forged_nullifier: None,
        };
        Ok(Spend {
            activity: Activity::Shield,
            plan,
            account: Some(name.to_string()),
            input: InputNote::Dummy,
            spends: None,
            pay: owned(pay),
        })
    }

    /// A spend of one existing note.
    pub(crate) fn spend_note(&self, request: NoteSpend<'_>) -> Result<Spend<F>, PoolError> {
        let owner = self.wallet(request.owner)?;
        let note = owner.notes.get(request.note).ok_or_else(|| {
            PoolError::Corrupt(format!("{} has no note {}", request.owner, request.note))
        })?;
        let position = usize::try_from(note.position)
            .map_err(|_| PoolError::Corrupt("note position overflows".into()))?;
        let input = Input::Note {
            sk: self.wallet(request.key_holder)?.key()?,
            value: F::from_u64(u64::from(note.value)),
            rcm: from_hex(&note.rcm)?,
            position,
        };
        let plan = Plan {
            input,
            outputs: self.outputs(request.pay)?,
            v_pub_in: 0,
            v_pub_out: request.v_pub_out,
            forged_nullifier: if request.forge_nullifier { Some(random()?) } else { None },
        };
        Ok(Spend {
            activity: request.activity,
            plan,
            account: (request.v_pub_out > 0).then(|| request.key_holder.to_string()),
            input: InputNote::Note {
                owner: request.owner.to_string(),
                key_holder: request.key_holder.to_string(),
                value: note.value,
                position: note.position,
            },
            spends: Some((request.owner.to_string(), request.note)),
            pay: owned(request.pay),
        })
    }

    fn outputs(&self, pay: [Pay<'_>; 2]) -> Result<[Output<F>; 2], PoolError> {
        let output = |(recipient, value): Pay<'_>| -> Result<Output<F>, PoolError> {
            let pk = match recipient {
                Some(name) => self.wallet(name)?.address_element()?,
                None => random()?,
            };
            Ok(Output { pk, value: signed(value), rcm: random()? })
        };
        Ok([output(pay[0])?, output(pay[1])?])
    }

    /// Proves the spend with this world's circuit, has the ledger verify and decide, and on
    /// acceptance records it and hands the new notes to their wallets.
    pub(crate) fn execute(
        &mut self,
        keys: &mut Keys,
        control: &Control,
        spend: Spend<F>,
    ) -> Result<Executed<F>, PoolError> {
        let assignment = spend.plan.assignment(self.ledger.tree())?;
        let system = keys.system_id();
        let (prepared, setup_micros) = keys.prepared(self.circuit, control)?;
        let [public, private] = [&assignment.public, &assignment.private].map(|values| {
            values.iter().map(|value| value.to_le_bytes()).collect::<Vec<FieldBytes>>()
        });
        let started = Instant::now();
        let proven = prepared.prove_assignment(&public, &private, control)?;
        let prove_micros = micros(started);
        let started = Instant::now();
        let verdict = prepared.verify(&proven.public, &proven.proof, control)?;
        let verify_micros = micros(started);
        let mut shielded = self.public_part(system, &spend, &assignment, &proven.proof);
        (shielded.prove_micros, shielded.verify_micros) = (prove_micros, verify_micros);
        let decision = self.ledger.decide(verdict, &shielded, spend.account.as_deref());
        let mut public = PublicPart { shielded: Some(shielded.clone()), ..PublicPart::default() };
        let mut touched = Vec::new();
        if decision.accepted {
            let record = self.ledger.record(shielded, spend.account.as_deref())?;
            let positions = record.shielded.as_ref().and_then(|s| s.positions).unwrap_or_default();
            touched = self.deliver(&spend, &assignment, positions, record.index)?;
            public = PublicPart {
                recorded: Some(record.index),
                kind: Some(record.kind),
                transparent: record.transparent,
                shielded: record.shielded,
            };
        }
        let private = private_part(&spend, &public, decision.accepted);
        let receipt = Receipt {
            activity: spend.activity,
            decision: Some(decision),
            public,
            private,
            setup_micros,
        };
        Ok(Executed { receipt, assignment, touched })
    }

    fn public_part(
        &self,
        system: &str,
        spend: &Spend<F>,
        assignment: &Assignment<F>,
        proof: &[u8],
    ) -> ShieldedPublic {
        let hex_at =
            |index: usize| assignment.public.get(index).map_or_else(String::new, |v| to_hex(*v));
        ShieldedPublic {
            system: system.to_string(),
            circuit: self.circuit,
            root: hex_at(0),
            nullifier: hex_at(1),
            commitments: [hex_at(2), hex_at(3)],
            v_pub_in: spend.plan.v_pub_in,
            v_pub_out: spend.plan.v_pub_out,
            proof: hex::encode(proof),
            proof_bytes: proof.len() as u64,
            prove_micros: 0,
            verify_micros: 0,
            positions: None,
        }
    }

    /// Marks the input spent and gives each deliverable output to its recipient's wallet.
    fn deliver(
        &mut self,
        spend: &Spend<F>,
        assignment: &Assignment<F>,
        positions: [u64; 2],
        transaction: u64,
    ) -> Result<Vec<String>, PoolError> {
        let mut touched = Vec::new();
        if let Some((owner, note)) = &spend.spends {
            self.wallet_mut(owner)?.mark_spent(*note);
            touched.push(owner.clone());
        }
        for (j, (recipient, value)) in spend.pay.iter().enumerate() {
            let (Some(name), Some(value)) = (recipient, note_value(*value)) else { continue };
            let commitment = assignment.public.get(2 + j).copied().unwrap_or_else(F::zero);
            let rcm = spend.plan.outputs[j].rcm;
            let received = Received { value, rcm, commitment, position: positions[j], transaction };
            self.wallet_mut(name)?.receive(received)?;
            touched.push(name.clone());
        }
        touched.sort();
        touched.dedup();
        Ok(touched)
    }
}

fn owned(pay: [Pay<'_>; 2]) -> [(Option<String>, i64); 2] {
    pay.map(|(recipient, value)| (recipient.map(str::to_string), value))
}

/// The private half of a receipt: the input, and each output with where it went.
fn private_part<F>(spend: &Spend<F>, public: &PublicPart, accepted: bool) -> PrivatePart {
    let positions = public.shielded.as_ref().and_then(|shielded| shielded.positions);
    let outputs = spend
        .pay
        .iter()
        .enumerate()
        .map(|(j, (recipient, value))| {
            let kept = recipient.is_some() && note_value(*value).is_some();
            let delivery =
                if kept { NoteDelivery::HandedToLocalWallet } else { NoteDelivery::Discarded };
            OutputNote {
                recipient: recipient.clone(),
                value: *value,
                position: positions.map(|p| p[j]),
                delivery: accepted.then_some(delivery),
            }
        })
        .collect();
    PrivatePart { address: None, input: Some(spend.input.clone()), outputs }
}
