use crate::driver::NativeContext;
use crate::fs::Confined;
use crate::repo::RepoError;
use crate::representation::BatchRecoveryBoundaryEffectResult as R;
use std::fs::File;

fn acquire(context: &NativeContext) -> Result<(Confined, Confined, File), RepoError> {
    let files = crate::fs::execute_confined(&context.root)?;
    let control = files.control()?;
    let lock = control.lock()?;
    Ok((files, control, lock))
}

pub fn execute(
    _effect: crate::representation::BatchRecoveryBoundaryEffect,
    context: &mut NativeContext,
) -> crate::representation::BatchRecoveryBoundaryEffectResult {
    match acquire(context) {
        Ok((files, control, lock)) => {
            context.files = Some(files);
            context.control = Some(control);
            context.lock = Some(lock);
            R::LockAcquired
        }
        Err(error) => context.fail(error, R::LockUnavailable),
    }
}
