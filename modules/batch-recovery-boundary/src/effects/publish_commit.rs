use crate::driver::{NativeContext, Operation, Output};
use crate::repo::RepoError;
use crate::representation::BatchRecoveryBoundaryEffectResult as R;

pub fn execute(
    _effect: crate::representation::BatchRecoveryBoundaryEffect,
    context: &mut NativeContext,
) -> crate::representation::BatchRecoveryBoundaryEffectResult {
    let Operation::Commit(message) = &context.operation else {
        return context.fail(RepoError::UnsupportedRepository, R::CommitNotPublished);
    };
    let Some(files) = context.files.as_ref() else {
        return context.fail(RepoError::UnsupportedRepository, R::CommitNotPublished);
    };
    let Some(control) = context.control.as_ref() else {
        return context.fail(RepoError::UnsupportedRepository, R::CommitNotPublished);
    };
    match crate::git::commit(&context.root, files, control, &context.policy, message) {
        Ok(oid) => {
            context.output = Some(Output::Commit(oid));
            R::CommitPublished
        }
        Err(error @ RepoError::PublicationUnknown) => {
            context.fail(error, R::CommitPublicationUnknown)
        }
        Err(error) => context.fail(error, R::CommitNotPublished),
    }
}
