//! Where a run is, in words: the working line while it runs, and where it stopped when it ends.

use zk_core::{AttackKind, Stage, SystemError};
use zk_i18n::Language;

use crate::phrases::fill;
use crate::phrases::run::Msg as R;

/// The working line's text for `system` in `stage`; `None` before the first stage arrives.
pub fn status(stage: Option<Stage>, system: &str, language: Language) -> String {
    let template = match stage {
        None => R::StageStarting,
        Some(Stage::Setup) => R::StageSetup,
        Some(Stage::Prove) => R::StageProve,
        Some(Stage::Verify) => R::StageVerify,
        Some(Stage::Attack(AttackKind::FlipProofByte)) => R::StageFlip,
        Some(Stage::Attack(AttackKind::BumpPublicInput)) => R::StageBump,
        Some(Stage::Attack(AttackKind::DishonestWitness)) => R::StageDishonest,
        Some(Stage::Round { round, of }) => {
            return fill(
                R::StageRound.text(language),
                &[("round", &round.to_string()), ("of", &of.to_string()), ("system", system)],
            );
        }
    };
    fill(template.text(language), &[("system", system)])
}

/// A stage as the end of "stopped after …" or "failed during …".
pub(crate) fn stage_name(stage: Stage, language: Language) -> String {
    let msg = match stage {
        Stage::Setup => R::AfterSetup,
        Stage::Prove => R::AfterProve,
        Stage::Verify => R::AfterVerify,
        Stage::Attack(AttackKind::FlipProofByte) => R::AfterFlip,
        Stage::Attack(AttackKind::BumpPublicInput) => R::AfterBump,
        Stage::Attack(AttackKind::DishonestWitness) => R::AfterDishonest,
        Stage::Round { round, of } => {
            return fill(
                R::AfterRound.text(language),
                &[("round", &round.to_string()), ("of", &of.to_string())],
            );
        }
    };
    msg.text(language).to_string()
}

/// What a system's own error says. The detail strings come from the system and are shown as given.
pub(crate) fn system_error(error: &SystemError, language: Language) -> String {
    match error {
        SystemError::Cancelled => R::ErrCancelled.text(language).to_string(),
        SystemError::Unsatisfied(why) => fill(R::ErrUnsatisfied.text(language), &[("why", why)]),
        SystemError::NotApplicable(why) => {
            fill(R::ErrNotApplicable.text(language), &[("why", why)])
        }
        SystemError::CompanionMissing { executable } => {
            fill(R::ErrCompanionMissing.text(language), &[("executable", executable)])
        }
        SystemError::Failed(why) => why.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_working_line_names_the_stage_and_the_system() {
        let english = Language::ENGLISH;
        assert_eq!(status(Some(Stage::Prove), "Groth16", english), "Proving with Groth16");
        assert_eq!(
            status(Some(Stage::Round { round: 3, of: 40 }), "Cave", english),
            "Round 3 of 40 with Cave"
        );
        assert_eq!(stage_name(Stage::Setup, english), "setup");
    }
}
