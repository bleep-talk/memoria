//! Public repository facade and typed operation errors.
use crate::driver::{Operation, Output};
use crate::fs::open_directory;
use batch_recovery_decisions::{
    ContentHash, ContextOptions, MemoryContext, MemoryError, MemoryPath, Mutation, Policy,
    TreeEntry,
};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum RepoError {
    Memory(MemoryError),
    Io(std::io::Error),
    Sys(rustix::io::Errno),
    Git(git2::Error),
    UnsupportedRepository,
    UnsafePath(String),
    InvalidRequest(String),
    NotFound(String),
    AlreadyExists(String),
    ExpectedHashMismatch(String),
    LimitExceeded,
    InvalidJournal,
    RecoveryConflict(String),
    RecoveryRequired(String),
    StagedChanges,
    HeadMismatch,
    PublicationConflict,
    PublicationUnknown,
}

impl RepoError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Memory(error) => match error {
                MemoryError::ProtectedMutation(_) => "policy_violation",
                MemoryError::StaleWrite(_) => "expected_hash_mismatch",
                MemoryError::ContextTooLarge { .. } => "context_too_large",
                MemoryError::UnexpectedRecoveryContents(_) => "recovery_conflict",
                _ => "invalid_request",
            },
            Self::Io(_) => "io_error",
            Self::Sys(error)
                if *error == rustix::io::Errno::LOOP || *error == rustix::io::Errno::NOTDIR =>
            {
                "unsafe_path"
            }
            Self::Sys(_) => "io_error",
            Self::Git(_) => "git_error",
            Self::UnsupportedRepository => "unsupported_repository",
            Self::UnsafePath(_) => "unsafe_path",
            Self::InvalidRequest(_) => "invalid_request",
            Self::NotFound(_) => "not_found",
            Self::AlreadyExists(_) => "already_exists",
            Self::ExpectedHashMismatch(_) => "expected_hash_mismatch",
            Self::LimitExceeded => "journal_limit_exceeded",
            Self::InvalidJournal => "invalid_journal",
            Self::RecoveryConflict(_) => "recovery_conflict",
            Self::RecoveryRequired(_) => "recovery_required",
            Self::StagedChanges => "staged_changes",
            Self::HeadMismatch => "head_mismatch",
            Self::PublicationConflict => "publication_conflict",
            Self::PublicationUnknown => "publication_unknown",
        }
    }
}
impl fmt::Display for RepoError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Memory(error) => write!(output, "{error}"),
            Self::Io(error) => write!(output, "{error}"),
            Self::Sys(error) => write!(output, "{error}"),
            Self::Git(error) => write!(output, "{error}"),
            Self::UnsupportedRepository => write!(output, "unsupported repository"),
            Self::UnsafePath(path) => write!(output, "unsafe path: {path}"),
            Self::InvalidRequest(message) => write!(output, "invalid request: {message}"),
            Self::NotFound(path) => write!(output, "memory not found: {path}"),
            Self::AlreadyExists(path) => write!(output, "memory already exists: {path}"),
            Self::ExpectedHashMismatch(path) => {
                write!(output, "expected content does not match: {path}")
            }
            Self::LimitExceeded => write!(output, "resource limit exceeded"),
            Self::InvalidJournal => write!(output, "invalid recovery journal"),
            Self::RecoveryConflict(path) => {
                write!(output, "recovery conflicts with external edit: {path}")
            }
            Self::RecoveryRequired(id) => write!(output, "batch {id} requires recovery"),
            Self::StagedChanges => write!(output, "Git index has staged changes"),
            Self::HeadMismatch => write!(output, "Git HEAD changed"),
            Self::PublicationConflict => {
                write!(output, "Git publication conflicts with another revision")
            }
            Self::PublicationUnknown => write!(output, "Git publication outcome is unknown"),
        }
    }
}
impl std::error::Error for RepoError {}
impl From<MemoryError> for RepoError {
    fn from(value: MemoryError) -> Self {
        Self::Memory(value)
    }
}
impl From<git2::Error> for RepoError {
    fn from(value: git2::Error) -> Self {
        Self::Git(value)
    }
}

pub struct MemoryRepo {
    pub(crate) root: PathBuf,
    pub(crate) policy: Policy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExpectedContent {
    Absent,
    Hash(ContentHash),
}

#[derive(Clone, Debug)]
pub struct MemoryFile {
    content: String,
    hash: ContentHash,
    metadata: BTreeMap<String, serde_json::Value>,
}
impl MemoryFile {
    pub(crate) fn from_document(document: batch_recovery_decisions::MarkdownDocument) -> Self {
        Self {
            content: document.raw().to_owned(),
            hash: document.hash().clone(),
            metadata: document.metadata().clone(),
        }
    }
    pub fn content(&self) -> &str {
        &self.content
    }
    pub fn hash(&self) -> &ContentHash {
        &self.hash
    }
    pub fn metadata(&self) -> &BTreeMap<String, serde_json::Value> {
        &self.metadata
    }
}

#[derive(Clone, Debug)]
pub struct BatchReceipt {
    operation_id: String,
}
impl BatchReceipt {
    pub(crate) fn new(operation_id: String) -> Self {
        Self { operation_id }
    }
    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }
}

