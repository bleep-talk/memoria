use memoria_memory::*;
use serde_json::{json, Value};

fn record_json(record: &BatchRecoveryBoundaryTransitionRecord, first: bool) -> Value {
    let variant = |value: &dyn std::fmt::Debug| json!({"name":format!("{value:?}").split('(').next().unwrap_or(""),"data":{}});
    json!({"scenario_start":first,"state_before":variant(&record.state_before),"state_after":variant(&record.state_after),"input":format!("{:?}",record.input),"output":{"next_state":variant(&record.output.next_state),"events":record.output.events.iter().map(|x|variant(x)).collect::<Vec<_>>(),"commands":record.output.commands.iter().map(|x|variant(x)).collect::<Vec<_>>(),"effects":record.output.effects.iter().map(|x|variant(x)).collect::<Vec<_>>(),"reply":record.output.reply.as_ref().map(|x|variant(x)),"rejection":record.output.rejection.as_ref().map(|x|variant(x))},"source":{"file":record.source.file,"function":record.source.function,"branch":record.source.branch}})
}

fn write_records(records: Vec<BatchRecoveryBoundaryTransitionRecord>, independent: bool) {
    let Ok(output) = std::env::var("RMS_TRACE_OUTPUT") else {
        return;
    };
    let encoded = records
        .iter()
        .enumerate()
        .map(|(i, record)| record_json(record, independent || i == 0))
        .collect::<Vec<_>>();
    let bundle = json!({"spec":"rms/trace-bundle/v0.1","machine":"BatchRecoveryBoundaryMachine","records":encoded});
    std::fs::write(output, serde_json::to_vec_pretty(&bundle).unwrap()).unwrap();
}

fn produce(inputs: Vec<BatchRecoveryBoundaryInput>) {
    write_records(replay_trace(initial_state(), inputs), false);
}

#[test]
fn produce_operation_traces() {
    use BatchRecoveryBoundaryCommand as C;
    use BatchRecoveryBoundaryEffectResult as R;
    use BatchRecoveryBoundaryState as S;
    let states = [
        S::Ready,
        S::AcquiringLock,
        S::InspectingRepository,
        S::InspectingJournal,
        S::RollingBack,
        S::CleaningRecoveredJournal,
        S::ReadingSnapshot,
        S::PersistingJournal,
        S::ApplyingFilesystem,
        S::PersistingCompletion,
        S::CleaningCompletedJournal,
        S::InspectingGit,
        S::PublishingCommit,
        S::ReconcilingCommit,
        S::ReleasingSuccess,
        S::ReleasingError,
    ];
    let commands = [
        C::Init,
        C::Open,
        C::Read,
        C::Search,
        C::Write,
        C::Delete,
        C::Exists,
        C::List,
        C::ReadMetadata,
        C::Apply,
        C::BuildContext,
        C::Tree,
        C::Status,
        C::Diff,
        C::Commit,
        C::Log,
        C::InvalidRequestInput,
    ];
    let facts = [
        R::LockAcquired,
        R::LockUnavailable,
        R::RepositoryObserved(true),
        R::RepositoryObserved(false),
        R::RepositoryInspectionError,
        R::JournalObserved(JournalFact::Empty),
        R::JournalObserved(JournalFact::Incomplete),
        R::JournalObserved(JournalFact::Complete),
        R::JournalObserved(JournalFact::Conflict),
        R::JournalInspectionError,
        R::RollbackDurable(false),
        R::RollbackDurable(true),
        R::RollbackConflict,
        R::RollbackIoError,
        R::JournalCleanupDurable,
        R::JournalCleanupIoError,
        R::SnapshotObserved(SnapshotFact::ReadReady),
        R::SnapshotObserved(SnapshotFact::MutationReady),
        R::SnapshotObserved(SnapshotFact::GitNeeded),
        R::SnapshotObserved(SnapshotFact::Rejected),
        R::SnapshotIoError,
        R::JournalDurable,
        R::JournalPersistenceUncertain,
        R::FilesystemPlanApplied(false),
        R::FilesystemPlanApplied(true),
        R::FilesystemApplyInterrupted,
        R::CompletionDurable,
        R::CompletionPersistenceUncertain,
        R::GitObserved(GitFact::QueryReady),
        R::GitObserved(GitFact::CommitReady),
        R::GitObserved(GitFact::Rejected),
        R::GitInspectionError,
        R::CommitPublished,
        R::CommitNotPublished,
        R::CommitPublicationUnknown,
        R::PublicationReconciled(true),
        R::PublicationReconciled(false),
        R::PublicationStillUnknown,
        R::LockReleased,
        R::LockReleaseError,
    ];
    let mut records = Vec::new();
    let mut cases = std::collections::BTreeSet::new();
    for state in states {
        for command in commands {
            let record = transition_record(state, BatchRecoveryBoundaryInput::Command(command));
            if record.source.branch != "illegal_transition" && cases.insert(record.source.branch) {
                records.push(record);
            }
        }
        for fact in facts {
            let record = transition_record(state, BatchRecoveryBoundaryInput::EffectResult(fact));
            if record.source.branch != "illegal_transition" && cases.insert(record.source.branch) {
                records.push(record);
            }
        }
    }
    write_records(records, true);
}

#[test]
fn produce_recovery_traces() {
    use BatchRecoveryBoundaryEffectResult as R;
    produce(vec![
        BatchRecoveryBoundaryInput::Command(BatchRecoveryBoundaryCommand::Open),
        BatchRecoveryBoundaryInput::EffectResult(R::LockAcquired),
        BatchRecoveryBoundaryInput::EffectResult(R::RepositoryObserved(true)),
        BatchRecoveryBoundaryInput::EffectResult(R::JournalObserved(JournalFact::Incomplete)),
        BatchRecoveryBoundaryInput::EffectResult(R::RollbackDurable(false)),
        BatchRecoveryBoundaryInput::EffectResult(R::JournalCleanupDurable),
        BatchRecoveryBoundaryInput::EffectResult(R::SnapshotObserved(SnapshotFact::ReadReady)),
        BatchRecoveryBoundaryInput::EffectResult(R::LockReleased),
    ]);
}

#[test]
fn produce_publication_traces() {
    use BatchRecoveryBoundaryEffectResult as R;
    produce(vec![
        BatchRecoveryBoundaryInput::Command(BatchRecoveryBoundaryCommand::Commit),
        BatchRecoveryBoundaryInput::EffectResult(R::LockAcquired),
        BatchRecoveryBoundaryInput::EffectResult(R::RepositoryObserved(true)),
        BatchRecoveryBoundaryInput::EffectResult(R::JournalObserved(JournalFact::Empty)),
        BatchRecoveryBoundaryInput::EffectResult(R::SnapshotObserved(SnapshotFact::GitNeeded)),
        BatchRecoveryBoundaryInput::EffectResult(R::GitObserved(GitFact::CommitReady)),
        BatchRecoveryBoundaryInput::EffectResult(R::CommitPublicationUnknown),
        BatchRecoveryBoundaryInput::EffectResult(R::PublicationReconciled(true)),
        BatchRecoveryBoundaryInput::EffectResult(R::LockReleased),
    ]);
}
