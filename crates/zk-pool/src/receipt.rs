//! What activities return: plain data a screen draws, split into what became public and what stayed
//! private. Values are numbers, hex strings and enums; the sentences belong to the screen.

use serde::{Deserialize, Serialize};
use zk_core::{ExampleId, Verdict};

/// The spend circuit a ledger verifies against and a wallet proves with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpendCircuit {
    /// `pool-spend`, every check in place: the only circuit a pool's own ledger ever uses.
    #[serde(rename = "pool-spend")]
    Honest,
    /// `pool-spend-no-range-checks`, for the counterfeit attack's scratch ledger.
    #[serde(rename = "pool-spend-no-range-checks")]
    WithoutRangeChecks,
    /// `pool-spend-unbound-nullifier`, for the unbound-nullifier attack's scratch ledger.
    #[serde(rename = "pool-spend-unbound-nullifier")]
    WithoutNullifierBinding,
}

impl SpendCircuit {
    /// The statement behind the circuit.
    pub fn example(self) -> ExampleId {
        match self {
            Self::Honest => ExampleId::PoolSpend,
            Self::WithoutRangeChecks => ExampleId::PoolSpendWithoutRangeChecks,
            Self::WithoutNullifierBinding => ExampleId::PoolSpendWithoutNullifierBinding,
        }
    }
}

/// Which activity produced a receipt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Activity {
    /// A spending key and its address were created.
    NewWallet,
    /// Transparent coins were credited in the open.
    Faucet,
    /// Transparent coins went into the shielded pool as a note.
    Shield,
    /// A note was spent to pay someone, with change back.
    Send,
    /// A note was spent to take coins out of the pool.
    Unshield,
}

/// What kind a recorded transaction is, as far as its public data tells.
///
/// A private transfer to someone else and one to oneself look the same from outside; only the public
/// amounts say whether value entered or left the pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransactionKind {
    /// A transparent credit, with no proof.
    Faucet,
    /// A shielded transaction with public value coming in.
    Shield,
    /// A shielded transaction with no public value moving.
    ShieldedTransfer,
    /// A shielded transaction with public value going out.
    Unshield,
}

/// Which way a transparent balance moved.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Direction {
    /// The account received coins.
    Credit,
    /// The account paid coins into the pool.
    Debit,
}

/// A transparent balance change, which everyone sees with its account name.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransparentMove {
    /// The account, named after its wallet.
    pub account: String,
    /// Credit or debit.
    pub direction: Direction,
    /// How much.
    pub amount: u16,
}

/// Why a ledger refused a transaction. A ledger reports every reason that applies, not the first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Rejection {
    /// The verifier did not accept the proof against the public inputs.
    ProofRejected,
    /// The anchor is not a root this ledger's tree has ever had.
    UnknownRoot,
    /// The nullifier is already in the nullifier set: that note was spent.
    NullifierAlreadySpent,
    /// The transparent account cannot pay the value shielded.
    NotEnoughTransparentFunds,
    /// More value would leave the shielded pool than ever entered it (ZIP 209's turnstile).
    TurnstileWouldGoNegative,
    /// The commitment tree has no room for two more notes.
    TreeFull,
}

/// A ledger's decision on one shielded transaction, with every check it made.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    /// Whether the transaction was recorded; true exactly when `rejections` is empty.
    pub accepted: bool,
    /// The verifier's own verdict, shown even when another check failed.
    pub proof: Verdict,
    /// Every reason for refusal.
    pub rejections: Vec<Rejection>,
    /// Value in the shielded pool before the transaction.
    pub pool_balance_before: u64,
    /// Value in the shielded pool after it, or what it would have been; negative trips the turnstile.
    pub pool_balance_after: i64,
}

