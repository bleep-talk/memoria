use crate::representation::{
    BatchRecoveryBoundaryCommand as C, BatchRecoveryBoundaryEffect as E,
    BatchRecoveryBoundaryEffectResult as R, BatchRecoveryBoundaryInput as I,
    BatchRecoveryBoundaryRejection as X, BatchRecoveryBoundaryReply as Y,
    BatchRecoveryBoundaryState as S, GitFact, JournalFact, SnapshotFact,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchRecoveryBoundaryTransition {
    pub next_state: S,
    pub events: Vec<crate::representation::BatchRecoveryBoundaryEvent>,
    pub commands: Vec<C>,
    pub effects: Vec<E>,
    pub reply: Option<Y>,
    pub rejection: Option<X>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchRecoveryBoundarySourceProvenance {
    pub file: &'static str,
    pub function: &'static str,
    pub branch: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchRecoveryBoundaryTransitionRecord {
    pub state_before: S,
    pub state_after: S,
    pub input: I,
    pub output: BatchRecoveryBoundaryTransition,
    pub source: BatchRecoveryBoundarySourceProvenance,
}

pub struct BatchRecoveryBoundaryMachine;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PendingCommitDecision {
    AbortedBeforePublication,
    Published,
    Conflict,
}

pub(crate) fn pending_commit_decision(
    expected: Option<&str>,
    intended: &str,
    observed: Option<&str>,
) -> PendingCommitDecision {
    if observed == Some(intended) {
        PendingCommitDecision::Published
    } else if observed == expected {
        PendingCommitDecision::AbortedBeforePublication
    } else {
        PendingCommitDecision::Conflict
    }
}

impl BatchRecoveryBoundaryMachine {
    pub fn transition(state: S, input: I) -> BatchRecoveryBoundaryTransition {
        transition(state, input)
    }
}

pub fn transition(
    state: crate::representation::BatchRecoveryBoundaryState,
    input: crate::representation::BatchRecoveryBoundaryInput,
) -> BatchRecoveryBoundaryTransition {
    transition_record(state, input).output
}

pub fn transition_record(
    state: crate::representation::BatchRecoveryBoundaryState,
    input: crate::representation::BatchRecoveryBoundaryInput,
) -> BatchRecoveryBoundaryTransitionRecord {
    use I::*;
    use R::*;
    let (next_state, effect, reply, rejection, branch) = match (state, input) {
        (S::Ready, Command(C::Init)) => (
            S::AcquiringLock,
            Some(E::AcquireRepoLock),
            None,
            None,
            "begin_init",
        ),
        (S::Ready, Command(C::Open)) => (
            S::AcquiringLock,
            Some(E::AcquireRepoLock),
            None,
            None,
            "begin_open",
        ),
        (S::Ready, Command(C::Read)) => (
            S::AcquiringLock,
            Some(E::AcquireRepoLock),
            None,
            None,
            "begin_read",
        ),
        (S::Ready, Command(C::Write)) => (
            S::AcquiringLock,
            Some(E::AcquireRepoLock),
            None,
            None,
            "begin_write",
        ),
        (S::Ready, Command(C::Delete)) => (
            S::AcquiringLock,
            Some(E::AcquireRepoLock),
            None,
            None,
            "begin_delete",
        ),
        (S::Ready, Command(C::Exists)) => (
            S::AcquiringLock,
            Some(E::AcquireRepoLock),
            None,
            None,
            "begin_exists",
        ),
        (S::Ready, Command(C::List)) => (
            S::AcquiringLock,
            Some(E::AcquireRepoLock),
            None,
            None,
            "begin_list",
        ),
        (S::Ready, Command(C::ReadMetadata)) => (
            S::AcquiringLock,
            Some(E::AcquireRepoLock),
            None,
            None,
            "begin_read_metadata",
        ),
        (S::Ready, Command(C::Apply)) => (
            S::AcquiringLock,
            Some(E::AcquireRepoLock),
            None,
            None,
            "begin_apply",
        ),
        (S::Ready, Command(C::BuildContext)) => (
            S::AcquiringLock,
            Some(E::AcquireRepoLock),
            None,
            None,
            "begin_build_context",
        ),
        (S::Ready, Command(C::Tree)) => (
            S::AcquiringLock,
            Some(E::AcquireRepoLock),
            None,
            None,
            "begin_tree",
        ),
        (S::Ready, Command(C::Status)) => (
            S::AcquiringLock,
            Some(E::AcquireRepoLock),
            None,
            None,
            "begin_status",
        ),
        (S::Ready, Command(C::Diff)) => (
            S::AcquiringLock,
            Some(E::AcquireRepoLock),
            None,
            None,
            "begin_diff",
        ),
        (S::Ready, Command(C::Commit)) => (
            S::AcquiringLock,
            Some(E::AcquireRepoLock),
            None,
            None,
            "begin_commit",
        ),
        (S::Ready, Command(C::Log)) => (
            S::AcquiringLock,
            Some(E::AcquireRepoLock),
            None,
            None,
            "begin_log",
        ),
        (S::Ready, Command(C::InvalidRequestInput)) => (
            S::Ready,
            None,
            None,
            Some(X::InvalidRequest),
            "invalid_request_without_effects",
        ),
        (S::AcquiringLock, EffectResult(LockAcquired)) => (
            S::InspectingRepository,
            Some(E::InspectRepository),
            None,
            None,
            "lock_acquired",
        ),
        (S::AcquiringLock, EffectResult(LockUnavailable)) => (
            S::Ready,
            None,
            None,
            Some(X::OperationError),
            "lock_unavailable",
        ),
        (S::InspectingRepository, EffectResult(RepositoryObserved(true))) => (
            S::InspectingJournal,
            Some(E::InspectJournal),
            None,
            None,
            "supported_repository_observed",
        ),
        (S::InspectingRepository, EffectResult(RepositoryObserved(false))) => (
            S::ReleasingError,
            Some(E::ReleaseRepoLock),
            None,
            None,
            "unsupported_repository_observed",
        ),
        (S::InspectingRepository, EffectResult(RepositoryInspectionError)) => (
            S::ReleasingError,
            Some(E::ReleaseRepoLock),
            None,
            None,
            "repository_inspection_error",
        ),
        (S::InspectingJournal, EffectResult(JournalObserved(JournalFact::Empty))) => (
            S::ReadingSnapshot,
            Some(E::ReadSnapshot),
            None,
            None,
            "provider_reports_no_recovery",
        ),
        (S::InspectingJournal, EffectResult(JournalObserved(JournalFact::Incomplete))) => (
            S::RollingBack,
            Some(E::RollbackJournal),
            None,
            None,
            "provider_requires_rollback",
        ),
        (S::InspectingJournal, EffectResult(JournalObserved(JournalFact::Complete))) => (
            S::CleaningRecoveredJournal,
            Some(E::CleanupJournal),
            None,
            None,
            "provider_requires_completed_cleanup",
        ),
        (S::InspectingJournal, EffectResult(JournalObserved(JournalFact::Conflict))) => (
            S::ReleasingError,
            Some(E::ReleaseRepoLock),
            None,
            None,
            "provider_reports_recovery_conflict_or_invalid_journal",
        ),
        (S::InspectingJournal, EffectResult(JournalInspectionError)) => (
            S::ReleasingError,
            Some(E::ReleaseRepoLock),
            None,
            None,
            "journal_inspection_error",
        ),
        (S::RollingBack, EffectResult(RollbackDurable(false))) => (
            S::CleaningRecoveredJournal,
            Some(E::CleanupJournal),
            None,
            None,
            "rollback_durable",
        ),
        (S::RollingBack, EffectResult(RollbackDurable(true))) => (
            S::RollingBack,
            Some(E::RollbackJournal),
            None,
            None,
            "provider_continues_rollback",
        ),
        (S::RollingBack, EffectResult(RollbackConflict)) => (
            S::ReleasingError,
            Some(E::ReleaseRepoLock),
            None,
            None,
            "rollback_external_edit_conflict",
        ),
        (S::RollingBack, EffectResult(RollbackIoError)) => (
            S::ReleasingError,
            Some(E::ReleaseRepoLock),
            None,
            None,
            "rollback_requires_retry",
        ),
        (S::CleaningRecoveredJournal, EffectResult(JournalCleanupDurable)) => (
            S::ReadingSnapshot,
            Some(E::ReadSnapshot),
            None,
            None,
            "recovered_journal_cleaned",
        ),
        (S::CleaningRecoveredJournal, EffectResult(JournalCleanupIoError)) => (
            S::ReleasingError,
            Some(E::ReleaseRepoLock),
            None,
            None,
            "recovered_cleanup_requires_retry",
        ),
        (S::ReadingSnapshot, EffectResult(SnapshotObserved(SnapshotFact::ReadReady))) => (
            S::ReleasingSuccess,
            Some(E::ReleaseRepoLock),
            None,
            None,
            "provider_returns_read_or_noop_result",
        ),
        (S::ReadingSnapshot, EffectResult(SnapshotObserved(SnapshotFact::MutationReady))) => (
            S::PersistingJournal,
            Some(E::PersistJournal),
            None,
            None,
            "provider_validates_bounded_mutation",
        ),
        (S::ReadingSnapshot, EffectResult(SnapshotObserved(SnapshotFact::GitNeeded))) => (
            S::InspectingGit,
            Some(E::InspectGit),
            None,
            None,
            "git_operation_requires_observation",
        ),
        (S::ReadingSnapshot, EffectResult(SnapshotObserved(SnapshotFact::Rejected))) => (
            S::ReleasingError,
            Some(E::ReleaseRepoLock),
            None,
            None,
            "provider_rejects_request_or_expected_hash",
        ),
        (S::ReadingSnapshot, EffectResult(SnapshotIoError)) => (
            S::ReleasingError,
            Some(E::ReleaseRepoLock),
            None,
            None,
            "snapshot_io_error",
        ),
        (S::PersistingJournal, EffectResult(JournalDurable)) => (
            S::ApplyingFilesystem,
            Some(E::ApplyFilesystemPlan),
            None,
            None,
            "durable_journal_allows_apply",
        ),
        (S::PersistingJournal, EffectResult(JournalPersistenceUncertain)) => (
            S::ReleasingError,
            Some(E::ReleaseRepoLock),
            None,
            None,
            "journal_durability_unknown_no_apply",
        ),
        (S::ApplyingFilesystem, EffectResult(FilesystemPlanApplied(false))) => (
            S::PersistingCompletion,
            Some(E::PersistCompletion),
            None,
            None,
            "filesystem_plan_applied",
        ),
        (S::ApplyingFilesystem, EffectResult(FilesystemPlanApplied(true))) => (
            S::ApplyingFilesystem,
            Some(E::ApplyFilesystemPlan),
            None,
            None,
            "provider_continues_apply",
        ),
        (S::ApplyingFilesystem, EffectResult(FilesystemApplyInterrupted)) => (
            S::InspectingJournal,
            Some(E::InspectJournal),
            None,
            None,
            "interrupted_apply_enters_recovery",
        ),
        (S::PersistingCompletion, EffectResult(CompletionDurable)) => (
            S::CleaningCompletedJournal,
            Some(E::CleanupJournal),
            None,
            None,
            "durable_completion_allows_cleanup",
        ),
        (S::PersistingCompletion, EffectResult(CompletionPersistenceUncertain)) => (
            S::ReleasingError,
            Some(E::ReleaseRepoLock),
            None,
            None,
            "completion_durability_unknown",
        ),
        (S::CleaningCompletedJournal, EffectResult(JournalCleanupDurable)) => (
            S::ReleasingSuccess,
            Some(E::ReleaseRepoLock),
            None,
            None,
            "completed_journal_cleaned",
        ),
        (S::CleaningCompletedJournal, EffectResult(JournalCleanupIoError)) => (
            S::ReleasingError,
            Some(E::ReleaseRepoLock),
            None,
            None,
            "completed_cleanup_requires_retry",
        ),
        (S::InspectingGit, EffectResult(GitObserved(GitFact::QueryReady))) => (
            S::ReleasingSuccess,
            Some(E::ReleaseRepoLock),
            None,
            None,
            "git_query_result_ready",
        ),
        (S::InspectingGit, EffectResult(GitObserved(GitFact::CommitReady))) => (
            S::PublishingCommit,
            Some(E::PublishCommit),
            None,
            None,
            "clean_index_and_expected_head_match",
        ),
        (S::InspectingGit, EffectResult(GitObserved(GitFact::Rejected))) => (
            S::ReleasingError,
            Some(E::ReleaseRepoLock),
            None,
            None,
            "staged_changes_or_head_mismatch",
        ),
        (S::InspectingGit, EffectResult(GitInspectionError)) => (
            S::ReleasingError,
            Some(E::ReleaseRepoLock),
            None,
            None,
            "git_inspection_error",
        ),
        (S::PublishingCommit, EffectResult(CommitPublished)) => (
            S::ReleasingSuccess,
            Some(E::ReleaseRepoLock),
            None,
            None,
            "fenced_commit_publication_confirmed",
        ),
        (S::PublishingCommit, EffectResult(CommitNotPublished)) => (
            S::ReleasingError,
            Some(E::ReleaseRepoLock),
            None,
            None,
            "fenced_commit_not_published",
        ),
        (S::PublishingCommit, EffectResult(CommitPublicationUnknown)) => (
            S::ReconcilingCommit,
            Some(E::ReconcileCommit),
            None,
            None,
            "uncertain_commit_requires_reconciliation",
        ),
        (S::ReconcilingCommit, EffectResult(PublicationReconciled(true))) => (
            S::ReleasingSuccess,
            Some(E::ReleaseRepoLock),
            None,
            None,
            "intended_commit_confirmed_at_target_ref",
        ),
        (S::ReconcilingCommit, EffectResult(PublicationReconciled(false))) => (
            S::ReleasingError,
            Some(E::ReleaseRepoLock),
            None,
            None,
            "intended_commit_absent_or_conflicting_ref",
        ),
        (S::ReconcilingCommit, EffectResult(PublicationStillUnknown)) => (
            S::ReleasingError,
            Some(E::ReleaseRepoLock),
            None,
            None,
            "publication_remains_unknown",
        ),
        (S::ReleasingSuccess, EffectResult(LockReleased)) => (
            S::Ready,
            None,
            Some(Y::OperationResult),
            None,
            "operation_result_after_resource_release",
        ),
        (S::ReleasingSuccess, EffectResult(LockReleaseError)) => (
            S::Ready,
            None,
            None,
            Some(X::OperationError),
            "success_path_release_error",
        ),
        (S::ReleasingError, EffectResult(LockReleased)) => (
            S::Ready,
            None,
            None,
            Some(X::OperationError),
            "operation_error_after_resource_release",
        ),
        (S::ReleasingError, EffectResult(LockReleaseError)) => (
            S::Ready,
            None,
            None,
            Some(X::OperationError),
            "operation_and_release_error",
        ),
        _ => (
            state,
            None,
            None,
            Some(X::IllegalTransition),
            "illegal_transition",
        ),
    };
    let output = BatchRecoveryBoundaryTransition {
        next_state,
        events: Vec::new(),
        commands: Vec::new(),
        effects: effect.into_iter().collect(),
        reply,
        rejection,
    };
    BatchRecoveryBoundaryTransitionRecord {
        state_before: state,
        state_after: next_state,
        input,
        output,
        source: BatchRecoveryBoundarySourceProvenance {
            file: "src/transition.rs",
            function: "transition_record",
            branch,
        },
    }
}

pub fn replay_trace(
    initial_state: S,
    inputs: impl IntoIterator<Item = I>,
) -> Vec<BatchRecoveryBoundaryTransitionRecord> {
    let mut state = initial_state;
    let mut records = Vec::new();
    for input in inputs {
        let record = transition_record(state, input);
        state = record.state_after;
        records.push(record);
    }
    records
}

#[cfg(test)]
mod pending_commit_tests {
    use super::*;

    #[test]
    fn observed_ref_decides_recovery_without_guessing() {
        assert_eq!(
            pending_commit_decision(Some("old"), "new", Some("old")),
            PendingCommitDecision::AbortedBeforePublication
        );
        assert_eq!(
            pending_commit_decision(Some("old"), "new", Some("new")),
            PendingCommitDecision::Published
        );
        assert_eq!(
            pending_commit_decision(Some("old"), "new", Some("other")),
            PendingCommitDecision::Conflict
        );
        assert_eq!(
            pending_commit_decision(None, "new", None),
            PendingCommitDecision::AbortedBeforePublication
        );
    }
}
