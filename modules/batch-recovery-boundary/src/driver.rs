//! The boundary driver owns the transition/effect/result loop.
use crate::fs::Confined;
use crate::journal::Journal;
use crate::repo::{
    BatchReceipt, MemoryFile, MemoryRepo, RepoError, SearchMatch, SearchResult,
    MAX_SEARCH_LINE_BYTES,
};
use crate::representation::{
    BatchRecoveryBoundaryCommand as C, BatchRecoveryBoundaryEffect as E,
    BatchRecoveryBoundaryEffectResult as R, BatchRecoveryBoundaryInput as I,
    BatchRecoveryBoundaryState as S, GitFact, SnapshotFact,
};
use crate::transition::{transition_record, BatchRecoveryBoundaryTransitionRecord};
use batch_recovery_decisions::{
    parse_markdown, BatchRequest, ContextOptions, MemoryContext, MemoryPath, Mutation, Policy,
    PreparedBatch, SnapshotEntry, TreeEntry,
};
use std::collections::BTreeMap;
use std::fs::File;
use std::path::PathBuf;

pub(crate) enum Operation {
    Init,
    Open,
    Read(MemoryPath),
    Search(String, usize),
    Exists(MemoryPath),
    ReadMetadata(MemoryPath),
    List,
    BuildContext(ContextOptions),
    Tree(bool),
    Mutation(C, Vec<Mutation>),
    Status,
    Diff,
    Commit(String),
    Log,
}
impl Operation {
    fn command(&self) -> C {
        match self {
            Self::Init => C::Init,
            Self::Open => C::Open,
            Self::Read(_) => C::Read,
            Self::Search(_, _) => C::Search,
            Self::Exists(_) => C::Exists,
            Self::ReadMetadata(_) => C::ReadMetadata,
            Self::List => C::List,
            Self::BuildContext(_) => C::BuildContext,
            Self::Tree(_) => C::Tree,
            Self::Mutation(command, _) => *command,
            Self::Status => C::Status,
            Self::Diff => C::Diff,
            Self::Commit(_) => C::Commit,
            Self::Log => C::Log,
        }
    }
}

pub(crate) enum Output {
    Ready,
    File(MemoryFile),
    Search(SearchResult),
    Bool(bool),
    Metadata(BTreeMap<String, serde_json::Value>),
    Paths(Vec<MemoryPath>),
    Context(MemoryContext),
    Tree(Vec<TreeEntry>),
    Receipt(BatchReceipt),
    Status(Vec<String>),
    Diff(String),
    Commit(String),
    Log(Vec<String>),
}

pub(crate) struct NativeContext {
    pub(crate) root: PathBuf,
    pub(crate) policy: Policy,
    pub(crate) operation: Operation,
    pub(crate) files: Option<Confined>,
    pub(crate) control: Option<Confined>,
    pub(crate) lock: Option<File>,
    pub(crate) journal: Option<Journal>,
    pub(crate) prepared: Option<PreparedBatch>,
    pub(crate) apply_index: usize,
    pub(crate) rollback_index: usize,
    pub(crate) output: Option<Output>,
    pub(crate) error: Option<RepoError>,
    pub(crate) failed_request: bool,
}

impl NativeContext {
    pub(crate) fn files(&self) -> Result<&Confined, RepoError> {
        self.files.as_ref().ok_or(RepoError::UnsupportedRepository)
    }
    pub(crate) fn control(&self) -> Result<&Confined, RepoError> {
        self.control
            .as_ref()
            .ok_or(RepoError::UnsupportedRepository)
    }
    pub(crate) fn journal(&self) -> Result<&Journal, RepoError> {
        self.journal.as_ref().ok_or(RepoError::InvalidJournal)
    }
    pub(crate) fn fail(&mut self, error: RepoError, result: R) -> R {
        self.error = Some(error);
        result
    }

