//! The two views the pool is about: the ledger everyone reads, and a wallet only its owner reads.

use zk_i18n::Language;
use zk_pool::{LedgerView, SpendCircuit, TransactionKind, WalletView};

use super::receipt::transparent;
use super::{SHOWN_BYTES, coins, disclaimer, heading};
use crate::activities::pool::PoolSystem;
use crate::activities::short_hex;
use crate::doc::{Align, Block, Doc, Entry, Hue, Kind, Span, Table, Tone, plain, span};
use crate::format;
use crate::phrases::fill;
use crate::phrases::pool::Msg as M;

fn header(language: Language, columns: &[M]) -> Vec<Vec<Span>> {
    columns.iter().map(|msg| vec![plain(msg.text(language))]).collect()
}

fn circuit_id(circuit: SpendCircuit) -> &'static str {
    circuit.example().id()
}

fn kind_name(kind: TransactionKind, language: Language) -> &'static str {
    match kind {
        TransactionKind::Faucet => M::KindFaucet,
        TransactionKind::Shield => M::KindShield,
        TransactionKind::ShieldedTransfer => M::KindShieldedTransfer,
        TransactionKind::Unshield => M::KindUnshield,
    }
    .text(language)
}

/// What the world sees: balances in the open, the tree, the nullifiers and every transaction.
pub(crate) fn ledger(ledger: &LedgerView, system: PoolSystem, language: Language) -> Entry {
    let mut doc = Doc::new();
    heading(&mut doc, fill(M::LedgerTitle.text(language), &[("system", system.name())]));
    let balances: Vec<String> = ledger
        .transparent
        .iter()
        .map(|(name, amount)| format!("{name} {}", coins(*amount)))
        .collect();
    let balances = if balances.is_empty() {
        M::LedgerNone.text(language).to_string()
    } else {
        balances.join(" · ")
    };
    let tree = fill(
        M::LedgerTreeValue.text(language),
        &[
            ("used", &ledger.commitments.len().to_string()),
            ("capacity", &ledger.tree_capacity.to_string()),
        ],
    );
    let pairs = [
        (M::LedgerCircuit, circuit_id(ledger.circuit).to_string()),
        (M::LedgerPoolValue, coins(ledger.pool_balance)),
        (M::LedgerTree, tree),
        (M::LedgerRoot, short_hex(&ledger.root, SHOWN_BYTES)),
        (M::LedgerNullifiers, ledger.nullifiers.len().to_string()),
        (M::LedgerTransparent, balances),
    ];
    doc.pairs(
        pairs
            .into_iter()
            .map(|(label, value)| (label.text(language).into(), vec![plain(value)]))
            .collect(),
    );
    if ledger.transactions.is_empty() {
        doc.line(vec![span(M::LedgerEmpty.text(language), Tone::of(Hue::Secondary))]);
    } else {
        doc.push(transactions(ledger, language));
    }
    disclaimer(&mut doc, language);
    Entry::new(Kind::Result, doc)
}

fn transactions(ledger: &LedgerView, language: Language) -> Block {
    let secondary = Tone::of(Hue::Secondary);
    let rows = ledger
        .transactions
        .iter()
        .map(|record| {
            let moved = record.transparent.as_ref().map(|m| transparent(m, language));
            let shielded = record.shielded.as_ref();
            let nullifier = shielded.map(|s| short_hex(&s.nullifier, 4)).unwrap_or_default();
            let notes = shielded
                .map(|s| {
                    format!(
                        "{} {}",
                        short_hex(&s.commitments[0], 4),
                        short_hex(&s.commitments[1], 4)
                    )
                })
                .unwrap_or_default();
            let proof = shielded.map(|s| format::size(s.proof_bytes)).unwrap_or_default();
            vec![
                vec![plain(record.index.to_string())],
                vec![plain(kind_name(record.kind, language))],
                vec![plain(moved.unwrap_or_default())],
                vec![span(nullifier, secondary)],
                vec![span(notes, secondary)],
                vec![plain(proof)],
            ]
        })
        .collect();
    let columns =
        [M::ColIndex, M::ColKind, M::ColTransparent, M::ColNullifier, M::ColNotes, M::ColProof];
    Block::Table(Table {
        header: header(language, &columns),
        align: vec![Align::Right, Align::Left, Align::Left, Align::Left, Align::Left, Align::Right],
        rows,
        wrap: false,
        optional: vec![4, 3, 5],
    })
}

