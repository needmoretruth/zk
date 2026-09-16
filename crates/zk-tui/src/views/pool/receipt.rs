//! A receipt: what the world sees, what only the wallets see, the verifier's verdict and every
//! rule the ledger applied.

use zk_core::Verdict;
use zk_i18n::Language;
use zk_pool::{
    Activity, Decision, Direction, InputNote, NoteDelivery, OutputNote, Receipt, Rejection,
    ShieldedPublic, TransactionKind, TransparentMove,
};

use super::{SHOWN_BYTES, coins, heading};
use crate::activities::pool::{PoolAction, PoolSystem};
use crate::activities::short_hex;
use crate::doc::{Doc, Entry, Hue, Kind, Span, Tone, plain, span};
use crate::format;
use crate::phrases::fill;
use crate::phrases::pool::Msg as M;

/// Every rule a ledger applies to a shielded transaction, in the order it applies them.
const RULES: [(Rejection, M); 6] = [
    (Rejection::ProofRejected, M::RuleProof),
    (Rejection::UnknownRoot, M::RuleRoot),
    (Rejection::NullifierAlreadySpent, M::RuleNullifier),
    (Rejection::NotEnoughTransparentFunds, M::RuleFunds),
    (Rejection::TurnstileWouldGoNegative, M::RuleTurnstile),
    (Rejection::TreeFull, M::RuleTree),
];

fn mark(good: bool) -> Span {
    if good { span("✓ ", Tone::of(Hue::Success)) } else { span("✗ ", Tone::of(Hue::Failure)) }
}

/// The whole receipt of `action`, proved with `system`.
pub(crate) fn receipt(
    receipt: &Receipt,
    action: &PoolAction,
    system: PoolSystem,
    language: Language,
) -> Entry {
    let mut doc = Doc::new();
    headline(&mut doc, receipt, action, system, language);
    heading(&mut doc, M::WorldHeading.text(language));
    world(&mut doc, receipt, language);
    heading(&mut doc, M::OwnerHeading.text(language));
    owner(&mut doc, receipt, action, language);
    if let Some(decision) = &receipt.decision {
        verdict(&mut doc, decision, system, language);
        rules(&mut doc, decision, language);
    }
    if let Some(micros) = receipt.setup_micros {
        let text = fill(
            M::KeysBuilt.text(language),
            &[("system", system.name()), ("time", &format::duration(micros))],
        );
        doc.line(vec![span(text, Tone::of(Hue::Secondary))]);
    }
    Entry::new(Kind::Result, doc)
}

