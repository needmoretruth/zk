//! The public ledger: transparent balances, the commitment tree, the nullifier set, the turnstile
//! and the transactions, with the rules that decide whether a shielded transaction is recorded.

use std::collections::BTreeMap;

use zk_circuit::ZkField;
use zk_core::Verdict;
use zk_examples::merkle_tree::MerkleTree;
use zk_examples::pool::new_note_tree;

use crate::element::{from_hex, to_hex};
use crate::error::PoolError;
use crate::receipt::{
    Decision, Direction, Rejection, ShieldedPublic, SpendCircuit, TransactionKind,
    TransactionRecord, TransparentMove,
};
use crate::view::{FORMAT, LedgerView};

/// The ledger's public state, and the tree rebuilt from it so roots and paths can be computed.
#[derive(Clone, Debug)]
pub(crate) struct Ledger<F> {
    view: LedgerView,
    tree: MerkleTree<F>,
}

impl<F: ZkField> Ledger<F> {
    /// An empty ledger whose transactions `system` verifies against `circuit`.
    pub(crate) fn new(system: &str, circuit: SpendCircuit) -> Self {
        let tree = new_note_tree::<F>();
        let root = to_hex(tree.root());
        let view = LedgerView {
            format: FORMAT,
            system: system.to_string(),
            field: F::NAME.to_string(),
            circuit,
            transparent: BTreeMap::new(),
            pool_balance: 0,
            tree_capacity: tree.capacity() as u64,
            commitments: Vec::new(),
            root: root.clone(),
            roots_seen: vec![root],
            nullifiers: Vec::new(),
            transactions: Vec::new(),
        };
        Self { view, tree }
    }

    /// Rebuilds a ledger read from disk, refusing one made by another system, over another field,
    /// or whose stored root does not match its commitments.
    pub(crate) fn from_view(view: LedgerView, system: &str) -> Result<Self, PoolError> {
        let mismatch = |expected: &str, found: &str| PoolError::Mismatch {
            expected: expected.to_string(),
            found: found.to_string(),
        };
        if view.format != FORMAT {
            return Err(PoolError::Corrupt(format!("ledger format {}", view.format)));
        }
        if view.system != system {
            return Err(mismatch(system, &view.system));
        }
        if view.field != F::NAME {
            return Err(mismatch(F::NAME, &view.field));
        }
        let mut tree = new_note_tree::<F>();
        for commitment in &view.commitments {
            if tree.push(from_hex(commitment)?).is_none() {
                return Err(PoolError::Corrupt("more commitments than the tree holds".into()));
            }
        }
        if view.circuit != SpendCircuit::Honest {
            return Err(PoolError::Corrupt("a stored ledger must use the honest circuit".into()));
        }
        if to_hex(tree.root()) != view.root {
            return Err(PoolError::Corrupt("the ledger's root does not match its notes".into()));
        }
        Ok(Self { view, tree })
    }

    /// The public state.
    pub(crate) fn view(&self) -> &LedgerView {
        &self.view
    }

    /// The commitment tree, for building spend paths.
    pub(crate) fn tree(&self) -> &MerkleTree<F> {
        &self.tree
    }

    /// An account's transparent balance, zero if it never had one.
    pub(crate) fn transparent_balance(&self, account: &str) -> u64 {
        self.view.transparent.get(account).copied().unwrap_or(0)
    }

    /// Whether the ledger holds `commitment` at `position`.
    pub(crate) fn holds(&self, position: u64, commitment: &str) -> bool {
        let stored = usize::try_from(position).ok().and_then(|i| self.view.commitments.get(i));
        stored.is_some_and(|stored| stored == commitment)
    }

    /// Whether `nullifier` has been published.
    pub(crate) fn is_spent(&self, nullifier: &str) -> bool {
        self.view.nullifiers.iter().any(|seen| seen == nullifier)
    }

    /// Credits `account` in the open and records it; a faucet needs no proof.
    pub(crate) fn faucet(&mut self, account: &str, amount: u16) -> TransactionRecord {
        *self.view.transparent.entry(account.to_string()).or_insert(0) += u64::from(amount);
        let transparent =
            TransparentMove { account: account.to_string(), direction: Direction::Credit, amount };
        self.push(TransactionKind::Faucet, Some(transparent), None)
    }