/// The public half of a shielded transaction: exactly what its verifier, and anyone reading the
/// ledger, receives.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShieldedPublic {
    /// Proof system ID, such as `groth16`.
    pub system: String,
    /// The circuit the proof was made for.
    pub circuit: SpendCircuit,
    /// The anchor: a root of the commitment tree.
    pub root: String,
    /// The spent note's nullifier.
    pub nullifier: String,
    /// The two output notes' commitments.
    pub commitments: [String; 2],
    /// Public value entering the pool.
    pub v_pub_in: u16,
    /// Public value leaving the pool.
    pub v_pub_out: u16,
    /// The proof, as the proof system serializes it, in hex.
    pub proof: String,
    /// Proof size in bytes.
    pub proof_bytes: u64,
    /// Proving time on this machine; the ledger cannot check it, the wallet reported it.
    pub prove_micros: u64,
    /// Verification time on this machine.
    pub verify_micros: u64,
    /// Where the two commitments landed in the tree, once the ledger accepted them.
    pub positions: Option<[u64; 2]>,
}

/// One transaction as the ledger records it, in the order accepted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionRecord {
    /// Position in the ledger's list, from 0.
    pub index: u64,
    /// What the public data says it is.
    pub kind: TransactionKind,
    /// The transparent side, if any.
    pub transparent: Option<TransparentMove>,
    /// The shielded side, if any.
    pub shielded: Option<ShieldedPublic>,
}

/// What an activity put in front of the world.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicPart {
    /// Index of the recorded transaction; `None` when nothing was recorded.
    pub recorded: Option<u64>,
    /// What the public data says it is.
    pub kind: Option<TransactionKind>,
    /// The transparent side.
    pub transparent: Option<TransparentMove>,
    /// The shielded side, shown even when the ledger refused it, since it was submitted in the open.
    pub shielded: Option<ShieldedPublic>,
}

/// The input a spend consumed, as only the wallets know it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum InputNote {
    /// A zero-value input under a throwaway key, which lets a shield use the spend circuit without
    /// owning a note (as Sapling's dummy spends do).
    Dummy,
    /// A note from `owner`'s wallet, spent with the key of `key_holder`.
    Note {
        /// Whose wallet holds the note.
        owner: String,
        /// Whose spending key the proof used; differs from `owner` only in the steal attack.
        key_holder: String,
        /// The note's value.
        value: u16,
        /// Its position in the commitment tree.
        position: u64,
    },
}

/// How an output note reached its owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NoteDelivery {
    /// Handed straight to the recipient's wallet file on this machine. A real shielded system
    /// encrypts the note to its recipient and publishes the ciphertext on-chain; this toy has neither
    /// encryption nor a chain.
    HandedToLocalWallet,
    /// Kept by nobody: a zero-value padding output, or a counterfeit negative note no wallet can hold.
    Discarded,
}

/// An output note as only the wallets know it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputNote {
    /// The wallet it was paid to; `None` for padding paid to a random address.
    pub recipient: Option<String>,
    /// Its value, signed so a counterfeit output of `−5` shows as `−5`.
    pub value: i64,
    /// Its position in the commitment tree, once accepted.
    pub position: Option<u64>,
    /// How it reached its owner; `None` when the ledger refused the transaction.
    pub delivery: Option<NoteDelivery>,
}

/// What an activity kept out of the ledger.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivatePart {
    /// A new wallet's address; its spending key stays in the wallet file.
    pub address: Option<String>,
    /// The spent input.
    pub input: Option<InputNote>,
    /// The two outputs.
    pub outputs: Vec<OutputNote>,
}

/// Everything one activity did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    /// Which activity.
    pub activity: Activity,
    /// The ledger's decision, for shielded transactions.
    pub decision: Option<Decision>,
    /// What became public.
    pub public: PublicPart,
    /// What stayed private.
    pub private: PrivatePart,
    /// Time spent building the proof system's keys first, when this activity was the first to need
    /// them.
    pub setup_micros: Option<u64>,
}

impl Receipt {
    /// Whether the activity took effect: always for wallets and faucets, on acceptance for spends.
    pub fn accepted(&self) -> bool {
        self.decision.as_ref().is_none_or(|decision| decision.accepted)
    }
}
