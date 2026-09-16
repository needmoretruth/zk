//! Four attacks on the pool, each played in scratch worlds so the reader's own ledger is untouched.
//!
//! Every scratch world starts empty with two wallets, `mallory` (the attacker) and `victim`, each
//! given 100 transparent coins. Starting empty, rather than from a copy of the reader's ledger, keeps
//! every outcome the same whatever the reader has done. In particular the turnstile stops the
//! counterfeit only while the pool holds less than the attacker takes out; in a copy holding other
//! people's shielded coins, the counterfeit would leave the pool at their expense instead.

use serde::{Deserialize, Serialize};
use zk_circuit::ZkField;
use zk_core::Control;

use crate::error::PoolError;
use crate::keys::Keys;
use crate::receipt::{Activity, Receipt, SpendCircuit};
use crate::view::WalletView;
use crate::world::{NoteSpend, Pay, Spend, World};

const ATTACKER: &str = "mallory";
const VICTIM: &str = "victim";
/// What each scratch wallet receives from the faucet and shields.
const STAKE: u16 = 100;
/// The extra value the counterfeit creates, balanced by an output of `−COUNTERFEIT`.
const COUNTERFEIT: u16 = 5;

/// The four attacks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Attack {
    /// Spend a note that is already spent, with a fresh valid proof: the nullifier set refuses it.
    DoubleSpend,
    /// Spend another wallet's note without its spending key: the verifier refuses the proof.
    Steal,
    /// Create value with a negative output: the honest circuit refuses it, a circuit without range
    /// checks accepts it, and the turnstile stops the counterfeit leaving the pool.
    Counterfeit,
    /// Spend one note twice under two nullifiers: the honest circuit refuses it, a circuit that does
    /// not bind the nullifier to the note accepts both.
    UnboundNullifier,
}

impl Attack {
    /// All four, in teaching order.
    pub const ALL: [Attack; 4] =
        [Self::DoubleSpend, Self::Steal, Self::Counterfeit, Self::UnboundNullifier];
}

/// What one step of an attack tried.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum AttackAction {
    /// The attacker shields its faucet coins, to own a note.
    AttackerShields,
    /// The victim shields its faucet coins, to own a note worth stealing.
    VictimShields,
    /// The attacker spends its note honestly.
    SpendNote,
    /// The attacker spends the same note again with a fresh, valid proof.
    RespendSpentNote,
    /// The attacker spends the victim's note, proving with its own key.
    SpendNoteWithoutKey,
    /// The attacker spends its note into these two outputs, one of them negative.
    SpendWithNegativeOutput {
        /// The two output values.
        outputs: [i64; 2],
    },
    /// The attacker takes all its shielded value out of the pool.
    UnshieldEverything {
        /// How much it asks for.
        amount: u16,
    },
    /// The attacker spends its already spent note again, publishing a random nullifier.
    RespendWithFreshNullifier,
}

impl AttackAction {
    /// Whether the step is a forgery a sound ledger must refuse, as opposed to setting the scene.
    pub fn is_forgery(self) -> bool {
        !matches!(self, Self::AttackerShields | Self::VictimShields | Self::SpendNote)
    }
}

/// One transaction of an attack and what the ledger made of it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttackStep {
    /// What was tried.
    pub action: AttackAction,
    /// The circuit the scratch ledger verified against.
    pub circuit: SpendCircuit,
    /// The transaction and the ledger's decision, with every reason it refused.
    pub receipt: Receipt,
    /// Labels of the circuit's assertions the proved values break, in circuit order: why a proof of
    /// them cannot verify. Empty when the values satisfy the circuit.
    pub violated: Vec<String>,
    /// Value in the scratch pool after the step.
    pub pool_balance: u64,
    /// The attacker's shielded balance after the step.
    pub attacker_balance: u64,
    /// Nullifiers published in the scratch ledger after the step.
    pub nullifiers: u64,
}

