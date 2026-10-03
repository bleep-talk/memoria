use memoria_memory::*;
use serde_json::{json, Value};

fn name(value: &impl std::fmt::Debug) -> String {
    format!("{value:?}")
        .split(['(', '{'])
        .next()
        .unwrap_or("")
        .to_owned()
}
fn variant(value: &impl std::fmt::Debug) -> Value {
    json!({"name":name(value),"data":{}})
}

fn parse_state(raw: &str) -> BatchRecoveryBoundaryState {
    use BatchRecoveryBoundaryState::*;
    match raw {
        "Ready" => Ready,
        "AcquiringLock" => AcquiringLock,
        "InspectingRepository" => InspectingRepository,
        "InspectingJournal" => InspectingJournal,
        "RollingBack" => RollingBack,
        "CleaningRecoveredJournal" => CleaningRecoveredJournal,
        "ReadingSnapshot" => ReadingSnapshot,
        "PersistingJournal" => PersistingJournal,
        "ApplyingFilesystem" => ApplyingFilesystem,
        "PersistingCompletion" => PersistingCompletion,
        "CleaningCompletedJournal" => CleaningCompletedJournal,
        "InspectingGit" => InspectingGit,
        "PublishingCommit" => PublishingCommit,
        "ReconcilingCommit" => ReconcilingCommit,
        "ReleasingSuccess" => ReleasingSuccess,
        "ReleasingError" => ReleasingError,
        other => panic!("unknown state: {other}"),
    }
}

fn parse_input(value: &Value) -> BatchRecoveryBoundaryInput {
    use BatchRecoveryBoundaryCommand as C;
    use BatchRecoveryBoundaryEffectResult as R;
    let kind = value["kind"].as_str().expect("input kind");
    let name = value["name"].as_str().expect("input name");
    match kind {
        "command" => BatchRecoveryBoundaryInput::Command(match name {
            "Init" => C::Init,
            "Open" => C::Open,
            "Read" => C::Read,
            "Search" => C::Search,
            "Write" => C::Write,
            "Delete" => C::Delete,
            "Exists" => C::Exists,
            "List" => C::List,
            "ReadMetadata" => C::ReadMetadata,
            "Apply" => C::Apply,
            "BuildContext" => C::BuildContext,
            "Tree" => C::Tree,
            "Status" => C::Status,
            "Diff" => C::Diff,
            "Commit" => C::Commit,
            "Log" => C::Log,
            "InvalidRequestInput" => C::InvalidRequestInput,
            other => panic!("unknown command: {other}"),
        }),
        "effect-result" => BatchRecoveryBoundaryInput::EffectResult(match name {
            "LockAcquired" => R::LockAcquired,
            "LockUnavailable" => R::LockUnavailable,
            "RepositoryObserved" => {
                R::RepositoryObserved(value["data"]["supported"].as_bool().unwrap_or(true))
            }
            "RepositoryInspectionError" => R::RepositoryInspectionError,
            "JournalObserved" => {
                R::JournalObserved(match value["data"]["fact"].as_str().unwrap_or("Empty") {
                    "Empty" => JournalFact::Empty,
                    "Incomplete" => JournalFact::Incomplete,
                    "Complete" => JournalFact::Complete,
                    _ => JournalFact::Conflict,
                })
            }
            "JournalInspectionError" => R::JournalInspectionError,
            "RollbackDurable" => {
                R::RollbackDurable(value["data"]["more"].as_bool().unwrap_or(false))
            }
            "RollbackConflict" => R::RollbackConflict,
            "RollbackIoError" => R::RollbackIoError,
            "JournalCleanupDurable" => R::JournalCleanupDurable,
            "JournalCleanupIoError" => R::JournalCleanupIoError,
            "SnapshotObserved" => R::SnapshotObserved(
                match value["data"]["fact"].as_str().unwrap_or("ReadReady") {
                    "ReadReady" => SnapshotFact::ReadReady,
                    "MutationReady" => SnapshotFact::MutationReady,
                    "GitNeeded" => SnapshotFact::GitNeeded,
                    _ => SnapshotFact::Rejected,
                },
            ),
            "SnapshotIoError" => R::SnapshotIoError,
            "JournalDurable" => R::JournalDurable,
            "JournalPersistenceUncertain" => R::JournalPersistenceUncertain,
            "FilesystemPlanApplied" => {
                R::FilesystemPlanApplied(value["data"]["more"].as_bool().unwrap_or(false))
            }
            "FilesystemApplyInterrupted" => R::FilesystemApplyInterrupted,
            "CompletionDurable" => R::CompletionDurable,
            "CompletionPersistenceUncertain" => R::CompletionPersistenceUncertain,
            "GitObserved" => R::GitObserved(
                match value["data"]["fact"].as_str().unwrap_or("QueryReady") {
                    "QueryReady" => GitFact::QueryReady,
                    "CommitReady" => GitFact::CommitReady,
                    _ => GitFact::Rejected,
                },
            ),
            "GitInspectionError" => R::GitInspectionError,
            "CommitPublished" => R::CommitPublished,
            "CommitNotPublished" => R::CommitNotPublished,
            "CommitPublicationUnknown" => R::CommitPublicationUnknown,
            "PublicationReconciled" => {
                R::PublicationReconciled(value["data"]["published"].as_bool().unwrap_or(true))
            }
            "PublicationStillUnknown" => R::PublicationStillUnknown,
            "LockReleased" => R::LockReleased,
            "LockReleaseError" => R::LockReleaseError,
            other => panic!("unknown result: {other}"),
        }),
        other => panic!("unknown kind: {other}"),
    }
}

