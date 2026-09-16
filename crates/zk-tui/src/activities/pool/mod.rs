//! `/pool`: the Toy Shielded Pool, kept on this machine, proved with Groth16 or Halo 2.

pub mod store;

use std::path::PathBuf;

use serde_json::json;
use zk_circuit::ZkField;
use zk_core::{Control, SystemError};
use zk_i18n::Language;
use zk_pool::{Attack, AttackReport, LedgerView, Pool, PoolError, Receipt, WalletView};

use crate::activity::{Activity, Outbox};
use crate::doc::{Entry, Kind};
use crate::phrases::fill;
use crate::phrases::pool::Msg as M;
use crate::views::pool as view;

pub use store::PoolCache;
use store::{AnyPool, choose_system, chosen_system, data_root, pool_dir};

/// The proof system a pool's spends are proved with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PoolSystem {
    /// Groth16 on BLS12-381, as Zcash Sapling proves spends.
    #[default]
    Groth16,
    /// Halo 2 on the Pasta curves, as Zcash Orchard proves actions.
    Halo2,
}

impl PoolSystem {
    /// Both, in the order Zcash used them.
    pub const ALL: [PoolSystem; 2] = [Self::Groth16, Self::Halo2];

    /// The word typed after `/pool use`, also the name of its directory.
    pub fn key(self) -> &'static str {
        match self {
            Self::Groth16 => "groth16",
            Self::Halo2 => "halo2",
        }
    }

    /// The system a typed word names.
    pub fn from_key(key: &str) -> Option<PoolSystem> {
        Self::ALL.into_iter().find(|system| system.key() == key)
    }

    /// Its display name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Groth16 => "Groth16",
            Self::Halo2 => "Halo 2",
        }
    }
}

/// The words typed after `/pool attack`.
pub fn attack_key(attack: Attack) -> &'static str {
    match attack {
        Attack::DoubleSpend => "double-spend",
        Attack::Steal => "steal",
        Attack::Counterfeit => "counterfeit",
        Attack::UnboundNullifier => "unbound-nullifier",
    }
}

/// The attack a typed word names.
pub fn attack_from_key(key: &str) -> Option<Attack> {
    Attack::ALL.into_iter().find(|attack| attack_key(*attack) == key)
}

/// An amount the pool can move: a whole number from 0 to 65,535. Zero parses, and the pool
/// refuses it with its own reason.
pub fn parse_amount(text: &str) -> Option<u16> {
    text.parse().ok()
}

/// One thing to do with the pool.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PoolAction {
    /// The list of actions.
    Help,
    /// Prove with this system from now on.
    Use(PoolSystem),
    /// Create a wallet.
    NewWallet(String),
    /// What one wallet's owner sees.
    Wallet(String),
    /// Every wallet with its balances.
    Wallets,
    /// Credit transparent coins in the open.
    Faucet {
        /// The wallet whose transparent account is credited.
        name: String,
        /// Coins.
        amount: u16,
    },
    /// Move transparent coins into a note.
    Shield {
        /// The wallet that shields.
        name: String,
        /// Coins.
        amount: u16,
    },
    /// Pay from one wallet's note to another wallet.
    Send {
        /// The paying wallet.
        from: String,
        /// The paid wallet.
        to: String,
        /// Coins.
        amount: u16,
    },
    /// Take coins out of a note into the transparent account.
    Unshield {
        /// The wallet that unshields.
        name: String,
        /// Coins.
        amount: u16,
    },
    /// What the world sees.
    Ledger,
    /// Run an attack in scratch pools.
    Attack(Attack),
    /// Delete this system's ledger and wallets.
    Reset {
        /// Whether the reader confirmed; without it the pool only says how to.
        confirmed: bool,
    },
}

/// `/pool <action>`.
#[derive(Clone, Debug)]
pub struct PoolRun {
    /// What to do.
    pub action: PoolAction,
    /// `--data-dir`, or `None` for the platform's data directory.
    pub data_dir: Option<PathBuf>,
    /// The pool a TUI session keeps open; a fresh cache opens the pool from disk.
    pub cache: PoolCache,
    /// The language of every cell.
    pub language: Language,
}

impl Activity for PoolRun {
    fn subject(&self) -> String {
        M::Subject.text(self.language).to_string()
    }