    pub(crate) fn inspect_snapshot(&mut self) -> Result<SnapshotFact, RepoError> {
        if self.failed_request {
            return Ok(SnapshotFact::Rejected);
        }
        let files = self.files()?;
        let output = match &self.operation {
            Operation::Init | Operation::Open => {
                for root in self.policy.roots() {
                    files.ensure_dir(root)?;
                }
                Output::Ready
            }
            Operation::Read(path) | Operation::ReadMetadata(path) => {
                let bytes = files
                    .read(path.as_str(), self.policy.max_file_bytes())?
                    .ok_or_else(|| RepoError::NotFound(path.to_string()))?;
                let document = parse_markdown(&bytes, &self.policy)?;
                if matches!(self.operation, Operation::Read(_)) {
                    Output::File(MemoryFile::from_document(document))
                } else {
                    Output::Metadata(document.metadata().clone())
                }
            }
            Operation::Search(query, limit) => Output::Search(search_snapshot(
                files.list(&self.policy)?,
                query,
                *limit,
                &self.policy,
            )?),
            Operation::Exists(path) => Output::Bool(
                files
                    .read(path.as_str(), self.policy.max_file_bytes())?
                    .is_some(),
            ),
            Operation::List => {
                let mut paths = files
                    .list(&self.policy)?
                    .into_iter()
                    .filter_map(|entry| match entry {
                        SnapshotEntry::File(path, _) => Some(path),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                paths.sort();
                Output::Paths(paths)
            }
            Operation::BuildContext(options) => Output::Context(MemoryContext::new(
                &files.list(&self.policy)?,
                &self.policy,
                options.clone(),
            )?),
            Operation::Tree(descriptions) => {
                let rendered = batch_recovery_decisions::render_memory(
                    &batch_recovery_decisions::RenderRequest::Tree(
                        files.list(&self.policy)?,
                        self.policy.clone(),
                        *descriptions,
                    ),
                )?;
                let batch_recovery_decisions::RenderResult::Tree(tree) = rendered else {
                    unreachable!()
                };
                Output::Tree(tree)
            }
            Operation::Mutation(_, operations) => {
                let mut originals = BTreeMap::new();
                for operation in operations {
                    self.policy.check_write(operation.path())?;
                    let bytes =
                        files.read(operation.path().as_str(), self.policy.max_file_bytes())?;
                    let before = bytes
                        .map(String::from_utf8)
                        .transpose()
                        .map_err(|_| batch_recovery_decisions::MemoryError::InvalidUtf8)?;
                    originals.insert(operation.path().clone(), before);
                }
                let id = uuid::Uuid::new_v4().to_string();
                let prepared = BatchRequest::new(
                    id.clone(),
                    operations.clone(),
                    originals,
                    self.policy.clone(),
                )?
                .prepare()?;
                self.prepared = Some(prepared);
                self.output = Some(Output::Receipt(BatchReceipt::new(id)));
                return Ok(SnapshotFact::MutationReady);
            }
            Operation::Status | Operation::Diff | Operation::Commit(_) | Operation::Log => {
                return Ok(SnapshotFact::GitNeeded)
            }
        };
        self.output = Some(output);
        Ok(SnapshotFact::ReadReady)
    }

    pub(crate) fn inspect_git(&mut self) -> Result<GitFact, RepoError> {
        let output = match &self.operation {
            Operation::Status => Some(Output::Status(crate::git::status(
                &self.root,
                &self.policy,
            )?)),
            Operation::Diff => Some(Output::Diff(crate::git::diff(&self.root, &self.policy)?)),
            Operation::Log => Some(Output::Log(crate::git::log(&self.root, 100)?)),
            Operation::Commit(_) => {
                crate::git::preflight_commit(&self.root)?;
                return Ok(GitFact::CommitReady);
            }
            _ => return Err(RepoError::UnsupportedRepository),
        };
        self.output = output;
        Ok(GitFact::QueryReady)
    }
}

fn excerpt(line: &str, match_offset: usize) -> (String, bool) {
    if line.len() <= MAX_SEARCH_LINE_BYTES {
        return (line.to_owned(), false);
    }
    let desired = match_offset.saturating_sub(128);
    let start = line
        .char_indices()
        .find_map(|(offset, _)| (offset >= desired).then_some(offset))
        .unwrap_or(line.len());
    let mut end = start;
    for (offset, ch) in line[start..].char_indices() {
        if offset + ch.len_utf8() > MAX_SEARCH_LINE_BYTES {
            break;
        }
        end = start + offset + ch.len_utf8();
    }
    (line[start..end].to_owned(), true)
}

/// A pure projection over the confined, locked working-file snapshot.
fn search_snapshot(
    snapshot: Vec<SnapshotEntry>,
    query: &str,
    limit: usize,
    policy: &Policy,
) -> Result<SearchResult, RepoError> {
    let mut files = Vec::new();
    for entry in snapshot {
        match entry {
            SnapshotEntry::File(path, content) => files.push((path, content)),
            SnapshotEntry::Directory(_) => {}
            SnapshotEntry::Unsafe(path) => return Err(RepoError::UnsafePath(path.to_string())),
        }
    }
    files.sort_by(|a, b| a.0.as_str().as_bytes().cmp(b.0.as_str().as_bytes()));
    let mut matches = Vec::new();
    let mut limited = false;
    for (path, content) in files {
        let _ = parse_markdown(content.as_bytes(), policy).map_err(|error| {
            RepoError::InvalidRequest(format!("invalid Markdown at {path}: {error}"))
        })?;
        for (index, line) in content.lines().enumerate() {
            if let Some(offset) = line.find(query) {
                if matches.len() == limit {
                    limited = true;
                    continue;
                }
                let (text, truncated) = excerpt(line, offset);
                matches.push(SearchMatch {
                    path: path.as_str().to_owned(),
                    line: index + 1,
                    text,
                    truncated,
                });
            }
        }
    }
    Ok(SearchResult { matches, limited })
}

fn execute(context: &mut NativeContext, effect: E) -> R {
    match effect {
        E::AcquireRepoLock => crate::effects::acquire_lock::execute(effect, context),
        E::InspectRepository => crate::effects::inspect_repository::execute(effect, context),
        E::InspectJournal => crate::effects::inspect_journal::execute(effect, context),
        E::RollbackJournal => crate::effects::rollback_journal::execute(effect, context),
        E::CleanupJournal => crate::effects::cleanup_journal::execute(effect, context),
        E::ReadSnapshot => crate::effects::read_snapshot::execute(effect, context),
        E::PersistJournal => crate::effects::persist_journal::execute(effect, context),
        E::ApplyFilesystemPlan => crate::effects::apply_filesystem::execute(effect, context),
        E::PersistCompletion => crate::effects::persist_completion::execute(effect, context),
        E::InspectGit => crate::effects::inspect_git::execute(effect, context),
        E::PublishCommit => crate::effects::publish_commit::execute(effect, context),
        E::ReconcileCommit => crate::effects::reconcile_commit::execute(effect, context),
        E::ReleaseRepoLock => crate::effects::release_lock::execute(effect, context),
    }
}

/// Execute one public operation through the canonical machine.
pub(crate) fn run_operation(repo: &MemoryRepo, operation: Operation) -> Result<Output, RepoError> {
    let command = operation.command();
    let mut context = NativeContext {
        root: repo.root.clone(),
        policy: repo.policy.clone(),
        operation,
        files: None,
        control: None,
        lock: None,
        journal: None,
        prepared: None,
        apply_index: 0,
        rollback_index: 0,
        output: None,
        error: None,
        failed_request: false,
    };
    let records = drive(S::Ready, I::Command(command), &mut context);
    if records
        .last()
        .is_some_and(|record| record.output.reply.is_some())
    {
        context
            .output
            .take()
            .ok_or(RepoError::UnsupportedRepository)
    } else {
        Err(context
            .error
            .take()
            .unwrap_or(RepoError::UnsupportedRepository))
    }
}

/// Return every complete transition record from one request lifecycle.
pub(crate) fn drive(
    state: crate::representation::BatchRecoveryBoundaryState,
    input: crate::representation::BatchRecoveryBoundaryInput,
    context: &mut NativeContext,
) -> Vec<BatchRecoveryBoundaryTransitionRecord> {
    let mut state = state;
    let mut input = input;
    let mut records = Vec::new();
    for _ in 0..10000 {
        let record = transition_record(state, input);
        state = record.state_after;
        let terminal = record.output.reply.is_some() || record.output.rejection.is_some();
        let effect = record.output.effects.first().copied();
        records.push(record);
        if terminal {
            return records;
        }
        let Some(effect) = effect else {
            context.error = Some(RepoError::UnsupportedRepository);
            return records;
        };
        input = I::EffectResult(execute(context, effect));
    }
    context.error = Some(RepoError::LimitExceeded);
    records
}
