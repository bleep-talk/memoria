//! Closed lifecycle vocabulary. Executors supply observations; transitions decide phases.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BatchRecoveryBoundaryState {
    Ready,
    AcquiringLock,
    InspectingRepository,
    InspectingJournal,
    RollingBack,
    CleaningRecoveredJournal,
    ReadingSnapshot,
    PersistingJournal,
    ApplyingFilesystem,
    PersistingCompletion,
    CleaningCompletedJournal,
    InspectingGit,
    PublishingCommit,
    ReconcilingCommit,
    ReleasingSuccess,
    ReleasingError,
}

pub fn initial_state() -> BatchRecoveryBoundaryState {
    BatchRecoveryBoundaryState::Ready
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BatchRecoveryBoundaryCommand {
    Init,
    Open,
    Read,
    Search,
    Write,
    Delete,
    Exists,
    List,
    ReadMetadata,
    Apply,
    BuildContext,
    Tree,
    Status,
    Diff,
    Commit,
    Log,
    InvalidRequestInput,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchRecoveryBoundaryCommandEnvelope {
    pub command_id: String,
    pub target_machine: String,
    pub correlation_id: String,
    pub causation_id: String,
    pub idempotency_key: String,
    pub command: BatchRecoveryBoundaryCommand,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BatchRecoveryBoundaryEvent {}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchRecoveryBoundaryEventEnvelope {
    pub event_id: String,
    pub source_machine: String,
    pub correlation_id: String,
    pub causation_id: String,
    pub sequence: u64,
    pub schema_version: u32,
    pub occurred_at: String,
    pub event: BatchRecoveryBoundaryEvent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BatchRecoveryBoundaryEffect {
    AcquireRepoLock,
    InspectRepository,
    InspectJournal,
    RollbackJournal,
    CleanupJournal,
    ReadSnapshot,
    PersistJournal,
    ApplyFilesystemPlan,
    PersistCompletion,
    InspectGit,
    PublishCommit,
    ReconcileCommit,
    ReleaseRepoLock,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchRecoveryBoundaryEffectEnvelope {
    pub effect_id: String,
    pub requester: String,
    pub correlation_id: String,
    pub causation_id: String,
    pub idempotency_key: String,
    pub effect: BatchRecoveryBoundaryEffect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JournalFact {
    Empty,
    Incomplete,
    Complete,
    Conflict,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotFact {
    ReadReady,
    MutationReady,
    GitNeeded,
    Rejected,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GitFact {
    QueryReady,
    CommitReady,
    Rejected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BatchRecoveryBoundaryEffectResult {
    LockAcquired,
    LockUnavailable,
    RepositoryObserved(bool),
    RepositoryInspectionError,
    JournalObserved(JournalFact),
    JournalInspectionError,
    RollbackDurable(bool),
    RollbackConflict,
    RollbackIoError,
    JournalCleanupDurable,
    JournalCleanupIoError,
    SnapshotObserved(SnapshotFact),
    SnapshotIoError,
    JournalDurable,
    JournalPersistenceUncertain,
    FilesystemPlanApplied(bool),
    FilesystemApplyInterrupted,
    CompletionDurable,
    CompletionPersistenceUncertain,
    GitObserved(GitFact),
    GitInspectionError,
    CommitPublished,
    CommitNotPublished,
    CommitPublicationUnknown,
    PublicationReconciled(bool),
    PublicationStillUnknown,
    LockReleased,
    LockReleaseError,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchRecoveryBoundaryEffectResultEnvelope {
    pub effect_id: String,
    pub requester: String,
    pub correlation_id: String,
    pub causation_id: String,
    pub status: String,
    pub result: BatchRecoveryBoundaryEffectResult,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BatchRecoveryBoundaryInput {
    Command(BatchRecoveryBoundaryCommand),
    EffectResult(BatchRecoveryBoundaryEffectResult),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BatchRecoveryBoundaryReply {
    OperationResult,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BatchRecoveryBoundaryRejection {
    InvalidRequest,
    OperationError,
    IllegalTransition,
}
