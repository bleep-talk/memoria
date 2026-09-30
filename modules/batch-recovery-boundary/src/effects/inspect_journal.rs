use crate::driver::NativeContext;
use crate::journal::Journal;
use crate::repo::RepoError;
use crate::representation::{BatchRecoveryBoundaryEffectResult as R, JournalFact};

pub fn execute(
    _effect: crate::representation::BatchRecoveryBoundaryEffect,
    context: &mut NativeContext,
) -> crate::representation::BatchRecoveryBoundaryEffectResult {
    let result = (|| {
        let journal = Journal::load(context.control()?, &context.policy)?;
        let fact = match &journal {
            None => JournalFact::Empty,
            Some(journal) => journal.recovery_fact(context.files()?, &context.policy)?,
        };
        Ok::<_, RepoError>((journal, fact))
    })();
    match result {
        Ok((journal, fact)) => {
            context.journal = journal;
            context.rollback_index = 0;
            R::JournalObserved(fact)
        }
        Err(error @ RepoError::RecoveryConflict(_)) | Err(error @ RepoError::InvalidJournal) => {
            context.fail(error, R::JournalObserved(JournalFact::Conflict))
        }
        Err(error) => context.fail(error, R::JournalInspectionError),
    }
}
