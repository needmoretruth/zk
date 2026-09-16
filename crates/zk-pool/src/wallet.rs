//! A wallet's private state: its key, and the notes only it can open and spend.

use zk_circuit::ZkField;
use zk_examples::pool::{address, nullifier};

use crate::element::{from_hex, random, to_hex};
use crate::error::PoolError;
use crate::ledger::Ledger;
use crate::receipt::NoteDelivery;
use crate::view::{FORMAT, NoteRecord, WalletView};

/// Where a new note came from and where it sits, as the wallet receiving it learns.
pub(crate) struct Received<F> {
    /// Its value.
    pub(crate) value: u16,
    /// Its commitment randomness.
    pub(crate) rcm: F,
    /// Its commitment.
    pub(crate) commitment: F,
    /// Its tree position.
    pub(crate) position: u64,
    /// The transaction that created it.
    pub(crate) transaction: u64,
}

impl WalletView {
    /// A wallet with a fresh spending key from the operating system.
    pub(crate) fn create<F: ZkField>(name: &str) -> Result<Self, PoolError> {
        let sk = random::<F>()?;
        Ok(Self {
            format: FORMAT,
            name: name.to_string(),
            spending_key: to_hex(sk),
            address: to_hex(address(sk)),
            balance: 0,
            notes: Vec::new(),
        })
    }

    /// Takes a wallet read from disk and brings it in line with the ledger, which is the source of
    /// truth: a note the ledger does not hold is dropped (its transaction never got recorded), and a
    /// note is spent exactly when its nullifier is published.
    pub(crate) fn reconcile<F: ZkField>(
        mut self,
        file_name: &str,
        ledger: &Ledger<F>,
    ) -> Result<Self, PoolError> {
        if self.format != FORMAT || self.name != file_name {
            return Err(PoolError::Corrupt(format!("wallet file {file_name} does not match")));
        }
        let sk = from_hex::<F>(&self.spending_key)?;
        if to_hex(address(sk)) != self.address {
            return Err(PoolError::Corrupt(format!(
                "wallet {file_name}: address is not its key's"
            )));
        }
        self.notes.retain(|note| ledger.holds(note.position, &note.commitment));
        for note in &mut self.notes {
            note.spent = ledger.is_spent(&note.nullifier);
        }
        self.refresh_balance();
        Ok(self)
    }

    /// The spending key as a field element.
    pub(crate) fn key<F: ZkField>(&self) -> Result<F, PoolError> {
        from_hex(&self.spending_key)
    }

    /// The address as a field element.
    pub(crate) fn address_element<F: ZkField>(&self) -> Result<F, PoolError> {
        from_hex(&self.address)
    }

    /// Keeps a note paid to this wallet, computing the nullifier only this wallet's key yields.
    pub(crate) fn receive<F: ZkField>(&mut self, note: Received<F>) -> Result<(), PoolError> {
        let sk = self.key::<F>()?;
        self.notes.push(NoteRecord {
            value: note.value,
            rcm: to_hex(note.rcm),
            commitment: to_hex(note.commitment),
            nullifier: to_hex(nullifier(sk, note.commitment)),
            position: note.position,
            spent: false,
            received_in: note.transaction,
            delivery: NoteDelivery::HandedToLocalWallet,
        });
        self.refresh_balance();
        Ok(())
    }

    /// Marks note `index` spent.
    pub(crate) fn mark_spent(&mut self, index: usize) {
        if let Some(note) = self.notes.get_mut(index) {
            note.spent = true;
        }
        self.refresh_balance();
    }

    /// The smallest unspent note worth at least `amount`, as `(index, value)`, so large notes stay
    /// whole; otherwise the largest unspent value, for the error.
    pub(crate) fn pick_note(&self, amount: u16) -> Result<(usize, u16), u16> {
        let unspent = || self.notes.iter().enumerate().filter(|(_, note)| !note.spent);
        unspent()
            .filter(|(_, note)| note.value >= amount)
            .min_by_key(|(_, note)| note.value)
            .map(|(index, note)| (index, note.value))
            .ok_or_else(|| unspent().map(|(_, note)| note.value).max().unwrap_or(0))
    }

    fn refresh_balance(&mut self) {
        self.balance =
            self.notes.iter().filter(|note| !note.spent).map(|note| u64::from(note.value)).sum();
    }
}
