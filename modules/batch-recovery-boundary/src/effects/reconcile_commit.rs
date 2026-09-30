use crate::driver::{NativeContext, Output};
use crate::representation::BatchRecoveryBoundaryEffectResult as R;

pub fn execute(
    _effect: crate::representation::BatchRecoveryBoundaryEffect,
    context: &mut NativeContext,
) -> crate::representation::BatchRecoveryBoundaryEffectResult {
    let Some(control) = context.control.as_ref() else {
        return R::PublicationStillUnknown;
    };
    match crate::git::reconcile_commit(&context.root, control) {
        Ok(Some(oid)) => {
            context.output = Some(Output::Commit(oid));
            context.error = None;
            R::PublicationReconciled(true)
        }
        Ok(None) => R::PublicationReconciled(false),
        Err(_) => R::PublicationStillUnknown,
    }
}
