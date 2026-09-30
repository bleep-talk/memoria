// RMS declared role: representation
// Module: batch-recovery-decisions
// Machine mode: stateful-transition-machine
// Fill this role body without changing semantics outside RMS spec apply or focused machine structure outside RMS machine apply.
use crate::{hash_content, parse_markdown, ContentHash, MemoryError, MemoryPath, Policy};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Mutation {
    Create {
        path: MemoryPath,
        content: String,
    },
    Replace {
        path: MemoryPath,
        content: String,
        expected: ContentHash,
    },
    Delete {
        path: MemoryPath,
        expected: ContentHash,
    },
}
impl Mutation {
    pub fn path(&self) -> &MemoryPath {
        match self {
            Self::Create { path, .. } | Self::Replace { path, .. } | Self::Delete { path, .. } => {
                path
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchRequest {
    id: String,
    operations: Vec<Mutation>,
    originals: BTreeMap<MemoryPath, Option<String>>,
    policy: Policy,
}
impl BatchRequest {
    pub fn new(
        id: String,
        operations: Vec<Mutation>,
        originals: BTreeMap<MemoryPath, Option<String>>,
        policy: Policy,
    ) -> Result<Self, MemoryError> {
        validate_id(&id)?;
        Ok(Self {
            id,
            operations,
            originals,
            policy,
        })
    }
    pub fn prepare(&self) -> Result<PreparedBatch, MemoryError> {
        if self.operations.is_empty() {
            return Err(MemoryError::EmptyBatch);
        }
        if self.operations.len() > self.policy.max_operations() {
            return Err(MemoryError::ParserLimitExceeded);
        }
        let mut changes = Vec::new();
        for operation in &self.operations {
            let path = operation.path();
            self.policy.check_write(path)?;
            let before = self
                .originals
                .get(path)
                .ok_or(MemoryError::InvalidRecoveryRecord)?
                .clone();
            let actual = before.as_ref().map(|v| hash_content(v.as_bytes()));
            let after = match operation {
                Mutation::Create { content, .. } => {
                    if actual.is_some() {
                        return Err(MemoryError::StaleWrite(path.to_string()));
                    }
                    Some(content.clone())
                }
                Mutation::Replace {
                    content, expected, ..
                } => {
                    if actual.as_ref() != Some(expected) {
                        return Err(MemoryError::StaleWrite(path.to_string()));
                    }
                    Some(content.clone())
                }
                Mutation::Delete { expected, .. } => {
                    if actual.as_ref() != Some(expected) {
                        return Err(MemoryError::StaleWrite(path.to_string()));
                    }
                    None
                }
            };
            changes.push(FileChange {
                path: path.clone(),
                before,
                after,
            });
        }
        PreparedBatch::new(self.id.clone(), changes, &self.policy)
    }
}

fn validate_id(id: &str) -> Result<(), MemoryError> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(MemoryError::InvalidRecoveryRecord);
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileChange {
    path: MemoryPath,
    before: Option<String>,
    after: Option<String>,
}
impl FileChange {
    pub fn new(path: MemoryPath, before: Option<String>, after: Option<String>) -> Self {
        Self {
            path,
            before,
            after,
        }
    }
    pub fn path(&self) -> &MemoryPath {
        &self.path
    }
    pub fn before(&self) -> Option<&str> {
        self.before.as_deref()
    }
    pub fn after(&self) -> Option<&str> {
        self.after.as_deref()
    }
    pub fn before_hash(&self) -> Option<ContentHash> {
        self.before.as_ref().map(|v| hash_content(v.as_bytes()))
    }
    pub fn after_hash(&self) -> Option<ContentHash> {
        self.after.as_ref().map(|v| hash_content(v.as_bytes()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PreparedBatch {
    id: String,
    digest: ContentHash,
    changes: Vec<FileChange>,
}
impl PreparedBatch {
    pub fn new(id: String, changes: Vec<FileChange>, policy: &Policy) -> Result<Self, MemoryError> {
        validate_id(&id)?;
        if changes.is_empty() {
            return Err(MemoryError::EmptyBatch);
        }
        if changes.len() > policy.max_operations() {
            return Err(MemoryError::ParserLimitExceeded);
        }
        let mut paths = BTreeSet::new();
        let mut size = 0usize;
        for change in &changes {
            policy.check_write(change.path())?;
            // ASCII aliases can address the same file on macOS.
            let key = change.path().as_str().to_ascii_lowercase();
            if paths.iter().any(|p: &String| {
                p == &key
                    || p.starts_with(&(key.clone() + "/"))
                    || key.starts_with(&(p.clone() + "/"))
            }) {
                return Err(MemoryError::DuplicateTarget(change.path().to_string()));
            }
            paths.insert(key);
            if change.before.is_none() && change.after.is_none() {
                return Err(MemoryError::InvalidRecoveryRecord);
            }
            for value in [&change.before, &change.after].into_iter().flatten() {
                if value.len() > policy.max_file_bytes() {
                    return Err(MemoryError::ParserLimitExceeded);
                }
                size = size
                    .checked_add(value.len())
                    .ok_or(MemoryError::ArithmeticOverflow)?;
            }
            if let Some(after) = &change.after {
                parse_markdown(after.as_bytes(), policy)?;
            }
        }
        if size > policy.max_batch_bytes() {
            return Err(MemoryError::ParserLimitExceeded);
        }
        let encoded =
            serde_json::to_vec(&(&id, &changes)).map_err(|_| MemoryError::InvalidRecoveryRecord)?;
        Ok(Self {
            id,
            digest: hash_content(&encoded),
            changes,
        })
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn digest(&self) -> &ContentHash {
        &self.digest
    }
    pub fn changes(&self) -> &[FileChange] {
        &self.changes
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "index", rename_all = "snake_case")]
pub enum PlanAction {
    Prepare,
    Apply(usize),
    MarkCompletion,
    Rollback(usize),
    Cleanup,
    Inspect,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchPlan {
    batch: PreparedBatch,
    action: PlanAction,
    sequence: u64,
}
impl BatchPlan {
    pub fn new(
        batch: PreparedBatch,
        action: PlanAction,
        sequence: u64,
    ) -> Result<Self, MemoryError> {
        if matches!(action,PlanAction::Apply(i)|PlanAction::Rollback(i) if i>=batch.changes.len()) {
            return Err(MemoryError::InvalidRecoveryRecord);
        }
        Ok(Self {
            batch,
            action,
            sequence,
        })
    }
    pub fn batch(&self) -> &PreparedBatch {
        &self.batch
    }
    pub fn action(&self) -> &PlanAction {
        &self.action
    }
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
}
