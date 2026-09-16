//! The pool a reader plays with: its activities, and the files they are saved in.

use std::path::{Path, PathBuf};

use zk_circuit::ZkField;
use zk_core::{Control, ProofSystem};

use crate::attack::{self, Attack, AttackReport};
use crate::error::PoolError;
use crate::keys::Keys;
use crate::ledger::Ledger;
use crate::receipt::{Activity, PrivatePart, PublicPart, Receipt, SpendCircuit};
use crate::storage::{self, LEDGER_FILE};
use crate::view::{LedgerView, WalletView};
use crate::world::{Executed, NoteSpend, Pay, World};

static GROTH16: sys_groth16::Groth16 = sys_groth16::Groth16;
static HALO2: sys_halo2::Halo2 = sys_halo2::Halo2;

/// A Toy Shielded Pool over field `F`, proved by a system whose field is `F`.
///
/// The caller pairs the two; [`Pool::groth16`] and [`Pool::halo2`] are the pairs the museum offers.
/// Every activity that changes something writes the wallets it touched and then the ledger, each
/// file replaced atomically. The ledger is the source of truth: on opening, wallets are brought in
/// line with it, so a crash between the two writes loses nothing the ledger recorded.
pub struct Pool<F: ZkField> {
    dir: PathBuf,
    keys: Keys,
    control: Control,
    world: World<F>,
}

/// What an activity changed, so the pool knows which files to write.
struct Change<T> {
    value: T,
    wallets: Vec<String>,
    ledger: bool,
}

impl Pool<sys_groth16::Field> {
    /// Opens (or starts) a pool in `dir` proved with Groth16 on BLS12-381, as Zcash Sapling proves
    /// its spends.
    pub fn groth16(dir: impl AsRef<Path>) -> Result<Self, PoolError> {
        Self::open(&GROTH16, dir)
    }
}

impl Pool<sys_halo2::Field> {
    /// Opens (or starts) a pool in `dir` proved with Halo 2 on the Pasta curves, as Zcash Orchard
    /// proves its actions.
    pub fn halo2(dir: impl AsRef<Path>) -> Result<Self, PoolError> {
        Self::open(&HALO2, dir)
    }
}

impl<F: ZkField> Pool<F> {
    /// Opens the pool saved in `dir`, or an empty one if `dir` holds none. `system`'s field must be
    /// `F`. No keys are built yet: the first proof builds them.
    pub fn open(
        system: &'static dyn ProofSystem,
        dir: impl AsRef<Path>,
    ) -> Result<Self, PoolError> {
        let dir = dir.as_ref().to_path_buf();
        storage::create_dirs(&dir)?;
        let keys = Keys::new(system);
        let mut world = World::new(keys.system_id(), SpendCircuit::Honest);
        if let Some(view) = storage::read_json::<LedgerView>(&dir.join(LEDGER_FILE))? {
            world.ledger = Ledger::from_view(view, keys.system_id())?;
        }
        for name in storage::wallet_names(&dir)? {
            let path = storage::wallet_path(&dir, &name);
            let stored = storage::read_json::<WalletView>(&path)?
                .ok_or_else(|| PoolError::Storage(format!("{} vanished", path.display())))?;
            let wallet = stored.reconcile(&name, &world.ledger)?;
            world.wallets.insert(name, wallet);
        }
        Ok(Self { dir, keys, control: Control::new(), world })
    }

    /// Lets a screen watch progress and cancel key building or proving from another thread.
    pub fn set_control(&mut self, control: Control) {
        self.control = control;
    }

