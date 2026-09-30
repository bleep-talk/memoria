use crate::driver::NativeContext;
use crate::repo::RepoError;
use crate::representation::{BatchRecoveryBoundaryEffectResult as R, GitFact};

pub fn execute(
    _effect: crate::representation::BatchRecoveryBoundaryEffect,
    context: &mut NativeContext,
) -> crate::representation::BatchRecoveryBoundaryEffectResult {
    match context.inspect_git() {
        Ok(fact) => R::GitObserved(fact),
        Err(error @ RepoError::StagedChanges) | Err(error @ RepoError::HeadMismatch) => {
            context.fail(error, R::GitObserved(GitFact::Rejected))
        }
        Err(error) => context.fail(error, R::GitInspectionError),
    }
}
