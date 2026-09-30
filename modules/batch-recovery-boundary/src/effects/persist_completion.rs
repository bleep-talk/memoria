use crate::driver::NativeContext;
use crate::repo::RepoError;
use crate::representation::BatchRecoveryBoundaryEffectResult as R;

pub fn execute(
    _effect: crate::representation::BatchRecoveryBoundaryEffect,
    context: &mut NativeContext,
) -> crate::representation::BatchRecoveryBoundaryEffectResult {
    let result = (|| {
        let control = context.control.as_ref().ok_or(RepoError::InvalidJournal)?;
        let journal = context.journal.as_mut().ok_or(RepoError::InvalidJournal)?;
        crate::journal::crash("apply_complete_before_marker");
        journal.complete(control, &context.policy)
    })();
    match result {
        Ok(()) => {
            crate::journal::crash("completion_after_flush");
            R::CompletionDurable
        }
        Err(error) => context.fail(error, R::CompletionPersistenceUncertain),
    }
}