fn headline(
    doc: &mut Doc,
    receipt: &Receipt,
    action: &PoolAction,
    system: PoolSystem,
    language: Language,
) {
    if !receipt.accepted() {
        let tried = super::proving_text(action, system, language).unwrap_or_default();
        doc.led(vec![mark(false)], vec![span(tried, Tone::of(Hue::Failure).bold())]);
        doc.line(vec![span(M::ReceiptRefused.text(language), Tone::of(Hue::Failure))]);
        return;
    }
    let index = receipt.public.recorded.map_or_else(String::new, |index| index.to_string());
    let index = ("index", index.as_str());
    let text = match action {
        PoolAction::NewWallet(name) => fill(M::ReceiptNewWallet.text(language), &[("name", name)]),
        PoolAction::Faucet { name, amount } => fill(
            M::ReceiptFaucet.text(language),
            &[("name", name), ("amount", &coins(*amount)), index],
        ),
        PoolAction::Shield { name, amount } => fill(
            M::ReceiptShield.text(language),
            &[("name", name), ("amount", &coins(*amount)), index],
        ),
        PoolAction::Send { from, to, amount } => fill(
            M::ReceiptSend.text(language),
            &[("from", from), ("to", to), ("amount", &coins(*amount)), index],
        ),
        PoolAction::Unshield { name, amount } => fill(
            M::ReceiptUnshield.text(language),
            &[("name", name), ("amount", &coins(*amount)), index],
        ),
        _ => String::new(),
    };
    doc.led(vec![mark(true)], vec![span(text, Tone::BODY.bold())]);
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

/// A transparent balance change, as everyone sees it.
pub(crate) fn transparent(movement: &TransparentMove, language: Language) -> String {
    let msg = match movement.direction {
        Direction::Credit => M::TransparentCredit,
        Direction::Debit => M::TransparentDebit,
    };
    fill(msg.text(language), &[("account", &movement.account), ("amount", &coins(movement.amount))])
}

fn world(doc: &mut Doc, receipt: &Receipt, language: Language) {
    let public = &receipt.public;
    let mut pairs: Vec<(String, Vec<Span>)> = Vec::new();
    let mut pair = |label: M, text: String| {
        pairs.push((label.text(language).to_string(), vec![plain(text)]));
    };
    if let Some(kind) = public.kind {
        pair(M::FieldKind, kind_name(kind, language).to_string());
    }
    if let Some(movement) = &public.transparent {
        pair(M::FieldTransparent, transparent(movement, language));
    }
    if let Some(shielded) = &public.shielded {
        shielded_pairs(shielded, language, &mut pair);
    }
    if pairs.is_empty() {
        doc.tree(vec![vec![span(M::WorldNothing.text(language), Tone::of(Hue::Secondary))]]);
    } else {
        doc.pairs(pairs);
    }
}

fn shielded_pairs(shielded: &ShieldedPublic, language: Language, pair: &mut impl FnMut(M, String)) {
    let short = |hex: &str| short_hex(hex, SHOWN_BYTES);
    pair(M::FieldAnchor, short(&shielded.root));
    pair(M::FieldNullifier, short(&shielded.nullifier));
    pair(
        M::FieldCommitments,
        format!("{}  {}", short(&shielded.commitments[0]), short(&shielded.commitments[1])),
    );
    pair(
        M::FieldValue,
        fill(
            M::ValueInOut.text(language),
            &[("in", &coins(shielded.v_pub_in)), ("out", &coins(shielded.v_pub_out))],
        ),
    );
    pair(
        M::FieldProof,
        format!("{} · {}", format::size(shielded.proof_bytes), short(&shielded.proof)),
    );
    let timing = fill(
        M::Timing.text(language),
        &[
            ("prove", &format::duration(shielded.prove_micros)),
            ("verify", &format::duration(shielded.verify_micros)),
        ],
    );
    pair(M::FieldTiming, timing);
    if let Some([first, second]) = shielded.positions {
        pair(M::FieldPositions, format!("{first}, {second}"));
    }
}

fn owner(doc: &mut Doc, receipt: &Receipt, action: &PoolAction, language: Language) {
    let private = &receipt.private;
    let mut pairs: Vec<(String, Vec<Span>)> = Vec::new();
    if let (Some(address), PoolAction::NewWallet(name)) = (&private.address, action) {
        let key = fill(M::KeyKept.text(language), &[("name", name)]);
        pairs.push((
            M::FieldAddress.text(language).into(),
            vec![plain(short_hex(address, SHOWN_BYTES))],
        ));
        pairs.push((M::FieldKey.text(language).into(), vec![plain(key)]));
    }
    if let Some(input) = &private.input {
        pairs.push((M::FieldInput.text(language).into(), vec![plain(input_text(input, language))]));
    }
    for (index, output) in private.outputs.iter().enumerate() {
        let label = fill(M::FieldOutput.text(language), &[("number", &(index + 1).to_string())]);
        pairs.push((label, output_spans(output, language)));
    }
    if pairs.is_empty() {
        let note =
            if receipt.activity == Activity::Faucet { M::OwnerNothing } else { M::WorldNothing };
        doc.tree(vec![vec![span(note.text(language), Tone::of(Hue::Secondary))]]);
    } else {
        doc.pairs(pairs);
    }
}

fn input_text(input: &InputNote, language: Language) -> String {
    match input {
        InputNote::Dummy => M::InputDummy.text(language).to_string(),
        InputNote::Note { owner, key_holder, value, position } => {
            let values = [
                ("owner", owner.as_str()),
                ("holder", key_holder.as_str()),
                ("value", &coins(*value)),
                ("position", &position.to_string()),
            ];
            let msg = if owner == key_holder { M::InputNote } else { M::InputNoteOtherKey };
            fill(msg.text(language), &values)
        }
    }
}

fn output_spans(output: &OutputNote, language: Language) -> Vec<Span> {
    let value = output.value.to_string();
    let mut text = match &output.recipient {
        Some(recipient) => {
            fill(M::OutputTo.text(language), &[("value", &value), ("recipient", recipient)])
        }
        None => fill(M::OutputNobody.text(language), &[("value", &value)]),
    };
    if let Some(position) = output.position {
        let at = fill(M::OutputPosition.text(language), &[("position", &position.to_string())]);
        text = format!("{text} · {at}");
    }
    let mut spans = vec![plain(text)];
    if let Some(delivery) = output.delivery {
        let msg = match delivery {
            NoteDelivery::HandedToLocalWallet => M::DeliveryHanded,
            NoteDelivery::Discarded => M::DeliveryDiscarded,
        };
        spans.push(span(format!(" · {}", msg.text(language)), Tone::of(Hue::Secondary)));
    }
    spans
}

fn verdict(doc: &mut Doc, decision: &Decision, system: PoolSystem, language: Language) {
    let sys = ("system", system.name());
    let (good, text) = match &decision.proof {
        Verdict::Accepted => (true, fill(M::VerifierAccepted.text(language), &[sys])),
        Verdict::Rejected => (false, fill(M::VerifierRejected.text(language), &[sys])),
        Verdict::Malformed(why) => {
            (false, fill(M::VerifierMalformed.text(language), &[sys, ("why", why)]))
        }
    };
    doc.led(vec![mark(good)], vec![span(text, Tone::BODY.bold())]);
}

fn rules(doc: &mut Doc, decision: &Decision, language: Language) {
    heading(doc, M::RulesHeading.text(language));
    let before = decision.pool_balance_before.to_string();
    let after = decision.pool_balance_after.to_string();
    let items = RULES
        .iter()
        .map(|(rejection, msg)| {
            let held = !decision.rejections.contains(rejection);
            let text = fill(msg.text(language), &[("before", &before), ("after", &after)]);
            let tone = if held { Tone::BODY } else { Tone::of(Hue::Failure) };
            vec![mark(held), span(text, tone)]
        })
        .collect();
    doc.tree(items);
}
