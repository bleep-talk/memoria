//! Local Git history adapter. Commit objects are built without writing the caller's index.
use crate::fs::Confined;
use crate::repo::RepoError;
use crate::transition::{pending_commit_decision, PendingCommitDecision};
use batch_recovery_decisions::{MemoryPath, Policy, SnapshotEntry};
use git2::{
    DiffFormat, DiffOptions, IndexEntry, IndexTime, Oid, Repository, RepositoryState,
    StatusOptions, TreeWalkMode, TreeWalkResult,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

const INTENT: &str = "commit-intent.json";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CommitIntent {
    schema: u32,
    reference: String,
    expected: Option<String>,
    intended: String,
    tree: String,
}

impl CommitIntent {
    fn persist(&self, control: &Confined) -> Result<(), RepoError> {
        control.replace(
            INTENT,
            &serde_json::to_vec(self).map_err(|_| RepoError::InvalidJournal)?,
        )
    }
}

pub(crate) fn execute_local_git(root: &Path) -> Result<Repository, RepoError> {
    let repo = Repository::open(root)?;
    if repo.is_bare() || repo.is_worktree() || repo.state() != RepositoryState::Clean {
        return Err(RepoError::UnsupportedRepository);
    }
    if repo.path().canonicalize().map_err(RepoError::Io)?
        != root.join(".git").canonicalize().map_err(RepoError::Io)?
    {
        return Err(RepoError::UnsupportedRepository);
    }
    if std::fs::symlink_metadata(root.join(".gitmodules")).is_ok() {
        return Err(RepoError::UnsupportedRepository);
    }
    if repo.index()?.iter().any(|entry| entry.mode == 0o160000) {
        return Err(RepoError::UnsupportedRepository);
    }
    match repo.head() {
        Ok(head) => {
            let tree = head.peel_to_commit()?.tree()?;
            let mut has_gitlink = false;
            let result = tree.walk(TreeWalkMode::PreOrder, |_prefix, entry| {
                if entry.filemode() == 0o160000 {
                    has_gitlink = true;
                    TreeWalkResult::Abort
                } else {
                    TreeWalkResult::Ok
                }
            });
            if has_gitlink {
                return Err(RepoError::UnsupportedRepository);
            }
            result?;
        }
        Err(error) if error.code() == git2::ErrorCode::UnbornBranch => {}
        Err(error) => return Err(error.into()),
    }
    Ok(repo)
}

fn managed(path: &str, policy: &Policy) -> bool {
    MemoryPath::new(path).is_ok_and(|path| policy.check_read(&path).is_ok())
}

pub(crate) fn status(root: &Path, policy: &Policy) -> Result<Vec<String>, RepoError> {
    let repo = execute_local_git(root)?;
    let mut options = StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false);
    let mut paths = Vec::new();
    for entry in repo.statuses(Some(&mut options))?.iter() {
        if let Some(path) = entry.path() {
            if managed(path, policy) {
                paths.push(format!("{}:{}", entry.status().bits(), path));
            }
        }
    }
    paths.sort();
    Ok(paths)
}

pub(crate) fn diff(root: &Path, policy: &Policy) -> Result<String, RepoError> {
    let repo = execute_local_git(root)?;
    let head_tree = match repo.head() {
        Ok(head) => Some(head.peel_to_commit()?.tree()?),
        Err(error) if error.code() == git2::ErrorCode::UnbornBranch => None,
        Err(error) => return Err(error.into()),
    };
    let mut options = DiffOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .show_untracked_content(true);
    for root in policy.roots() {
        options.pathspec(root);
    }
    let diff = repo.diff_tree_to_workdir_with_index(head_tree.as_ref(), Some(&mut options))?;
    let mut output = Vec::new();
    diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
        output.extend_from_slice(line.content());
        true
    })?;
    String::from_utf8(output)
        .map_err(|_| RepoError::UnsafePath("Git diff contains non-UTF-8 content".into()))
}

