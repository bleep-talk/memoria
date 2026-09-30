//! Memoria's native repository boundary.
mod driver;
mod fs;
mod git;
mod journal;
mod parser;
mod repo;
mod representation;
mod transition;
mod effects {
    pub mod acquire_lock;
    pub mod apply_filesystem;
    pub mod cleanup_journal;
    pub mod inspect_git;
    pub mod inspect_journal;
    pub mod inspect_repository;
    pub mod persist_completion;
    pub mod persist_journal;
    pub mod publish_commit;
    pub mod read_snapshot;
    pub mod reconcile_commit;
    pub mod release_lock;
    pub mod rollback_journal;
}

pub use crate::repo::{BatchReceipt, ExpectedContent, MemoryFile, MemoryRepo, RepoError};
pub use crate::representation::*;
pub use crate::transition::{
    replay_trace, transition, transition_record, BatchRecoveryBoundaryMachine,
    BatchRecoveryBoundarySourceProvenance, BatchRecoveryBoundaryTransition,
    BatchRecoveryBoundaryTransitionRecord,
};
pub use batch_recovery_decisions::{
    ContentHash, ContextOptions, MemoryContext, MemoryError, MemoryPath, Mutation, Policy,
    TreeEntry,
};

pub fn semantic_shape() -> &'static str {
    "boundary-adapter"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutation_needs_durable_journal_before_apply() {
        let begin = transition_record(
            initial_state(),
            BatchRecoveryBoundaryInput::Command(BatchRecoveryBoundaryCommand::Write),
        );
        assert_eq!(begin.state_after, BatchRecoveryBoundaryState::AcquiringLock);
        assert_eq!(
            begin.output.effects,
            vec![BatchRecoveryBoundaryEffect::AcquireRepoLock]
        );
        let premature = transition(
            BatchRecoveryBoundaryState::PersistingJournal,
            BatchRecoveryBoundaryInput::EffectResult(
                BatchRecoveryBoundaryEffectResult::FilesystemPlanApplied(false),
            ),
        );
        assert_eq!(
            premature.rejection,
            Some(BatchRecoveryBoundaryRejection::IllegalTransition)
        );
        assert!(premature.effects.is_empty());
        let durable = transition(
            BatchRecoveryBoundaryState::PersistingJournal,
            BatchRecoveryBoundaryInput::EffectResult(
                BatchRecoveryBoundaryEffectResult::JournalDurable,
            ),
        );
        assert_eq!(
            durable.next_state,
            BatchRecoveryBoundaryState::ApplyingFilesystem
        );
        assert_eq!(
            durable.effects,
            vec![BatchRecoveryBoundaryEffect::ApplyFilesystemPlan]
        );
    }
}