    /// The proof system's ID, such as `groth16`.
    pub fn system_id(&self) -> &'static str {
        self.keys.system_id()
    }

    /// Creates a wallet: a spending key from the operating system and its address. Nothing reaches
    /// the ledger.
    pub fn new_wallet(&mut self, name: &str) -> Result<Receipt, PoolError> {
        storage::check_name(name)?;
        if self.world.wallets.contains_key(name) {
            return Err(PoolError::WalletExists(name.to_string()));
        }
        let wallet = WalletView::create::<F>(name)?;
        storage::write_json(&storage::wallet_path(&self.dir, name), &wallet)?;
        let private =
            PrivatePart { address: Some(wallet.address.clone()), ..PrivatePart::default() };
        self.world.wallets.insert(name.to_string(), wallet);
        Ok(Receipt {
            activity: Activity::NewWallet,
            decision: None,
            public: PublicPart::default(),
            private,
            setup_micros: None,
        })
    }

    /// Credits `name`'s transparent account with `amount`, entirely in public.
    pub fn faucet(&mut self, name: &str, amount: u16) -> Result<Receipt, PoolError> {
        self.world.wallet(name)?;
        nonzero(amount)?;
        self.transact(|world, _, _| {
            let record = world.ledger.faucet(name, amount);
            let public = PublicPart {
                recorded: Some(record.index),
                kind: Some(record.kind),
                transparent: record.transparent,
                shielded: None,
            };
            let receipt = Receipt {
                activity: Activity::Faucet,
                decision: None,
                public,
                private: PrivatePart::default(),
                setup_micros: None,
            };
            Ok(Change { value: receipt, wallets: Vec::new(), ledger: true })
        })
    }

    /// Moves `amount` from `name`'s transparent account into a new note of `name`'s.
    pub fn shield(&mut self, name: &str, amount: u16) -> Result<Receipt, PoolError> {
        nonzero(amount)?;
        let balance = self.world.ledger.transparent_balance(name);
        self.world.wallet(name)?;
        if balance < u64::from(amount) {
            let name = name.to_string();
            return Err(PoolError::NotEnoughTransparentFunds { name, balance, amount });
        }
        self.transact(|world, keys, control| {
            let spend = world.shield(name, amount)?;
            world.execute(keys, control, spend).map(Change::from)
        })
    }

    /// Pays `amount` from one of `from`'s notes to `to`, with the change back to `from`.
    pub fn send(&mut self, from: &str, to: &str, amount: u16) -> Result<Receipt, PoolError> {
        self.world.wallet(to)?;
        self.spend(Activity::Send, from, amount, |change| {
            ([(Some(to), i64::from(amount)), change], 0)
        })
    }

    /// Takes `amount` out of one of `name`'s notes into `name`'s transparent account.
    pub fn unshield(&mut self, name: &str, amount: u16) -> Result<Receipt, PoolError> {
        self.spend(Activity::Unshield, name, amount, |change| ([change, (None, 0)], amount))
    }

    /// Spends `owner`'s smallest note that covers `amount`; `layout` places the change output and
    /// says how much leaves the pool.
    fn spend<'a>(
        &mut self,
        activity: Activity,
        owner: &'a str,
        amount: u16,
        layout: impl FnOnce(Pay<'a>) -> ([Pay<'a>; 2], u16),
    ) -> Result<Receipt, PoolError> {
        nonzero(amount)?;
        let (note, value) = self.world.wallet(owner)?.pick_note(amount).map_err(|largest| {
            PoolError::NoNoteLargeEnough { name: owner.to_string(), amount, largest }
        })?;
        let change = value - amount;
        let change: Pay<'a> = if change > 0 { (Some(owner), i64::from(change)) } else { (None, 0) };
        let (pay, v_pub_out) = layout(change);
        let request = NoteSpend {
            activity,
            owner,
            note,
            key_holder: owner,
            pay,
            v_pub_out,
            forge_nullifier: false,
        };
        self.transact(|world, keys, control| {
            let spend = world.spend_note(request)?;
            world.execute(keys, control, spend).map(Change::from)
        })
    }

    /// The world's view: everything the ledger holds.
    pub fn ledger(&self) -> LedgerView {
        self.world.ledger.view().clone()
    }

    /// The owner's view of wallet `name`.
    pub fn wallet(&self, name: &str) -> Result<WalletView, PoolError> {
        self.world.wallet(name).cloned()
    }

    /// Every wallet's name, sorted.
    pub fn wallet_names(&self) -> Vec<String> {
        self.world.wallets.keys().cloned().collect()
    }

    /// Deletes the ledger and every wallet file and starts an empty pool. Keys already built are kept,
    /// since they depend on the circuit, not on the ledger.
    pub fn reset(&mut self) -> Result<(), PoolError> {
        storage::remove_pool_files(&self.dir)?;
        self.world = World::new(self.keys.system_id(), SpendCircuit::Honest);
        Ok(())
    }

    /// Runs one attack in scratch worlds held in memory; this pool's ledger and wallets are neither
    /// read nor changed. The scratch worlds reuse this pool's keys.
    pub fn attack(&mut self, attack: Attack) -> Result<AttackReport, PoolError> {
        attack::run::<F>(attack, &mut self.keys, &self.control)
    }

    /// Applies `change` to the world and saves what it touched; if anything fails, the world in
    /// memory goes back to what is on disk.
    fn transact<T>(
        &mut self,
        change: impl FnOnce(&mut World<F>, &mut Keys, &Control) -> Result<Change<T>, PoolError>,
    ) -> Result<T, PoolError> {
        let before = self.world.clone();
        let saved = change(&mut self.world, &mut self.keys, &self.control)
            .and_then(|change| self.save(&change.wallets, change.ledger).map(|()| change.value));
        if saved.is_err() {
            self.world = before;
        }
        saved
    }

    /// Writes the touched wallets first and the ledger last, so the ledger decides what happened.
    fn save(&self, wallets: &[String], ledger: bool) -> Result<(), PoolError> {
        for name in wallets {
            storage::write_json(&storage::wallet_path(&self.dir, name), self.world.wallet(name)?)?;
        }
        if ledger {
            storage::write_json(&self.dir.join(LEDGER_FILE), self.world.ledger.view())?;
        }
        Ok(())
    }
}

impl<F> From<Executed<F>> for Change<Receipt> {
    fn from(executed: Executed<F>) -> Self {
        let ledger = executed.receipt.accepted();
        Change { value: executed.receipt, wallets: executed.touched, ledger }
    }
}

fn nonzero(amount: u16) -> Result<(), PoolError> {
    if amount == 0 { Err(PoolError::ZeroAmount) } else { Ok(()) }
}
