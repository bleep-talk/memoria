use crate::driver::NativeContext;
use crate::repo::RepoError;
use crate::representation::BatchRecoveryBoundaryEffectResult as R;

pub fn execute(
    _effect: crate::representation::BatchRecoveryBoundaryEffect,
    context: &mut NativeContext,
) -> crate::representation::BatchRecoveryBoundaryEffectResult {
    let result = (|| {
        context
            .journal()?
            .apply_change(context.files()?, &context.policy, context.apply_index)?;
        Ok::<_, RepoError>(context.journal()?.changes().len())
    })();
    match result {
        Ok(total) => {
            context.apply_index += 1;
            R::FilesystemPlanApplied(context.apply_index < total)
        }
        Err(error) => {
            context.failed_request = true;
            context.fail(error, R::FilesystemApplyInterrupted)
        }
    }
}
