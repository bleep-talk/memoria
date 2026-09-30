use crate::driver::NativeContext;
use crate::representation::BatchRecoveryBoundaryEffectResult as R;

pub fn execute(
    _effect: crate::representation::BatchRecoveryBoundaryEffect,
    context: &mut NativeContext,
) -> crate::representation::BatchRecoveryBoundaryEffectResult {
    let Some(lock) = context.lock.take() else {
        return R::LockReleaseError;
    };
    let result = rustix::fs::flock(&lock, rustix::fs::FlockOperation::Unlock);
    drop(lock);
    match result {
        Ok(()) => R::LockReleased,
        Err(error) => context.fail(crate::repo::RepoError::Sys(error), R::LockReleaseError),
    }
}