/// Everything one attack did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttackReport {
    /// Which attack.
    pub attack: Attack,
    /// Its steps in order, across the honest and (for two attacks) broken scratch worlds.
    pub steps: Vec<AttackStep>,
    /// Whether a ledger running the honest circuit refused every forgery.
    pub honest_ledger_held: bool,
}

/// Plays `attack` with the pool's keys.
pub(crate) fn run<F: ZkField>(
    attack: Attack,
    keys: &mut Keys,
    control: &Control,
) -> Result<AttackReport, PoolError> {
    let mut scene = Scene { keys, control, steps: Vec::new() };
    match attack {
        Attack::DoubleSpend => scene.double_spend::<F>()?,
        Attack::Steal => scene.steal::<F>()?,
        Attack::Counterfeit => scene.counterfeit::<F>()?,
        Attack::UnboundNullifier => scene.unbound_nullifier::<F>()?,
    }
    let honest_ledger_held = scene
        .steps
        .iter()
        .filter(|step| step.circuit == SpendCircuit::Honest && step.action.is_forgery())
        .all(|step| !step.receipt.accepted());
    Ok(AttackReport { attack, steps: scene.steps, honest_ledger_held })
}

struct Scene<'k> {
    keys: &'k mut Keys,
    control: &'k Control,
    steps: Vec<AttackStep>,
}

