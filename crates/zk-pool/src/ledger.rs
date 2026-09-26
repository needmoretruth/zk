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
    /// or whose totals do not follow from its own transactions.
    ///
    /// The transactions are the history; the tree, the roots, the nullifier set, the turnstile and
    /// the transparent balances are what that history adds up to, and each is replayed here and
    /// compared. An edited total (a nullifier removed so a note can be spent again, a balance
    /// raised) is refused instead of believed.
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
        if view.circuit != SpendCircuit::Honest {
            return Err(PoolError::Corrupt("a stored ledger must use the honest circuit".into()));
        }
        let tree = replay::<F>(&view)?;
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
    pub(crate) fn faucet(
        &mut self,
        account: &str,
        amount: u16,
    ) -> Result<TransactionRecord, PoolError> {
        let balance = self.view.transparent.entry(account.to_string()).or_insert(0);
        *balance = credit(*balance, amount)?;
        let transparent =
            TransparentMove { account: account.to_string(), direction: Direction::Credit, amount };
        Ok(self.push(TransactionKind::Faucet, Some(transparent), None))
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
        let after = i64::try_from(before)
            .unwrap_or(i64::MAX)
            .saturating_add(i64::from(shielded.v_pub_in))
            .saturating_sub(i64::from(shielded.v_pub_out));
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
        self.view.pool_balance = credit(self.view.pool_balance, shielded.v_pub_in)?
            .saturating_sub(u64::from(shielded.v_pub_out));
        let (kind, transparent) = self.move_transparent(&shielded, account)?;
        Ok(self.push(kind, transparent, Some(shielded)))
    }

    /// Debits the shielding account or credits the unshielding one, and names the transaction kind.
    fn move_transparent(
        &mut self,
        shielded: &ShieldedPublic,
        account: Option<&str>,
    ) -> Result<(TransactionKind, Option<TransparentMove>), PoolError> {
        let (kind, direction, amount) = match (shielded.v_pub_in, shielded.v_pub_out) {
            (0, 0) => return Ok((TransactionKind::ShieldedTransfer, None)),
            (0, out) => (TransactionKind::Unshield, Direction::Credit, out),
            (v_in, _) => (TransactionKind::Shield, Direction::Debit, v_in),
        };
        let Some(account) = account else { return Ok((kind, None)) };
        let balance = self.view.transparent.entry(account.to_string()).or_insert(0);
        *balance = match direction {
            Direction::Credit => credit(*balance, amount)?,
            Direction::Debit => balance.saturating_sub(u64::from(amount)),
        };
        Ok((kind, Some(TransparentMove { account: account.to_string(), direction, amount })))
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

/// `balance + amount`, or an error when a balance read from disk is so large that adding overflows.
fn credit(balance: u64, amount: u16) -> Result<u64, PoolError> {
    balance.checked_add(u64::from(amount)).ok_or_else(|| {
        PoolError::Corrupt(format!("a balance of {balance} cannot grow by {amount}"))
    })
}

/// Replays `view.transactions` from an empty ledger and checks every total the view stores against
/// the replay, returning the commitment tree the replay built.
fn replay<F: ZkField>(view: &LedgerView) -> Result<MerkleTree<F>, PoolError> {
    let corrupt =
        |what: &str| PoolError::Corrupt(format!("{what} does not follow from its transactions"));
    let mut tree = new_note_tree::<F>();
    if view.tree_capacity != tree.capacity() as u64 {
        return Err(PoolError::Corrupt(format!("a tree of {} leaves", view.tree_capacity)));
    }
    let mut roots = vec![to_hex(tree.root())];
    let mut commitments = Vec::new();
    let mut nullifiers = Vec::new();
    let mut transparent = BTreeMap::<String, u64>::new();
    let mut pool_balance = 0u64;
    for (index, record) in view.transactions.iter().enumerate() {
        if record.index != index as u64 {
            return Err(PoolError::Corrupt(format!(
                "transaction {index} is numbered {}",
                record.index
            )));
        }
        if let Some(moved) = &record.transparent {
            let balance = transparent.entry(moved.account.clone()).or_insert(0);
            *balance = match moved.direction {
                Direction::Credit => credit(*balance, moved.amount)?,
                Direction::Debit => balance
                    .checked_sub(u64::from(moved.amount))
                    .ok_or_else(|| corrupt("a transparent balance"))?,
            };
        }
        let Some(shielded) = &record.shielded else { continue };
        from_hex::<F>(&shielded.nullifier)?;
        nullifiers.push(shielded.nullifier.clone());
        for commitment in &shielded.commitments {
            if tree.push(from_hex(commitment)?).is_none() {
                return Err(PoolError::Corrupt("more commitments than the tree holds".into()));
            }
            commitments.push(commitment.clone());
        }
        roots.push(to_hex(tree.root()));
        pool_balance = credit(pool_balance, shielded.v_pub_in)?
            .checked_sub(u64::from(shielded.v_pub_out))
            .ok_or_else(|| corrupt("the value in the pool"))?;
    }
    if commitments != view.commitments {
        return Err(corrupt("the list of note commitments"));
    }
    if roots != view.roots_seen {
        return Err(corrupt("the list of roots"));
    }
    if nullifiers != view.nullifiers {
        return Err(corrupt("the nullifier set"));
    }
    if pool_balance != view.pool_balance {
        return Err(corrupt("the value in the pool"));
    }
    // Accounts at zero may or may not be listed, depending on how they got there.
    let listed = |map: &BTreeMap<String, u64>| {
        map.iter()
            .filter(|(_, balance)| **balance > 0)
            .map(|(a, b)| (a.clone(), *b))
            .collect::<Vec<_>>()
    };
    if listed(&transparent) != listed(&view.transparent) {
        return Err(corrupt("a transparent balance"));
    }
    Ok(tree)
}