pub(crate) fn log(root: &Path, max: usize) -> Result<Vec<String>, RepoError> {
    let repo = execute_local_git(root)?;
    let head = match repo.head() {
        Ok(head) => head,
        Err(error) if error.code() == git2::ErrorCode::UnbornBranch => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let oid = head.target().ok_or(RepoError::UnsupportedRepository)?;
    let mut walk = repo.revwalk()?;
    walk.push(oid)?;
    let mut result = Vec::new();
    for oid in walk.take(max) {
        let commit = repo.find_commit(oid?)?;
        result.push(format!(
            "{} {}",
            commit.id(),
            commit.summary().unwrap_or("")
        ));
    }
    Ok(result)
}

fn current_head(repo: &Repository) -> Result<(String, Option<Oid>), RepoError> {
    let reference = repo.find_reference("HEAD")?;
    let target = reference
        .symbolic_target()
        .ok_or(RepoError::UnsupportedRepository)?
        .to_owned();
    let oid = match repo.find_reference(&target) {
        Ok(reference) => reference.target(),
        Err(error) if error.code() == git2::ErrorCode::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    Ok((target, oid))
}

fn staged_work(repo: &Repository, head: Option<Oid>) -> Result<bool, RepoError> {
    let index = repo.index()?;
    if index.has_conflicts() {
        return Ok(true);
    }
    let tree = match head {
        Some(oid) => Some(repo.find_commit(oid)?.tree()?),
        None => None,
    };
    let diff = repo.diff_tree_to_index(tree.as_ref(), Some(&index), None)?;
    Ok(diff.deltas().len() != 0)
}

pub(crate) fn preflight_commit(root: &Path) -> Result<(), RepoError> {
    let repo = execute_local_git(root)?;
    let (_, head) = current_head(&repo)?;
    if staged_work(&repo, head)? {
        return Err(RepoError::StagedChanges);
    }
    Ok(())
}

fn sync_index(repo: &Repository, tree: &git2::Tree<'_>) -> Result<(), RepoError> {
    let mut persisted_index = repo.index().map_err(|_| RepoError::PublicationUnknown)?;
    persisted_index
        .read_tree(tree)
        .map_err(|_| RepoError::PublicationUnknown)?;
    persisted_index
        .write()
        .map_err(|_| RepoError::PublicationUnknown)
}

/// Reconcile an interrupted local publication before the next operation runs.
/// The intent is durable before the reference update. An unrelated reference or
/// staged-index edit preserves the intent as evidence.
pub(crate) fn recover_pending_commit(root: &Path, control: &Confined) -> Result<(), RepoError> {
    let Some(bytes) = control.read(INTENT, 4096)? else {
        return Ok(());
    };
    let intent: CommitIntent =
        serde_json::from_slice(&bytes).map_err(|_| RepoError::InvalidJournal)?;
    if intent.schema != 1 {
        return Err(RepoError::InvalidJournal);
    }
    let expected = intent
        .expected
        .as_deref()
        .map(Oid::from_str)
        .transpose()
        .map_err(|_| RepoError::InvalidJournal)?;
    let intended = Oid::from_str(&intent.intended).map_err(|_| RepoError::InvalidJournal)?;
    let tree_id = Oid::from_str(&intent.tree).map_err(|_| RepoError::InvalidJournal)?;
    let repo = execute_local_git(root)?;
    let commit = repo
        .find_commit(intended)
        .map_err(|_| RepoError::InvalidJournal)?;
    if commit.tree_id() != tree_id
        || commit.parent_ids().collect::<Vec<_>>() != expected.into_iter().collect::<Vec<_>>()
    {
        return Err(RepoError::InvalidJournal);
    }
    let (reference, observed) = current_head(&repo)?;
    if reference != intent.reference {
        return Err(RepoError::PublicationConflict);
    }
    match pending_commit_decision(
        intent.expected.as_deref(),
        &intent.intended,
        observed.as_ref().map(|oid| oid.to_string()).as_deref(),
    ) {
        PendingCommitDecision::AbortedBeforePublication => {
            control.delete(INTENT)?;
            Ok(())
        }
        PendingCommitDecision::Published => {
            if staged_work(&repo, expected)? && staged_work(&repo, Some(intended))? {
                return Err(RepoError::StagedChanges);
            }
            sync_index(&repo, &repo.find_tree(tree_id)?)?;
            control.delete(INTENT)?;
            Ok(())
        }
        PendingCommitDecision::Conflict => Err(RepoError::PublicationConflict),
    }
}

pub(crate) fn reconcile_commit(
    root: &Path,
    control: &Confined,
) -> Result<Option<String>, RepoError> {
    let Some(bytes) = control.read(INTENT, 4096)? else {
        return Ok(None);
    };
    let intent: CommitIntent =
        serde_json::from_slice(&bytes).map_err(|_| RepoError::InvalidJournal)?;
    recover_pending_commit(root, control)?;
    let repo = execute_local_git(root)?;
    let (_, observed) = current_head(&repo)?;
    Ok(
        (observed.map(|oid| oid.to_string()) == Some(intent.intended.clone()))
            .then_some(intent.intended),
    )
}

pub(crate) fn commit(
    root: &Path,
    files: &Confined,
    control: &Confined,
    policy: &Policy,
    message: &str,
) -> Result<String, RepoError> {
    if message.trim().is_empty() || message.len() > 65536 {
        return Err(RepoError::UnsafePath("commit message".into()));
    }
    let repo = execute_local_git(root)?;
    let (reference_name, expected) = current_head(&repo)?;
    if staged_work(&repo, expected)? {
        return Err(RepoError::StagedChanges);
    }
    let mut index = git2::Index::new()?;
    let parent = expected.map(|oid| repo.find_commit(oid)).transpose()?;
    if let Some(parent) = &parent {
        index.read_tree(&parent.tree()?)?;
    }
    let obsolete = index
        .iter()
        .filter_map(|entry| {
            let path = std::str::from_utf8(&entry.path).ok()?;
            managed(path, policy).then(|| path.to_owned())
        })
        .collect::<Vec<_>>();
    for path in obsolete {
        index.remove_path(Path::new(&path))?;
    }
    for entry in files.list(policy)? {
        let SnapshotEntry::File(path, content) = entry else {
            continue;
        };
        let oid = repo.blob(content.as_bytes())?;
        index.add(&IndexEntry {
            ctime: IndexTime::new(0, 0),
            mtime: IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            file_size: content
                .len()
                .try_into()
                .map_err(|_| RepoError::LimitExceeded)?,
            id: oid,
            flags: 0,
            flags_extended: 0,
            path: path.as_str().as_bytes().to_vec(),
        })?;
    }
    if parent.is_none() && index.len() == 0 {
        return Err(RepoError::AlreadyExists("no managed changes".into()));
    }
    let tree_id = index.write_tree_to(&repo)?;
    let tree = repo.find_tree(tree_id)?;
    if parent
        .as_ref()
        .is_some_and(|parent| parent.tree_id() == tree_id)
    {
        return Err(RepoError::AlreadyExists("no managed changes".into()));
    }
    let signature = repo.signature()?;
    let parents: Vec<_> = parent.iter().collect();
    let intended = repo.commit(None, &signature, &signature, message, &tree, &parents)?;
    let (_, observed) = current_head(&repo)?;
    if observed != expected {
        return Err(RepoError::HeadMismatch);
    }
    CommitIntent {
        schema: 1,
        reference: reference_name.clone(),
        expected: expected.map(|oid| oid.to_string()),
        intended: intended.to_string(),
        tree: tree_id.to_string(),
    }
    .persist(control)?;
    crate::journal::crash("commit_after_intent");
    let publication = match expected {
        Some(old) => {
            repo.reference_matching(&reference_name, intended, true, old, "memoria commit")
        }
        None => repo.reference(&reference_name, intended, false, "memoria initial commit"),
    };
    match publication {
        Ok(_) => {
            crate::journal::crash("commit_after_ref");
            sync_index(&repo, &tree)?;
            control.delete(INTENT)?;
            Ok(intended.to_string())
        }
        Err(_) => {
            let (_, observed) = current_head(&repo)?;
            if observed == Some(intended) {
                sync_index(&repo, &tree)?;
                control.delete(INTENT)?;
                Ok(intended.to_string())
            } else if observed == expected {
                control.delete(INTENT)?;
                Err(RepoError::HeadMismatch)
            } else {
                Err(RepoError::PublicationConflict)
            }
        }
    }
}