fn record_json(record: &BatchRecoveryBoundaryTransitionRecord, normalized: &Value) -> Value {
    json!({
        "scenario_start":false,
        "state_before":variant(&record.state_before),"state_after":variant(&record.state_after),
        "input":normalized,
        "output":{"next_state":variant(&record.output.next_state),
            "events":record.output.events.iter().map(variant).collect::<Vec<_>>(),
            "commands":record.output.commands.iter().map(variant).collect::<Vec<_>>(),
            "effects":record.output.effects.iter().map(variant).collect::<Vec<_>>(),
            "reply":record.output.reply.as_ref().map(variant),
            "rejection":record.output.rejection.as_ref().map(variant)},
        "source":{"file":record.source.file,"function":record.source.function,"branch":record.source.branch}
    })
}

#[test]
fn probe_machine() {
    let (Ok(request_path), Ok(output_path)) = (
        std::env::var("RMS_PROBE_REQUEST"),
        std::env::var("RMS_PROBE_OUTPUT"),
    ) else {
        return;
    };
    let request: Value = serde_json::from_slice(&std::fs::read(request_path).unwrap()).unwrap();
    let states = [
        "Ready",
        "AcquiringLock",
        "InspectingRepository",
        "InspectingJournal",
        "RollingBack",
        "CleaningRecoveredJournal",
        "ReadingSnapshot",
        "PersistingJournal",
        "ApplyingFilesystem",
        "PersistingCompletion",
        "CleaningCompletedJournal",
        "InspectingGit",
        "PublishingCommit",
        "ReconcilingCommit",
        "ReleasingSuccess",
        "ReleasingError",
    ];
    let commands = [
        "Init",
        "Open",
        "Read",
        "Search",
        "Write",
        "Delete",
        "Exists",
        "List",
        "ReadMetadata",
        "Apply",
        "BuildContext",
        "Tree",
        "Status",
        "Diff",
        "Commit",
        "Log",
        "InvalidRequestInput",
    ];
    let results = [
        "LockAcquired",
        "LockUnavailable",
        "RepositoryObserved",
        "RepositoryInspectionError",
        "JournalObserved",
        "JournalInspectionError",
        "RollbackDurable",
        "RollbackConflict",
        "RollbackIoError",
        "JournalCleanupDurable",
        "JournalCleanupIoError",
        "SnapshotObserved",
        "SnapshotIoError",
        "JournalDurable",
        "JournalPersistenceUncertain",
        "FilesystemPlanApplied",
        "FilesystemApplyInterrupted",
        "CompletionDurable",
        "CompletionPersistenceUncertain",
        "GitObserved",
        "GitInspectionError",
        "CommitPublished",
        "CommitNotPublished",
        "CommitPublicationUnknown",
        "PublicationReconciled",
        "PublicationStillUnknown",
        "LockReleased",
        "LockReleaseError",
    ];
    let output = match request["operation"].as_str() {
        Some("describe") => {
            json!({"spec":"rms/machine-probe-description/v0.1","machine":"BatchRecoveryBoundaryMachine","initial_state":variant(&initial_state()),"states":states.iter().map(|name|json!({"name":name,"data_schema":{"type":"object"},"examples":[{"name":name,"data":{}}]})).collect::<Vec<_>>(),"inputs":commands.iter().map(|name|json!({"kind":"command","name":name,"data_schema":{"type":"object"},"example":{"kind":"command","name":name,"data":{}}})).chain(results.iter().map(|name|json!({"kind":"effect-result","name":name,"data_schema":{"type":"object"},"example":{"kind":"effect-result","name":name,"data":{}}}))).collect::<Vec<_>>() })
        }
        Some("evaluate") => {
            json!({"spec":"rms/machine-probe-evaluation/v0.2","machine":"BatchRecoveryBoundaryMachine","results":request["cases"].as_array().unwrap().iter().map(|case|{let state=parse_state(case["state"]["name"].as_str().unwrap());let input=&case["input"];let record=transition_record(state,parse_input(input));json!({"id":case["id"],"record":record_json(&record,input)})}).collect::<Vec<_>>() })
        }
        _ => {
            let mut state = if request["start"] == "initial" {
                initial_state()
            } else {
                parse_state(request["start"]["name"].as_str().unwrap())
            };
            let mut records = request["steps"]
                .as_array()
                .unwrap()
                .iter()
                .map(|step| {
                    let input = &step["input"];
                    let record = transition_record(state, parse_input(input));
                    state = record.state_after;
                    record_json(&record, input)
                })
                .collect::<Vec<_>>();
            if let Some(first) = records.first_mut() {
                first["scenario_start"] = json!(true);
            }
            json!({"spec":"rms/trace-bundle/v0.1","machine":"BatchRecoveryBoundaryMachine","records":records})
        }
    };
    std::fs::write(output_path, serde_json::to_vec_pretty(&output).unwrap()).unwrap();
}