impl MemoryRepo {
    pub fn init(path: impl AsRef<Path>) -> Result<Self, RepoError> {
        Self::init_with_policy(path, Policy::default())
    }

    pub fn init_with_policy(path: impl AsRef<Path>, policy: Policy) -> Result<Self, RepoError> {
        let path = path.as_ref();
        open_directory(path, true)?;
        let _git = git2::Repository::init(path)?;
        let repo = Self {
            root: path.to_path_buf(),
            policy,
        };
        match repo.dispatch(Operation::Init)? {
            Output::Ready => Ok(repo),
            _ => unreachable!(),
        }
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, RepoError> {
        Self::open_with_policy(path, Policy::default())
    }

    pub fn open_with_policy(path: impl AsRef<Path>, policy: Policy) -> Result<Self, RepoError> {
        let repo = Self {
            root: path.as_ref().to_path_buf(),
            policy,
        };
        match repo.dispatch(Operation::Open)? {
            Output::Ready => Ok(repo),
            _ => unreachable!(),
        }
    }

    pub(crate) fn dispatch(&self, operation: Operation) -> Result<Output, RepoError> {
        crate::driver::run_operation(self, operation)
    }

    pub fn read(&self, raw: &str) -> Result<MemoryFile, RepoError> {
        let path = crate::parser::parse_request(raw, &self.policy, false)?;
        match self.dispatch(Operation::Read(path))? {
            Output::File(file) => Ok(file),
            _ => unreachable!(),
        }
    }

    pub fn exists(&self, raw: &str) -> Result<bool, RepoError> {
        let path = crate::parser::parse_request(raw, &self.policy, false)?;
        match self.dispatch(Operation::Exists(path))? {
            Output::Bool(value) => Ok(value),
            _ => unreachable!(),
        }
    }

    pub fn read_metadata(
        &self,
        raw: &str,
    ) -> Result<BTreeMap<String, serde_json::Value>, RepoError> {
        let path = crate::parser::parse_request(raw, &self.policy, false)?;
        match self.dispatch(Operation::ReadMetadata(path))? {
            Output::Metadata(value) => Ok(value),
            _ => unreachable!(),
        }
    }

    pub fn write(
        &self,
        raw: &str,
        content: &str,
        expected: ExpectedContent,
    ) -> Result<BatchReceipt, RepoError> {
        let path = crate::parser::parse_request(raw, &self.policy, true)?;
        let operation = match expected {
            ExpectedContent::Absent => Mutation::Create {
                path,
                content: content.to_owned(),
            },
            ExpectedContent::Hash(expected) => Mutation::Replace {
                path,
                content: content.to_owned(),
                expected,
            },
        };
        match self.dispatch(Operation::Mutation(
            crate::representation::BatchRecoveryBoundaryCommand::Write,
            vec![operation],
        ))? {
            Output::Receipt(receipt) => Ok(receipt),
            _ => unreachable!(),
        }
    }

    pub fn delete(&self, raw: &str, expected: ContentHash) -> Result<BatchReceipt, RepoError> {
        let path = crate::parser::parse_request(raw, &self.policy, true)?;
        match self.dispatch(Operation::Mutation(
            crate::representation::BatchRecoveryBoundaryCommand::Delete,
            vec![Mutation::Delete { path, expected }],
        ))? {
            Output::Receipt(receipt) => Ok(receipt),
            _ => unreachable!(),
        }
    }

    pub fn apply(&self, operations: Vec<Mutation>) -> Result<BatchReceipt, RepoError> {
        match self.dispatch(Operation::Mutation(
            crate::representation::BatchRecoveryBoundaryCommand::Apply,
            operations,
        ))? {
            Output::Receipt(receipt) => Ok(receipt),
            _ => unreachable!(),
        }
    }

    pub fn list(&self) -> Result<Vec<MemoryPath>, RepoError> {
        match self.dispatch(Operation::List)? {
            Output::Paths(paths) => Ok(paths),
            _ => unreachable!(),
        }
    }

    pub fn build_context(&self, options: ContextOptions) -> Result<MemoryContext, RepoError> {
        match self.dispatch(Operation::BuildContext(options))? {
            Output::Context(value) => Ok(value),
            _ => unreachable!(),
        }
    }

    pub fn tree(&self, descriptions: bool) -> Result<Vec<TreeEntry>, RepoError> {
        match self.dispatch(Operation::Tree(descriptions))? {
            Output::Tree(tree) => Ok(tree),
            _ => unreachable!(),
        }
    }

    pub fn status(&self) -> Result<Vec<String>, RepoError> {
        match self.dispatch(Operation::Status)? {
            Output::Status(value) => Ok(value),
            _ => unreachable!(),
        }
    }

    pub fn diff(&self) -> Result<String, RepoError> {
        match self.dispatch(Operation::Diff)? {
            Output::Diff(value) => Ok(value),
            _ => unreachable!(),
        }
    }

    pub fn log(&self) -> Result<Vec<String>, RepoError> {
        match self.dispatch(Operation::Log)? {
            Output::Log(value) => Ok(value),
            _ => unreachable!(),
        }
    }

    pub fn commit(&self, message: &str) -> Result<String, RepoError> {
        match self.dispatch(Operation::Commit(message.to_owned()))? {
            Output::Commit(value) => Ok(value),
            _ => unreachable!(),
        }
    }
}
