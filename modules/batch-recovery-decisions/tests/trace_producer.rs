use batch_recovery_decisions::*;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

fn variant(value: &impl std::fmt::Debug) -> String {
    format!("{value:?}")
        .split([' ', '{', '('])
        .next()
        .unwrap()
        .to_owned()
}

fn observation(
    batch: &PreparedBatch,
    sequence: u64,
    action: PlanAction,
    marker: CompletionMarker,
    contents: BTreeMap<MemoryPath, Option<ContentHash>>,
    cleaned: bool,
) -> RecoveryObservation {
    RecoveryObservation::new(
        batch.id().to_owned(),
        batch.digest().clone(),
        sequence,
        action,
        marker,
        ObservationResult::Confirmed,
        contents,
        true,
        cleaned,
    )
}

#[test]
fn produce_memoria_decision_trace() {
    let path = MemoryPath::new("knowledge/example.md").unwrap();
    let request = BatchRequest::new(
        "trace-batch".into(),
        vec![Mutation::Create {
            path: path.clone(),
            content: "example".into(),
        }],
        BTreeMap::from([(path.clone(), None)]),
        Policy::default(),
    )
    .unwrap();
    let invalid = BatchRequest::new(
        "empty-batch".into(),
        vec![],
        BTreeMap::new(),
        Policy::default(),
    )
    .unwrap();
    let batch = request.prepare().unwrap();
    let pending = PendingBatch::new(batch.clone());
    let before = BTreeMap::from([(path.clone(), None)]);
    let after = BTreeMap::from([(path, batch.changes()[0].after_hash())]);
    let prepared = observation(
        &batch,
        0,
        PlanAction::Prepare,
        CompletionMarker::Absent,
        before.clone(),
        false,
    );
    let unknown = observation(
        &batch,
        0,
        PlanAction::Prepare,
        CompletionMarker::Unknown,
        before,
        false,
    );
    let complete = observation(
        &batch,
        0,
        PlanAction::Prepare,
        CompletionMarker::DurableComplete,
        after.clone(),
        false,
    );
    let cleanup = observation(
        &batch,
        1,
        PlanAction::Cleanup,
        CompletionMarker::DurableComplete,
        after,
        true,
    );
    let prepared_pending = match pending.observe(prepared.clone()).unwrap() {
        RecoveryDecision::Continue(next, _) => next,
        _ => panic!("expected apply plan"),
    };
    let inspection_pending = match pending.observe(unknown.clone()).unwrap() {
        RecoveryDecision::Inspect(next, _) => next,
        _ => panic!("expected inspection plan"),
    };
    let cleanup_pending = match pending.resume(complete.clone()).unwrap() {
        RecoveryDecision::Continue(next, _) => next,
        _ => panic!("expected cleanup plan"),
    };
    let settled =
        SettledBatch::new("trace-batch".into(), BatchOutcome::Applied, cleanup.clone()).unwrap();
    let foreign = RecoveryObservation::new(
        "other-batch".into(),
        batch.digest().clone(),
        0,
        PlanAction::Prepare,
        CompletionMarker::Absent,
        ObservationResult::Confirmed,
        BTreeMap::new(),
        true,
        false,
    );
    let foreign_pending = PendingBatch::new(
        BatchRequest::new(
            "other-batch".into(),
            vec![Mutation::Create {
                path: MemoryPath::new("knowledge/other.md").unwrap(),
                content: "other".into(),
            }],
            BTreeMap::from([(MemoryPath::new("knowledge/other.md").unwrap(), None)]),
            Policy::default(),
        )
        .unwrap()
        .prepare()
        .unwrap(),
    );

    let states = vec![
        BatchRecoveryWorkflowState::Ready,
        BatchRecoveryWorkflowState::BatchPending(pending.clone()),
        BatchRecoveryWorkflowState::BatchPending(prepared_pending),
        BatchRecoveryWorkflowState::BatchPending(inspection_pending),
        BatchRecoveryWorkflowState::BatchPending(cleanup_pending),
        BatchRecoveryWorkflowState::Completed(settled),
    ];
    let commands = vec![
        BatchRecoveryWorkflowCommand::DecideMemory(RenderRequest::Context(
            vec![],
            Policy::default(),
            ContextOptions::new(0),
        )),
        BatchRecoveryWorkflowCommand::DecideMemory(RenderRequest::Context(
            vec![],
            Policy::default(),
            ContextOptions::new(32768),
        )),
        BatchRecoveryWorkflowCommand::BeginBatch(request),
        BatchRecoveryWorkflowCommand::BeginBatch(invalid),
        BatchRecoveryWorkflowCommand::ObserveRecovery(prepared.clone()),
        BatchRecoveryWorkflowCommand::ObserveRecovery(unknown.clone()),
        BatchRecoveryWorkflowCommand::ObserveRecovery(complete.clone()),
        BatchRecoveryWorkflowCommand::ObserveRecovery(cleanup),
        BatchRecoveryWorkflowCommand::ObserveRecovery(foreign.clone()),
        BatchRecoveryWorkflowCommand::ResumeRecovery(pending.clone(), prepared),
        BatchRecoveryWorkflowCommand::ResumeRecovery(pending.clone(), unknown),
        BatchRecoveryWorkflowCommand::ResumeRecovery(pending, complete),
        BatchRecoveryWorkflowCommand::ResumeRecovery(foreign_pending, foreign),
    ];
    let mut records = Vec::new();
    let mut cases = BTreeSet::new();
    for state in states {
        for command in &commands {
            let record = transition_record(
                state.clone(),
                BatchRecoveryWorkflowInput::Command(command.clone()),
            );
            if cases.insert(record.source.branch) {
                records.push(record);
            }
        }
    }
    assert!(records.iter().any(|record| record.output.reply.is_some()));
    assert!(records
        .iter()
        .any(|record| record.output.rejection.is_some()));
    if let Ok(output) = std::env::var("RMS_TRACE_OUTPUT") {
        let rows = records.iter().map(|record| {
            let BatchRecoveryWorkflowInput::Command(command) = &record.input;
            json!({
            "scenario_start":true,
            "state_before":variant(&record.state_before),
            "state_after":variant(&record.state_after),
            "input":{"kind":"command","name":variant(command),"data":{}},
            "output":{"next_state":variant(&record.state_after),"events":[],"commands":[],"effects":[],
                "reply":record.output.reply.as_ref().map(variant),
                "rejection":record.output.rejection.as_ref().map(variant)},
            "source":{"file":record.source.file,"function":record.source.function,"branch":record.source.branch},
            })
        }).collect::<Vec<Value>>();
        std::fs::write(output, serde_json::to_vec_pretty(&json!({
            "spec":"rms/trace-bundle/v0.1","machine":"BatchRecoveryWorkflowMachine","records":rows,
        })).unwrap()).unwrap();
    }
}
