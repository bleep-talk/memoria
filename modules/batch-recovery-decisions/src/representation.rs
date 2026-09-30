use crate::{
    BatchPlan, BatchRequest, PendingBatch, RecoveryObservation, RenderRequest, RenderResult,
    SettledBatch,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(tag = "code", content = "detail", rename_all = "snake_case")]
pub enum MemoryError {
    #[error("invalid memory path: {0}")]
    InvalidPath(String),
    #[error("invalid policy")]
    InvalidPolicy,
    #[error("protected memory: {0}")]
    ProtectedMutation(String),
    #[error("invalid content hash")]
    InvalidHash,
    #[error("malformed frontmatter: {0}")]
    MalformedYaml(String),
    #[error("description must be a string")]
    InvalidDescription,
    #[error("memory must be UTF-8")]
    InvalidUtf8,
    #[error("memory resource limit exceeded")]
    ParserLimitExceeded,
    #[error("duplicate or conflicting target: {0}")]
    DuplicateTarget(String),
    #[error("duplicate snapshot path: {0}")]
    DuplicateSnapshotPath(String),
    #[error("stale content precondition: {0}")]
    StaleWrite(String),
    #[error("context requires {required} bytes; budget is {allowed}")]
    ContextTooLarge { required: usize, allowed: usize },
    #[error("unsafe filesystem entry: {0}")]
    UnsupportedEntryKind(String),
    #[error("batch must contain at least one operation")]
    EmptyBatch,
    #[error("recovery conflict: {0}")]
    UnexpectedRecoveryContents(String),
    #[error("invalid recovery record")]
    InvalidRecoveryRecord,
    #[error("stale recovery observation")]
    StaleObservation,
    #[error("conflicting recovery observation")]
    ConflictingObservation,
    #[error("observation belongs to another batch")]
    ForeignBatch,
    #[error("numeric limit exceeded")]
    ArithmeticOverflow,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BatchRecoveryWorkflowState {
    Ready,
    BatchPending(PendingBatch),
    Completed(SettledBatch),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BatchRecoveryWorkflowCommand {
    DecideMemory(RenderRequest),
    BeginBatch(BatchRequest),
    ObserveRecovery(RecoveryObservation),
    ResumeRecovery(PendingBatch, RecoveryObservation),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BatchRecoveryWorkflowInput {
    Command(BatchRecoveryWorkflowCommand),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BatchRecoveryWorkflowEvent {}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BatchRecoveryWorkflowReply {
    MemoryDecision(RenderResult),
    BatchPlan(BatchPlan),
    BatchSettled(SettledBatch),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BatchRecoveryWorkflowRejection {
    MemoryRejected(MemoryError),
    IllegalTransition,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchRecoveryWorkflowCommandEnvelope {
    pub command_id: String,
    pub target_machine: String,
    pub correlation_id: String,
    pub causation_id: String,
    pub idempotency_key: String,
    pub command: BatchRecoveryWorkflowCommand,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchRecoveryWorkflowEventEnvelope {
    pub event_id: String,
    pub source_machine: String,
    pub correlation_id: String,
    pub causation_id: String,
    pub sequence: u64,
    pub schema_version: u32,
    pub occurred_at: Option<String>,
    pub event: BatchRecoveryWorkflowEvent,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchRecoveryWorkflowTransition {
    pub next_state: BatchRecoveryWorkflowState,
    pub events: Vec<BatchRecoveryWorkflowEvent>,
    pub commands: Vec<BatchRecoveryWorkflowCommand>,
    pub effects: Vec<()>,
    pub reply: Option<BatchRecoveryWorkflowReply>,
    pub rejection: Option<BatchRecoveryWorkflowRejection>,
    pub case: &'static str,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchRecoveryWorkflowSourceProvenance {
    pub file: &'static str,
    pub function: &'static str,
    pub branch: &'static str,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchRecoveryWorkflowTransitionRecord {
    pub state_before: BatchRecoveryWorkflowState,
    pub input: BatchRecoveryWorkflowInput,
    pub state_after: BatchRecoveryWorkflowState,
    pub output: BatchRecoveryWorkflowTransition,
    pub source: BatchRecoveryWorkflowSourceProvenance,
}
pub fn initial_state() -> BatchRecoveryWorkflowState {
    BatchRecoveryWorkflowState::Ready
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchRecoveryWorkflowMachine;
