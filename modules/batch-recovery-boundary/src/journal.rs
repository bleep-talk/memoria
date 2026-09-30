//! Bounded, durable recovery record. Before/after content is staged in the journal.
use crate::fs::Confined;
use crate::repo::RepoError;
use crate::representation::JournalFact;
use batch_recovery_decisions::{
    hash_content, CompletionMarker, FileChange, ObservationResult, PendingBatch, PlanAction,
    Policy, PreparedBatch, RecoveryDecision, RecoveryObservation,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const NAME: &str = "journal.json";
const SCHEMA: u32 = 1;

fn journal_limit(policy: &Policy) -> Result<usize, RepoError> {
    policy
        .max_batch_bytes()
        .checked_mul(2)
        .and_then(|n| n.checked_add(1024 * 1024))
        .ok_or(RepoError::LimitExceeded)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum JournalPhase {
    Prepared,
    Completed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Journal {
    schema: u32,
    repository_id: String,
    id: String,
    digest: batch_recovery_decisions::ContentHash,
    phase: JournalPhase,
    changes: Vec<FileChange>,
}

impl Journal {
    pub(crate) fn new(batch: &PreparedBatch, repository_id: String) -> Self {
        Self {
            schema: SCHEMA,
            repository_id,
            id: batch.id().to_owned(),
            digest: batch.digest().clone(),
            phase: JournalPhase::Prepared,
            changes: batch.changes().to_vec(),
        }
    }

    pub(crate) fn load(control: &Confined, policy: &Policy) -> Result<Option<Self>, RepoError> {
        let limit = journal_limit(policy)?;
        let Some(bytes) = control.read(NAME, limit)? else {
            return Ok(None);
        };
        let journal: Self =
            serde_json::from_slice(&bytes).map_err(|_| RepoError::InvalidJournal)?;
        if journal.schema != SCHEMA {
            return Err(RepoError::InvalidJournal);
        }
        if journal.repository_id != control.repository_id()? {
            return Err(RepoError::InvalidJournal);
        }
        let batch = PreparedBatch::new(journal.id.clone(), journal.changes.clone(), policy)
            .map_err(|_| RepoError::InvalidJournal)?;
        if batch.digest() != &journal.digest {
            return Err(RepoError::InvalidJournal);
        }
        Ok(Some(journal))
    }

    pub(crate) fn persist(&self, control: &Confined, policy: &Policy) -> Result<(), RepoError> {
        let bytes = serde_json::to_vec(self).map_err(|_| RepoError::InvalidJournal)?;
        if bytes.len() > journal_limit(policy)? {
            return Err(RepoError::LimitExceeded);
        }
        control.replace(NAME, &bytes)
    }

    pub(crate) fn complete(
        &mut self,
        control: &Confined,
        policy: &Policy,
    ) -> Result<(), RepoError> {
        self.phase = JournalPhase::Completed;
        self.persist(control, policy)
    }

    pub(crate) fn cleanup(&self, control: &Confined) -> Result<(), RepoError> {
        crash("cleanup_before_delete");
        control.delete(NAME)?;
        crash("cleanup_after_delete");
        Ok(())
    }

    pub(crate) fn changes(&self) -> &[FileChange] {
        &self.changes
    }

    pub(crate) fn recovery_fact(
        &self,
        files: &Confined,
        policy: &Policy,
    ) -> Result<JournalFact, RepoError> {
        match self.recovery_decision(files, policy)? {
            RecoveryDecision::Continue(_, plan)
                if matches!(plan.action(), PlanAction::Rollback(_)) =>
            {
                Ok(JournalFact::Incomplete)
            }
            RecoveryDecision::Continue(_, plan) if matches!(plan.action(), PlanAction::Cleanup) => {
                Ok(JournalFact::Complete)
            }
            _ => Err(RepoError::InvalidJournal),
        }
    }

    fn recovery_decision(
        &self,
        files: &Confined,
        policy: &Policy,
    ) -> Result<RecoveryDecision, RepoError> {
        let batch = PreparedBatch::new(self.id.clone(), self.changes.clone(), policy)
            .map_err(|_| RepoError::InvalidJournal)?;
        let mut contents = BTreeMap::new();
        for change in &self.changes {
            let bytes = files.read(change.path().as_str(), policy.max_file_bytes())?;
            contents.insert(
                change.path().clone(),
                bytes.as_ref().map(|data| hash_content(data)),
            );
        }
        let marker = match self.phase {
            JournalPhase::Prepared => CompletionMarker::Absent,
            JournalPhase::Completed => CompletionMarker::DurableComplete,
        };
        let observation = RecoveryObservation::new(
            self.id.clone(),
            self.digest.clone(),
            0,
            PlanAction::Inspect,
            marker,
            ObservationResult::Confirmed,
            contents,
            true,
            false,
        );
        PendingBatch::new(batch)
            .resume(observation)
            .map_err(|error| match error {
                batch_recovery_decisions::MemoryError::UnexpectedRecoveryContents(path) => {
                    RepoError::RecoveryConflict(path)
                }
                _ => RepoError::InvalidJournal,
            })
    }

    pub(crate) fn rollback_change(
        &self,
        files: &Confined,
        policy: &Policy,
        index: usize,
    ) -> Result<(), RepoError> {
        let change = self.changes.get(index).ok_or(RepoError::InvalidJournal)?;
        let current = files.read(change.path().as_str(), policy.max_file_bytes())?;
        let current_hash = current.as_ref().map(|data| hash_content(data));
        if current_hash != change.before_hash() && current_hash != change.after_hash() {
            return Err(RepoError::RecoveryConflict(change.path().to_string()));
        }
        match change.before() {
            Some(before) if current_hash != change.before_hash() => {
                files.replace(change.path().as_str(), before.as_bytes())
            }
            None if current.is_some() => files.delete(change.path().as_str()),
            _ => Ok(()),
        }
    }

    pub(crate) fn apply_change(
        &self,
        files: &Confined,
        policy: &Policy,
        index: usize,
    ) -> Result<(), RepoError> {
        let change = self.changes.get(index).ok_or(RepoError::InvalidJournal)?;
        let current = files.read(change.path().as_str(), policy.max_file_bytes())?;
        if current.as_ref().map(|data| hash_content(data)) != change.before_hash() {
            return Err(RepoError::RecoveryConflict(change.path().to_string()));
        }
        match change.after() {
            Some(after) => files.replace(change.path().as_str(), after.as_bytes()),
            None => files.delete(change.path().as_str()),
        }
    }
}

pub(crate) fn crash(point: &str) {
    if std::env::var("MEMORIA_TEST_CRASH").ok().as_deref() == Some(point) {
        std::process::abort();
    }
}
