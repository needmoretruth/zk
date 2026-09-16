//! What the Toy Shielded Pool shows: every receipt split into what the world sees and what only
//! the wallets see, the ledger, wallets, attacks, and the pool's own errors.

mod attack;
mod ledger;
mod receipt;

use std::path::Path;

use zk_i18n::Language;
use zk_pool::{PoolError, Rejection};

pub(crate) use attack::{attack, name as attack_name};
pub(crate) use ledger::{ledger, wallet, wallets};
pub(crate) use receipt::receipt;

use crate::activities::pool::{PoolAction, PoolSystem};
use crate::doc::{Block, Doc, Entry, Hue, Kind, Table, Tone, plain, span};
use crate::format;
use crate::phrases::fill;
use crate::phrases::pool::Msg as M;

/// Bytes of a commitment, nullifier, root or proof shown on screen.
pub(crate) const SHOWN_BYTES: usize = 6;

/// A bold heading line.
pub(crate) fn heading(doc: &mut Doc, text: impl Into<String>) {
    doc.line(vec![span(text, Tone::BODY.bold())]);
}

/// The line that says what this toy is and is not, wherever it names Zcash.
pub(crate) fn disclaimer(doc: &mut Doc, language: Language) {
    doc.line(vec![span(M::Disclaimer.text(language), Tone::of(Hue::Secondary))]);
}

/// Coins, with thousands separated.
pub(crate) fn coins(amount: impl Into<u64>) -> String {
    format::count(amount.into())
}

/// Every action with what it does.
pub(crate) fn help(language: Language) -> Entry {
    let rows = [
        ("/pool use <groth16|halo2>", M::HelpUse),
        ("/pool wallet new <name>", M::HelpWalletNew),
        ("/pool wallet <name>", M::HelpWallet),
        ("/pool wallets", M::HelpWallets),
        ("/pool faucet <name> <amount>", M::HelpFaucet),
        ("/pool shield <name> <amount>", M::HelpShield),
        ("/pool send <from> <to> <amount>", M::HelpSend),
        ("/pool unshield <name> <amount>", M::HelpUnshield),
        ("/pool ledger", M::HelpLedger),
        ("/pool attack <double-spend|steal|counterfeit|unbound-nullifier>", M::HelpAttack),
        ("/pool reset --yes", M::HelpReset),
    ]
    .into_iter()
    .map(|(usage, msg)| {
        vec![vec![plain(usage)], vec![span(msg.text(language), Tone::of(Hue::Secondary))]]
    })
    .collect();
    let mut doc = Doc::new();
    heading(&mut doc, M::HelpTitle.text(language));
    doc.push(Block::Table(Table { rows, wrap: true, ..Table::default() }));
    disclaimer(&mut doc, language);
    Entry::new(Kind::Result, doc)
}

/// How to go ahead with a reset.
pub(crate) fn confirm_reset(system: PoolSystem, language: Language) -> Entry {
    Entry::text(Kind::Warning, fill(M::ConfirmReset.text(language), &[("system", system.name())]))
}

/// The proof system now in use and where its pool is kept.
pub(crate) fn chosen(system: PoolSystem, dir: &Path, language: Language) -> Entry {
    let mut doc = Doc::new();
    doc.line(vec![plain(fill(M::Chosen.text(language), &[("system", system.name())]))]);
    let place = fill(M::ChosenDir.text(language), &[("dir", &dir.display().to_string())]);
    doc.line(vec![span(place, Tone::of(Hue::Secondary))]);
    disclaimer(&mut doc, language);
    Entry::new(Kind::Result, doc)
}

/// Why the pool could not do what was asked.
pub(crate) fn error(error: &PoolError, language: Language) -> Entry {
    let text = |msg: M, values: &[(&str, &str)]| fill(msg.text(language), values);
    let text = match error {
        PoolError::InvalidName(name) => text(M::ErrInvalidName, &[("name", name)]),
        PoolError::WalletExists(name) => text(M::ErrWalletExists, &[("name", name)]),
        PoolError::UnknownWallet(name) => text(M::ErrUnknownWallet, &[("name", name)]),
        PoolError::ZeroAmount => text(M::ErrZeroAmount, &[]),
        PoolError::NotEnoughTransparentFunds { name, balance, amount } => text(
            M::ErrNotEnoughTransparent,
            &[("name", name), ("balance", &coins(*balance)), ("amount", &coins(*amount))],
        ),
        PoolError::NoNoteLargeEnough { name, amount, largest } => text(
            M::ErrNoNoteLargeEnough,
            &[("name", name), ("amount", &coins(*amount)), ("largest", &coins(*largest))],
        ),
        PoolError::Storage(why) => text(M::ErrStorage, &[("why", why)]),
        PoolError::Corrupt(why) => text(M::ErrCorrupt, &[("why", why)]),
        PoolError::Mismatch { expected, found } => {
            text(M::ErrMismatch, &[("expected", expected), ("found", found)])
        }
        PoolError::Circuit(why) => text(M::ErrCircuit, &[("why", why)]),
        PoolError::Randomness(why) => text(M::ErrRandomness, &[("why", why)]),
        PoolError::System(error) => {
            text(M::ErrSystem, &[("why", &super::stage::system_error(error, language))])
        }
    };
    Entry::text(Kind::Error, text)
}

/// What an action that proves is doing, in words; `None` for actions that do not prove.
pub(crate) fn proving_text(
    action: &PoolAction,
    system: PoolSystem,
    language: Language,
) -> Option<String> {
    let sys = ("system", system.name());
    Some(match action {
        PoolAction::Shield { name, amount } => fill(
            M::ProvingShield.text(language),
            &[("name", name), ("amount", &coins(*amount)), sys],
        ),
        PoolAction::Send { from, to, amount } => fill(
            M::ProvingSend.text(language),
            &[("from", from), ("to", to), ("amount", &coins(*amount)), sys],
        ),
        PoolAction::Unshield { name, amount } => fill(
            M::ProvingUnshield.text(language),
            &[("name", name), ("amount", &coins(*amount)), sys],
        ),
        PoolAction::Attack(attack) => fill(
            M::ProvingAttack.text(language),
            &[("attack", attack::name(*attack).text(language)), sys],
        ),
        _ => return None,
    })
}

/// The running cell and working line for an action that proves; `None` for the others.
pub(crate) fn proving(
    action: &PoolAction,
    system: PoolSystem,
    language: Language,
) -> Option<(Entry, String)> {
    let text = proving_text(action, system, language)?;
    let status = fill(M::WorkingProve.text(language), &[("system", system.name())]);
    Some((Entry::text(Kind::Running, text), status))
}

/// A reason a ledger refused a transaction, in words.
pub(crate) fn reason(rejection: Rejection, before: u64, after: i64, language: Language) -> String {
    let msg = match rejection {
        Rejection::ProofRejected => M::ReasonProof,
        Rejection::UnknownRoot => M::ReasonRoot,
        Rejection::NullifierAlreadySpent => M::ReasonNullifier,
        Rejection::NotEnoughTransparentFunds => M::ReasonFunds,
        Rejection::TurnstileWouldGoNegative => M::ReasonTurnstile,
        Rejection::TreeFull => M::ReasonTree,
    };
    fill(msg.text(language), &[("before", &before.to_string()), ("after", &after.to_string())])
}
