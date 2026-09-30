// RMS declared role: representation
// Module: batch-recovery-decisions
// Machine mode: stateful-transition-machine
// Fill this role body without changing semantics outside RMS spec apply or focused machine structure outside RMS machine apply.
use crate::{BatchPlan, ContentHash, MemoryError, MemoryPath, PlanAction, PreparedBatch};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionMarker {
    Absent,
    DurableComplete,
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationResult {
    Confirmed,
    Failed,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryObservation {
    batch_id: String,
    digest: ContentHash,
    sequence: u64,
    action: PlanAction,
    marker: CompletionMarker,
    result: ObservationResult,
    contents: BTreeMap<MemoryPath, Option<ContentHash>>,
    inventory_valid: bool,
    cleaned: bool,
}
impl RecoveryObservation {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        batch_id: String,
        digest: ContentHash,
        sequence: u64,
        action: PlanAction,
        marker: CompletionMarker,
        result: ObservationResult,
        contents: BTreeMap<MemoryPath, Option<ContentHash>>,
        inventory_valid: bool,
        cleaned: bool,
    ) -> Self {
        Self {
            batch_id,
            digest,
            sequence,
            action,
            marker,
            result,
            contents,
            inventory_valid,
            cleaned,
        }
    }
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
    pub fn marker(&self) -> &CompletionMarker {
        &self.marker
    }
    pub fn result(&self) -> &ObservationResult {
        &self.result
    }
    pub fn cleaned(&self) -> bool {
        self.cleaned
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchOutcome {
    Applied,
    RolledBack,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettledBatch {
    batch_id: String,
    outcome: BatchOutcome,
    last: RecoveryObservation,
}
impl SettledBatch {
    pub fn new(
        batch_id: String,
        outcome: BatchOutcome,
        last: RecoveryObservation,
    ) -> Result<Self, MemoryError> {
        if batch_id != last.batch_id || !last.cleaned {
            return Err(MemoryError::InvalidRecoveryRecord);
        }
        Ok(Self {
            batch_id,
            outcome,
            last,
        })
    }
    pub fn id(&self) -> &str {
        &self.batch_id
    }
    pub fn outcome(&self) -> &BatchOutcome {
        &self.outcome
    }
    pub fn is_duplicate(&self, observation: &RecoveryObservation) -> bool {
        &self.last == observation
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingBatch {
    batch: PreparedBatch,
    action: PlanAction,
    sequence: u64,
    last: Option<RecoveryObservation>,
    rolled_back: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecoveryDecision {
    Continue(PendingBatch, BatchPlan),
    Settled(SettledBatch),
    Duplicate(BatchPlan),
    Inspect(PendingBatch, BatchPlan),
}
impl PendingBatch {
    pub fn new(batch: PreparedBatch) -> Self {
        Self {
            batch,
            action: PlanAction::Prepare,
            sequence: 0,
            last: None,
            rolled_back: false,
        }
    }
    pub fn batch(&self) -> &PreparedBatch {
        &self.batch
    }
    pub fn plan(&self) -> Result<BatchPlan, MemoryError> {
        BatchPlan::new(self.batch.clone(), self.action.clone(), self.sequence)
    }

    fn validate_observation(&self, o: &RecoveryObservation) -> Result<(), MemoryError> {
        if o.batch_id != self.batch.id() || &o.digest != self.batch.digest() {
            return Err(MemoryError::ForeignBatch);
        }
        if !o.inventory_valid || o.contents.len() != self.batch.changes().len() {
            return Err(MemoryError::InvalidRecoveryRecord);
        }
        for change in self.batch.changes() {
            let current = o
                .contents
                .get(change.path())
                .ok_or(MemoryError::InvalidRecoveryRecord)?;
            if current != &change.before_hash() && current != &change.after_hash() {
                return Err(MemoryError::UnexpectedRecoveryContents(
                    change.path().to_string(),
                ));
            }
        }
        Ok(())
    }
    fn check_contents(&self, o: &RecoveryObservation, after: bool) -> Result<(), MemoryError> {
        for change in self.batch.changes() {
            let expected = if after {
                change.after_hash()
            } else {
                change.before_hash()
            };
            if o.contents.get(change.path()) != Some(&expected) {
                return Err(MemoryError::UnexpectedRecoveryContents(
                    change.path().to_string(),
                ));
            }
        }
        Ok(())
    }
    fn next(
        &self,
        o: RecoveryObservation,
        action: PlanAction,
        rolled_back: bool,
    ) -> Result<(Self, BatchPlan), MemoryError> {
        let next = Self {
            batch: self.batch.clone(),
            action,
            sequence: o
                .sequence
                .checked_add(1)
                .ok_or(MemoryError::ArithmeticOverflow)?,
            last: Some(o),
            rolled_back,
        };
        let plan = next.plan()?;
        Ok((next, plan))
    }
    pub fn resume(&self, o: RecoveryObservation) -> Result<RecoveryDecision, MemoryError> {
        self.validate_observation(&o)?;
        if o.marker == CompletionMarker::Unknown {
            let (next, plan) = self.next(o, PlanAction::Inspect, self.rolled_back)?;
            return Ok(RecoveryDecision::Inspect(next, plan));
        }
        let (action, rolled_back) = if o.marker == CompletionMarker::DurableComplete {
            self.check_contents(&o, true)?;
            (PlanAction::Cleanup, false)
        } else {
            (PlanAction::Rollback(0), true)
        };
        let (next, plan) = self.next(o, action, rolled_back)?;
        Ok(RecoveryDecision::Continue(next, plan))
    }
    pub fn observe(&self, o: RecoveryObservation) -> Result<RecoveryDecision, MemoryError> {
        if let Some(last) = &self.last {
            if last == &o {
                return Ok(RecoveryDecision::Duplicate(self.plan()?));
            }
            if o.sequence <= last.sequence {
                return Err(MemoryError::StaleObservation);
            }
        }
        if o.sequence != self.sequence || o.action != self.action {
            return Err(MemoryError::ConflictingObservation);
        }
        self.validate_observation(&o)?;
        if o.result != ObservationResult::Confirmed
            || o.marker == CompletionMarker::Unknown
            || self.action == PlanAction::Inspect
        {
            return self.resume(o);
        }
        let (action, rolled_back) = match self.action {
            PlanAction::Prepare => {
                self.check_contents(&o, false)?;
                (PlanAction::Apply(0), false)
            }
            PlanAction::Apply(index) => {
                for change in self.batch.changes().iter().take(
                    index
                        .checked_add(1)
                        .ok_or(MemoryError::ArithmeticOverflow)?,
                ) {
                    if o.contents.get(change.path()) != Some(&change.after_hash()) {
                        return Err(MemoryError::UnexpectedRecoveryContents(
                            change.path().to_string(),
                        ));
                    }
                }
                if index
                    .checked_add(1)
                    .ok_or(MemoryError::ArithmeticOverflow)?
                    == self.batch.changes().len()
                {
                    (PlanAction::MarkCompletion, false)
                } else {
                    (
                        PlanAction::Apply(
                            index
                                .checked_add(1)
                                .ok_or(MemoryError::ArithmeticOverflow)?,
                        ),
                        false,
                    )
                }
            }
            PlanAction::MarkCompletion => {
                if o.marker != CompletionMarker::DurableComplete {
                    return Err(MemoryError::ConflictingObservation);
                }
                self.check_contents(&o, true)?;
                (PlanAction::Cleanup, false)
            }
            PlanAction::Rollback(index) => {
                for change in self.batch.changes().iter().take(
                    index
                        .checked_add(1)
                        .ok_or(MemoryError::ArithmeticOverflow)?,
                ) {
                    if o.contents.get(change.path()) != Some(&change.before_hash()) {
                        return Err(MemoryError::UnexpectedRecoveryContents(
                            change.path().to_string(),
                        ));
                    }
                }
                if index
                    .checked_add(1)
                    .ok_or(MemoryError::ArithmeticOverflow)?
                    == self.batch.changes().len()
                {
                    (PlanAction::Cleanup, true)
                } else {
                    (
                        PlanAction::Rollback(
                            index
                                .checked_add(1)
                                .ok_or(MemoryError::ArithmeticOverflow)?,
                        ),
                        true,
                    )
                }
            }
            PlanAction::Cleanup => {
                self.check_contents(&o, !self.rolled_back)?;
                if !o.cleaned {
                    return Err(MemoryError::ConflictingObservation);
                }
                let outcome = if self.rolled_back {
                    BatchOutcome::RolledBack
                } else {
                    BatchOutcome::Applied
                };
                return Ok(RecoveryDecision::Settled(SettledBatch::new(
                    self.batch.id().to_owned(),
                    outcome,
                    o,
                )?));
            }
            PlanAction::Inspect => return self.resume(o),
        };
        let (next, plan) = self.next(o, action, rolled_back)?;
        Ok(RecoveryDecision::Continue(next, plan))
    }
}