/// What only a wallet's owner sees: its key, its address, its notes and both balances.
pub(crate) fn wallet(
    wallet: &WalletView,
    transparent: u64,
    system: PoolSystem,
    language: Language,
) -> Entry {
    let name = wallet.name.as_str();
    let title = fill(M::WalletTitle.text(language), &[("name", name), ("system", system.name())]);
    let mut doc = Doc::new();
    heading(&mut doc, title);
    let public =
        span(format!(" · {}", M::WalletTransparentPublic.text(language)), Tone::of(Hue::Secondary));
    doc.pairs(vec![
        (
            M::FieldAddress.text(language).into(),
            vec![plain(short_hex(&wallet.address, SHOWN_BYTES))],
        ),
        (
            M::FieldKey.text(language).into(),
            vec![plain(short_hex(&wallet.spending_key, SHOWN_BYTES))],
        ),
        (M::WalletShielded.text(language).into(), vec![plain(coins(wallet.balance))]),
        (M::WalletTransparent.text(language).into(), vec![plain(coins(transparent)), public]),
    ]);
    if wallet.notes.is_empty() {
        doc.line(vec![span(M::WalletNoNotes.text(language), Tone::of(Hue::Secondary))]);
    } else {
        doc.push(notes(wallet, language));
    }
    Entry::new(Kind::Result, doc)
}

fn notes(wallet: &WalletView, language: Language) -> Block {
    let secondary = Tone::of(Hue::Secondary);
    let rows = wallet
        .notes
        .iter()
        .enumerate()
        .map(|(index, note)| {
            let state = if note.spent { M::StateSpent } else { M::StateUnspent };
            vec![
                vec![plain((index + 1).to_string())],
                vec![plain(coins(note.value))],
                vec![plain(note.position.to_string())],
                vec![plain(note.received_in.to_string())],
                vec![plain(state.text(language))],
                vec![span(short_hex(&note.commitment, 4), secondary)],
                vec![span(short_hex(&note.nullifier, 4), secondary)],
            ]
        })
        .collect();
    let columns = [
        M::ColNote,
        M::ColValue,
        M::ColPosition,
        M::ColReceived,
        M::ColState,
        M::ColCommitment,
        M::ColNullifier,
    ];
    let right = Align::Right;
    Block::Table(Table {
        header: header(language, &columns),
        align: vec![right, right, right, right, Align::Left, Align::Left, Align::Left],
        rows,
        wrap: false,
        optional: vec![6, 5, 3],
    })
}

/// Every wallet on this machine with both balances.
pub(crate) fn wallets(
    wallets: &[(WalletView, u64)],
    system: PoolSystem,
    language: Language,
) -> Entry {
    let mut doc = Doc::new();
    heading(&mut doc, fill(M::WalletsTitle.text(language), &[("system", system.name())]));
    if wallets.is_empty() {
        doc.line(vec![span(M::WalletsEmpty.text(language), Tone::of(Hue::Secondary))]);
        return Entry::new(Kind::Result, doc);
    }
    let rows = wallets
        .iter()
        .map(|(wallet, transparent)| {
            vec![
                vec![plain(wallet.name.as_str())],
                vec![plain(coins(wallet.balance))],
                vec![plain(coins(*transparent))],
                vec![plain(wallet.notes.len().to_string())],
            ]
        })
        .collect();
    let columns = [M::ColName, M::ColShielded, M::ColTransparent, M::ColNoteCount];
    doc.push(Block::Table(Table {
        header: header(language, &columns),
        align: vec![Align::Left, Align::Right, Align::Right, Align::Right],
        rows,
        ..Table::default()
    }));
    Entry::new(Kind::Result, doc)
}
