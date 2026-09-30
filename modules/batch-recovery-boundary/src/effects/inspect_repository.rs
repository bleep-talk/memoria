use crate::driver::NativeContext;
use crate::representation::BatchRecoveryBoundaryEffectResult as R;

pub fn execute(
    _effect: crate::representation::BatchRecoveryBoundaryEffect,
    context: &mut NativeContext,
) -> crate::representation::BatchRecoveryBoundaryEffectResult {
    let result = crate::git::execute_local_git(&context.root).and_then(|_| {
        context.control()?.ensure_repository_id()?;
        crate::git::recover_pending_commit(&context.root, context.control()?)
    });
    match result {
        Ok(_) => R::RepositoryObserved(true),
        Err(crate::repo::RepoError::UnsupportedRepository) => context.fail(
            crate::repo::RepoError::UnsupportedRepository,
            R::RepositoryObserved(false),
        ),
        Err(error) => context.fail(error, R::RepositoryInspectionError),
    }
}