impl Scene<'_> {
    fn double_spend<F: ZkField>(&mut self) -> Result<(), PoolError> {
        let mut world = self.scratch::<F>(SpendCircuit::Honest)?;
        self.shield(&mut world, ATTACKER, AttackAction::AttackerShields)?;
        let note = own_note(&world, ATTACKER, STAKE)?;
        let pay = |to: &'static str| [(Some(to), 60), (Some(ATTACKER), 40)];
        let spend = world.spend_note(honest_spend(ATTACKER, note, pay(VICTIM)))?;
        self.step(&mut world, AttackAction::SpendNote, spend)?;
        let again = world.spend_note(honest_spend(ATTACKER, note, pay(ATTACKER)))?;
        self.step(&mut world, AttackAction::RespendSpentNote, again)
    }

    fn steal<F: ZkField>(&mut self) -> Result<(), PoolError> {
        let mut world = self.scratch::<F>(SpendCircuit::Honest)?;
        self.shield(&mut world, VICTIM, AttackAction::VictimShields)?;
        let note = own_note(&world, VICTIM, STAKE)?;
        let theft = NoteSpend {
            key_holder: ATTACKER,
            ..honest_spend(VICTIM, note, [(Some(ATTACKER), i64::from(STAKE)), (None, 0)])
        };
        let spend = world.spend_note(theft)?;
        self.step(&mut world, AttackAction::SpendNoteWithoutKey, spend)
    }

    fn counterfeit<F: ZkField>(&mut self) -> Result<(), PoolError> {
        let inflated = STAKE + COUNTERFEIT;
        let outputs = [i64::from(inflated), -i64::from(COUNTERFEIT)];
        for circuit in [SpendCircuit::Honest, SpendCircuit::WithoutRangeChecks] {
            let mut world = self.scratch::<F>(circuit)?;
            self.shield(&mut world, ATTACKER, AttackAction::AttackerShields)?;
            let note = own_note(&world, ATTACKER, STAKE)?;
            let pay = outputs.map(|value| (Some(ATTACKER), value));
            let spend = world.spend_note(honest_spend(ATTACKER, note, pay))?;
            self.step(&mut world, AttackAction::SpendWithNegativeOutput { outputs }, spend)?;
            if circuit == SpendCircuit::WithoutRangeChecks {
                let note = own_note(&world, ATTACKER, inflated)?;
                let request = NoteSpend {
                    activity: Activity::Unshield,
                    v_pub_out: inflated,
                    ..honest_spend(ATTACKER, note, [(None, 0), (None, 0)])
                };
                let spend = world.spend_note(request)?;
                let action = AttackAction::UnshieldEverything { amount: inflated };
                self.step(&mut world, action, spend)?;
            }
        }
        Ok(())
    }

    fn unbound_nullifier<F: ZkField>(&mut self) -> Result<(), PoolError> {
        for circuit in [SpendCircuit::Honest, SpendCircuit::WithoutNullifierBinding] {
            let mut world = self.scratch::<F>(circuit)?;
            self.shield(&mut world, ATTACKER, AttackAction::AttackerShields)?;
            let note = own_note(&world, ATTACKER, STAKE)?;
            let pay = [(Some(ATTACKER), i64::from(STAKE)), (None, 0)];
            let spend = world.spend_note(honest_spend(ATTACKER, note, pay))?;
            self.step(&mut world, AttackAction::SpendNote, spend)?;
            let forged = NoteSpend { forge_nullifier: true, ..honest_spend(ATTACKER, note, pay) };
            let spend = world.spend_note(forged)?;
            self.step(&mut world, AttackAction::RespendWithFreshNullifier, spend)?;
        }
        Ok(())
    }

    /// An empty world on `circuit` with both wallets funded in the open.
    fn scratch<F: ZkField>(&self, circuit: SpendCircuit) -> Result<World<F>, PoolError> {
        let mut world = World::new(self.keys.system_id(), circuit);
        for name in [ATTACKER, VICTIM] {
            world.wallets.insert(name.to_string(), WalletView::create::<F>(name)?);
            world.ledger.faucet(name, STAKE);
        }
        Ok(world)
    }

    fn shield<F: ZkField>(
        &mut self,
        world: &mut World<F>,
        name: &str,
        action: AttackAction,
    ) -> Result<(), PoolError> {
        let spend = world.shield(name, STAKE)?;
        self.step(world, action, spend)
    }

    /// Executes `spend` in `world`, then evaluates the circuit on the proved values to name the
    /// assertions they break. The evaluation explains the verdict; it never stands in for it.
    fn step<F: ZkField>(
        &mut self,
        world: &mut World<F>,
        action: AttackAction,
        spend: Spend<F>,
    ) -> Result<(), PoolError> {
        let executed = world.execute(self.keys, self.control, spend)?;
        let circuit = world.circuit.example().circuit::<F>();
        let circuit = circuit.map_err(|e| PoolError::Circuit(e.to_string()))?;
        let evaluation = circuit
            .evaluate_unchecked(&executed.assignment)
            .map_err(|e| PoolError::Circuit(e.to_string()))?;
        let ledger = world.ledger.view();
        self.steps.push(AttackStep {
            action,
            circuit: world.circuit,
            receipt: executed.receipt,
            violated: evaluation.violated,
            pool_balance: ledger.pool_balance,
            attacker_balance: world.wallets.get(ATTACKER).map_or(0, |wallet| wallet.balance),
            nullifiers: ledger.nullifiers.len() as u64,
        });
        Ok(())
    }
}

/// A spend of `owner`'s note `note` with `owner`'s own key.
fn honest_spend<'a>(owner: &'a str, note: usize, pay: [Pay<'a>; 2]) -> NoteSpend<'a> {
    NoteSpend {
        activity: Activity::Send,
        owner,
        note,
        key_holder: owner,
        pay,
        v_pub_out: 0,
        forge_nullifier: false,
    }
}

/// The index of `name`'s most recent note worth exactly `value`, spent or not.
fn own_note<F>(world: &World<F>, name: &str, value: u16) -> Result<usize, PoolError> {
    let notes = world.wallets.get(name).map(|wallet| wallet.notes.as_slice()).unwrap_or_default();
    notes.iter().rposition(|note| note.value == value).ok_or_else(|| PoolError::NoNoteLargeEnough {
        name: name.to_string(),
        amount: value,
        largest: notes.iter().map(|note| note.value).max().unwrap_or(0),
    })
}