    /// Runs every rule on a submitted transaction whose proof the verifier judged `proof`.
    ///
    /// `account` is the transparent account paying `v_pub_in` or receiving `v_pub_out`.
    pub(crate) fn decide(
        &self,
        proof: Verdict,
        shielded: &ShieldedPublic,
        account: Option<&str>,
    ) -> Decision {
        let mut rejections = Vec::new();
        if !proof.accepted() {
            rejections.push(Rejection::ProofRejected);
        }
        if !self.view.roots_seen.contains(&shielded.root) {
            rejections.push(Rejection::UnknownRoot);
        }
        if self.is_spent(&shielded.nullifier) {
            rejections.push(Rejection::NullifierAlreadySpent);
        }
        let payer_balance = account.map_or(0, |account| self.transparent_balance(account));
        if payer_balance < u64::from(shielded.v_pub_in) {
            rejections.push(Rejection::NotEnoughTransparentFunds);
        }
        let before = self.view.pool_balance;
        let after = i64::try_from(before).unwrap_or(i64::MAX) + i64::from(shielded.v_pub_in)
            - i64::from(shielded.v_pub_out);
        if after < 0 {
            rejections.push(Rejection::TurnstileWouldGoNegative);
        }
        if self.tree.len() + 2 > self.tree.capacity() {
            rejections.push(Rejection::TreeFull);
        }
        Decision {
            accepted: rejections.is_empty(),
            proof,
            rejections,
            pool_balance_before: before,
            pool_balance_after: after,
        }
    }

    /// Records a transaction [`Ledger::decide`] accepted: appends both commitments, the new root and
    /// the nullifier, moves the transparent and shielded balances, and returns the record.
    pub(crate) fn record(
        &mut self,
        mut shielded: ShieldedPublic,
        account: Option<&str>,
    ) -> Result<TransactionRecord, PoolError> {
        let mut positions = [0u64; 2];
        for (slot, commitment) in positions.iter_mut().zip(&shielded.commitments) {
            let position = self.tree.push(from_hex(commitment)?).ok_or_else(|| {
                PoolError::Corrupt("recorded a transaction into a full tree".into())
            })?;
            *slot = position as u64;
            self.view.commitments.push(commitment.clone());
        }
        shielded.positions = Some(positions);
        self.view.root = to_hex(self.tree.root());
        self.view.roots_seen.push(self.view.root.clone());
        self.view.nullifiers.push(shielded.nullifier.clone());
        let (v_in, v_out) = (u64::from(shielded.v_pub_in), u64::from(shielded.v_pub_out));
        self.view.pool_balance = (self.view.pool_balance + v_in).saturating_sub(v_out);
        let (kind, transparent) = self.move_transparent(&shielded, account);
        Ok(self.push(kind, transparent, Some(shielded)))
    }

    /// Debits the shielding account or credits the unshielding one, and names the transaction kind.
    fn move_transparent(
        &mut self,
        shielded: &ShieldedPublic,
        account: Option<&str>,
    ) -> (TransactionKind, Option<TransparentMove>) {
        let (kind, direction, amount) = match (shielded.v_pub_in, shielded.v_pub_out) {
            (0, 0) => return (TransactionKind::ShieldedTransfer, None),
            (0, out) => (TransactionKind::Unshield, Direction::Credit, out),
            (v_in, _) => (TransactionKind::Shield, Direction::Debit, v_in),
        };
        let Some(account) = account else { return (kind, None) };
        let balance = self.view.transparent.entry(account.to_string()).or_insert(0);
        *balance = match direction {
            Direction::Credit => *balance + u64::from(amount),
            Direction::Debit => balance.saturating_sub(u64::from(amount)),
        };
        (kind, Some(TransparentMove { account: account.to_string(), direction, amount }))
    }

    fn push(
        &mut self,
        kind: TransactionKind,
        transparent: Option<TransparentMove>,
        shielded: Option<ShieldedPublic>,
    ) -> TransactionRecord {
        let index = self.view.transactions.len() as u64;
        let record = TransactionRecord { index, kind, transparent, shielded };
        self.view.transactions.push(record.clone());
        record
    }
}
