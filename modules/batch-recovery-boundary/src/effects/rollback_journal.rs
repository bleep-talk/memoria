use crate::driver::NativeContext;
use crate::repo::RepoError;
use crate::representation::BatchRecoveryBoundaryEffectResult as R;

pub fn execute(
    _effect: crate::representation::BatchRecoveryBoundaryEffect,
    context: &mut NativeContext,
) -> crate::representation::BatchRecoveryBoundaryEffectResult {
    let result = (|| {
        context.journal()?.rollback_change(
            context.files()?,
            &context.policy,
            context.rollback_index,
        )?;
        crate::journal::crash("rollback_after_change");
        Ok::<_, RepoError>(context.journal()?.changes().len())
    })();
    match result {
        Ok(total) => {
            context.rollback_index += 1;
            R::RollbackDurable(context.rollback_index < total)
        }
        Err(error @ RepoError::RecoveryConflict(_)) => context.fail(error, R::RollbackConflict),
        Err(error) => context.fail(error, R::RollbackIoError),
    }
}
