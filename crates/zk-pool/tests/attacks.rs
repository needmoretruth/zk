//! Each attack on a Groth16 pool: what the honest ledger refused and why, what the broken circuits
//! let through, and that the reader's own ledger never changed.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{ScratchDir, timing_line};
use zk_core::Verdict;
use zk_pool::{Attack, AttackAction, AttackReport, AttackStep, Pool, Rejection, SpendCircuit};

/// Runs `attack` on a pool holding one wallet and checks the pool is unchanged afterwards.
fn run(attack: Attack) -> AttackReport {
    let dir = ScratchDir::new("attack");
    let mut pool = Pool::groth16(dir.path()).unwrap();
    pool.new_wallet("alice").unwrap();
    pool.faucet("alice", 10).unwrap();
    let (ledger, wallets) = (pool.ledger(), pool.wallet_names());
    let report = pool.attack(attack).unwrap();
    for (index, step) in report.steps.iter().enumerate() {
        let label = format!("{attack:?} {index} {:?} {:?}", step.action, step.circuit);
        println!("{}", timing_line("groth16", &label, &step.receipt));
    }
    assert_eq!((pool.ledger(), pool.wallet_names()), (ledger, wallets));
    assert_eq!(Pool::groth16(dir.path()).unwrap().ledger(), pool.ledger());
    report
}

fn refused(step: &AttackStep, proof: Verdict, rejections: &[Rejection], violated: &[&str]) {
    let decision = step.receipt.decision.as_ref().unwrap();
    assert!(!decision.accepted, "{:?}", step.action);
    assert_eq!(decision.proof, proof, "{:?}", step.action);
    assert_eq!(decision.rejections, rejections, "{:?}", step.action);
    assert_eq!(step.violated, violated, "{:?}", step.action);
}

fn accepted(step: &AttackStep) {
    let decision = step.receipt.decision.as_ref().unwrap();
    assert!(decision.accepted, "{:?}: {decision:?}", step.action);
}

fn actions(report: &AttackReport) -> Vec<(AttackAction, SpendCircuit)> {
    report.steps.iter().map(|step| (step.action, step.circuit)).collect()
}

#[test]
fn a_double_spend_carries_a_valid_proof_and_is_stopped_by_the_nullifier_set() {
    let report = run(Attack::DoubleSpend);
    use {AttackAction::*, SpendCircuit::Honest};
    assert_eq!(
        actions(&report),
        [(AttackerShields, Honest), (SpendNote, Honest), (RespendSpentNote, Honest)]
    );
    accepted(&report.steps[0]);
    accepted(&report.steps[1]);
    refused(&report.steps[2], Verdict::Accepted, &[Rejection::NullifierAlreadySpent], &[]);
    assert_eq!(report.steps[2].attacker_balance, 40);
    assert!(report.honest_ledger_held);
}

#[test]
fn stealing_a_note_without_its_key_yields_a_proof_the_verifier_rejects() {
    let report = run(Attack::Steal);
    use {AttackAction::*, SpendCircuit::Honest};
    assert_eq!(actions(&report), [(VictimShields, Honest), (SpendNoteWithoutKey, Honest)]);
    accepted(&report.steps[0]);
    let membership = "input note is in the tree unless its value is zero";
    refused(&report.steps[1], Verdict::Rejected, &[Rejection::ProofRejected], &[membership]);
    assert_eq!(report.steps[1].attacker_balance, 0);
    assert!(report.honest_ledger_held);
}

#[test]
fn a_counterfeit_passes_only_without_range_checks_and_cannot_leave_through_the_turnstile() {
    let report = run(Attack::Counterfeit);
    use {AttackAction::*, SpendCircuit::*};
    let negative = SpendWithNegativeOutput { outputs: [105, -5] };
    assert_eq!(
        actions(&report),
        [
            (AttackerShields, Honest),
            (negative, Honest),
            (AttackerShields, WithoutRangeChecks),
            (negative, WithoutRangeChecks),
            (UnshieldEverything { amount: 105 }, WithoutRangeChecks),
        ]
    );
    let range = "v_out_2 fits in 16 bits";
    refused(&report.steps[1], Verdict::Rejected, &[Rejection::ProofRejected], &[range]);
    assert_eq!(report.steps[1].attacker_balance, 100);
    accepted(&report.steps[3]);
    assert_eq!((report.steps[3].attacker_balance, report.steps[3].pool_balance), (105, 100));
    let turnstile = &[Rejection::TurnstileWouldGoNegative];
    refused(&report.steps[4], Verdict::Accepted, turnstile, &[]);
    assert_eq!(report.steps[4].receipt.decision.as_ref().unwrap().pool_balance_after, -5);
    assert!(report.honest_ledger_held);
}

#[test]
fn an_unbound_nullifier_lets_one_note_be_spent_twice_only_in_the_broken_circuit() {
    let report = run(Attack::UnboundNullifier);
    use {AttackAction::*, SpendCircuit::*};
    let broken = WithoutNullifierBinding;
    assert_eq!(
        actions(&report),
        [
            (AttackerShields, Honest),
            (SpendNote, Honest),
            (RespendWithFreshNullifier, Honest),
            (AttackerShields, broken),
            (SpendNote, broken),
            (RespendWithFreshNullifier, broken),
        ]
    );
    let binding = "nullifier belongs to the input note and key";
    refused(&report.steps[2], Verdict::Rejected, &[Rejection::ProofRejected], &[binding]);
    accepted(&report.steps[4]);
    accepted(&report.steps[5]);
    let nullifier = |step: &AttackStep| step.receipt.public.shielded.clone().unwrap().nullifier;
    assert_ne!(nullifier(&report.steps[4]), nullifier(&report.steps[5]));
    assert_eq!((report.steps[5].attacker_balance, report.steps[5].nullifiers), (200, 3));
    assert!(report.honest_ledger_held);
}
