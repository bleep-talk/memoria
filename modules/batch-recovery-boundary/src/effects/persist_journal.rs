use crate::driver::NativeContext;
use crate::journal::Journal;
use crate::repo::RepoError;
use crate::representation::BatchRecoveryBoundaryEffectResult as R;

pub fn execute(
    _effect: crate::representation::BatchRecoveryBoundaryEffect,
    context: &mut NativeContext,
) -> crate::representation::BatchRecoveryBoundaryEffectResult {
    let result = (|| {
        let prepared = context.prepared.as_ref().ok_or(RepoError::InvalidJournal)?;
        let journal = Journal::new(prepared, context.control()?.repository_id()?);
        crate::journal::crash("journal_before_persist");
        journal.persist(context.control()?, &context.policy)?;
        Ok::<_, RepoError>(journal)
    })();
    match result {
        Ok(journal) => {
            context.journal = Some(journal);
            crate::journal::crash("journal_before_apply");
            R::JournalDurable
        }
        Err(error) => context.fail(error, R::JournalPersistenceUncertain),
    }
}