    fn run(self: Box<Self>, outbox: &Outbox, control: &Control) {
        let language = self.language;
        if self.action == PoolAction::Help {
            outbox.open(view::help(language));
            return;
        }
        let Some(root) = data_root(self.data_dir.as_deref()) else {
            outbox.open(Entry::text(Kind::Error, M::NoDataDir.text(language)));
            return;
        };
        let system = chosen_system(&root);
        let dir = pool_dir(&root, system);
        match &self.action {
            PoolAction::Reset { confirmed: false } => {
                outbox.open(view::confirm_reset(system, language));
            }
            PoolAction::Use(chosen) => {
                let entry = match choose_system(&root, *chosen) {
                    Ok(()) => view::chosen(*chosen, &pool_dir(&root, *chosen), language),
                    Err(error) => view::error(&PoolError::Storage(error.to_string()), language),
                };
                outbox
                    .record(json!({ "activity": "pool", "action": "use", "system": chosen.key() }));
                outbox.open(entry);
            }
            action => {
                let proving = view::proving(action, system, language);
                let cell = proving.map(|(entry, status)| {
                    outbox.status(status);
                    outbox.open(entry)
                });
                let result = self.cache.with((system, &dir), control, |pool| match pool {
                    AnyPool::Groth16(pool) => perform(pool, action),
                    AnyPool::Halo2(pool) => perform(pool, action),
                });
                let (entry, record) = shown(result, action, system, language);
                if let Some(record) = record {
                    outbox.record(record);
                }
                match cell {
                    Some(cell) => outbox.replace(cell, entry),
                    None => {
                        outbox.open(entry);
                    }
                }
            }
        }
    }
}

/// What an action produced, before it is drawn.
pub(crate) enum Outcome {
    Receipt(Box<Receipt>),
    Wallet(WalletView, u64),
    Wallets(Vec<(WalletView, u64)>),
    Ledger(LedgerView),
    Attack(AttackReport),
    Reset,
}

fn perform<F: ZkField>(pool: &mut Pool<F>, action: &PoolAction) -> Result<Outcome, PoolError> {
    let transparent =
        |ledger: &LedgerView, name: &str| ledger.transparent.get(name).copied().unwrap_or(0);
    Ok(match action {
        PoolAction::NewWallet(name) => Outcome::Receipt(Box::new(pool.new_wallet(name)?)),
        PoolAction::Faucet { name, amount } => {
            Outcome::Receipt(Box::new(pool.faucet(name, *amount)?))
        }
        PoolAction::Shield { name, amount } => {
            Outcome::Receipt(Box::new(pool.shield(name, *amount)?))
        }
        PoolAction::Send { from, to, amount } => {
            Outcome::Receipt(Box::new(pool.send(from, to, *amount)?))
        }
        PoolAction::Unshield { name, amount } => {
            Outcome::Receipt(Box::new(pool.unshield(name, *amount)?))
        }
        PoolAction::Wallet(name) => {
            let wallet = pool.wallet(name)?;
            Outcome::Wallet(wallet, transparent(&pool.ledger(), name))
        }
        PoolAction::Wallets => {
            let ledger = pool.ledger();
            let names = pool.wallet_names();
            let wallets = names.iter().map(|name| pool.wallet(name));
            let wallets = wallets.collect::<Result<Vec<_>, _>>()?;
            let with_transparent = wallets.into_iter().map(|w| {
                let t = transparent(&ledger, &w.name);
                (w, t)
            });
            Outcome::Wallets(with_transparent.collect())
        }
        PoolAction::Ledger => Outcome::Ledger(pool.ledger()),
        PoolAction::Attack(attack) => Outcome::Attack(pool.attack(*attack)?),
        PoolAction::Reset { .. } => {
            pool.reset()?;
            Outcome::Reset
        }
        // Help and choosing a system never reach a pool; were they to, the ledger is harmless.
        PoolAction::Help | PoolAction::Use(_) => Outcome::Ledger(pool.ledger()),
    })
}

/// The cell for an action's result and its record: an error cell for an error, and a warning
/// when a stop came before the ledger decided.
fn shown(
    result: Result<Outcome, PoolError>,
    action: &PoolAction,
    system: PoolSystem,
    language: Language,
) -> (Entry, Option<serde_json::Value>) {
    let outcome = match result {
        Ok(outcome) => outcome,
        Err(PoolError::System(SystemError::Cancelled)) => {
            return (Entry::text(Kind::Warning, M::Stopped.text(language)), None);
        }
        Err(error) => {
            let record =
                json!({ "activity": "pool", "system": system.key(), "error": error.to_string() });
            return (view::error(&error, language), Some(record));
        }
    };
    let (entry, data) = match &outcome {
        Outcome::Receipt(receipt) => {
            (view::receipt(receipt, action, system, language), json!(receipt))
        }
        Outcome::Wallet(wallet, transparent) => (
            view::wallet(wallet, *transparent, system, language),
            json!({ "wallet": wallet, "transparent": transparent }),
        ),
        Outcome::Wallets(wallets) => {
            let list: Vec<_> = wallets
                .iter()
                .map(|(wallet, transparent)| {
                    json!({
                        "name": wallet.name,
                        "shielded": wallet.balance,
                        "transparent": transparent,
                        "notes": wallet.notes.len(),
                    })
                })
                .collect();
            (view::wallets(wallets, system, language), json!({ "wallets": list }))
        }
        Outcome::Ledger(ledger) => (view::ledger(ledger, system, language), json!(ledger)),
        Outcome::Attack(report) => (view::attack(report, system, language), json!(report)),
        Outcome::Reset => {
            let text = fill(M::ResetDone.text(language), &[("system", system.name())]);
            (Entry::text(Kind::Result, text), json!({ "reset": true }))
        }
    };
    let mut record = json!({ "activity": "pool", "system": system.key() });
    record["result"] = data;
    (entry, Some(record))
}
