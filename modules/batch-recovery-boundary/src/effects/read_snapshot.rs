use crate::driver::NativeContext;
use crate::repo::RepoError;
use crate::representation::{BatchRecoveryBoundaryEffectResult as R, SnapshotFact};

pub fn execute(
    _effect: crate::representation::BatchRecoveryBoundaryEffect,
    context: &mut NativeContext,
) -> crate::representation::BatchRecoveryBoundaryEffectResult {
    match context.inspect_snapshot() {
        Ok(fact) => R::SnapshotObserved(fact),
        Err(error @ RepoError::Memory(_)) | Err(error @ RepoError::NotFound(_)) => {
            context.fail(error, R::SnapshotObserved(SnapshotFact::Rejected))
        }
        Err(error) => context.fail(error, R::SnapshotIoError),
    }
}
