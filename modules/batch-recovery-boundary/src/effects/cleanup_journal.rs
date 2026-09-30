use crate::driver::NativeContext;
use crate::representation::BatchRecoveryBoundaryEffectResult as R;

pub fn execute(
    _effect: crate::representation::BatchRecoveryBoundaryEffect,
    context: &mut NativeContext,
) -> crate::representation::BatchRecoveryBoundaryEffectResult {
    let result = match (context.journal(), context.control()) {
        (Ok(journal), Ok(control)) => journal.cleanup(control),
        (Err(error), _) | (_, Err(error)) => Err(error),
    };
    match result {
        Ok(()) => {
            context.journal = None;
            R::JournalCleanupDurable
        }
        Err(error) => context.fail(error, R::JournalCleanupIoError),
    }
}
