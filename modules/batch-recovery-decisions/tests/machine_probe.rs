use batch_recovery_decisions::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn variant(name: &str) -> Value {
    json!({"name":name,"data":{}})
}

fn state_name(state: &BatchRecoveryWorkflowState) -> &'static str {
    match state {
        BatchRecoveryWorkflowState::Ready => "Ready",
        BatchRecoveryWorkflowState::BatchPending(_) => "BatchPending",
        BatchRecoveryWorkflowState::Completed(_) => "Completed",
    }
}

fn fixture() -> (BatchRequest, PendingBatch, RecoveryObservation) {
    let path = MemoryPath::new("knowledge/probe.md").unwrap();
    let policy = Policy::default();
    let request = BatchRequest::new(
        "probe".into(),
        vec![Mutation::Create {
            path: path.clone(),
            content: "# Probe\n".into(),
        }],
        BTreeMap::from([(path, None)]),
        policy,
    )
    .unwrap();
    let pending = PendingBatch::new(request.prepare().unwrap());
    let observation = RecoveryObservation::new(
        "probe".into(),
        pending.batch().digest().clone(),
        0,
        PlanAction::Prepare,
        CompletionMarker::Absent,
        ObservationResult::Confirmed,
        BTreeMap::new(),
        true,
        true,
    );
    (request, pending, observation)
}

fn sample_state(name: &str) -> BatchRecoveryWorkflowState {
    let (_, pending, observation) = fixture();
    match name {
        "BatchPending" => BatchRecoveryWorkflowState::BatchPending(pending),
        "Completed" => BatchRecoveryWorkflowState::Completed(
            SettledBatch::new("probe".into(), BatchOutcome::Applied, observation).unwrap(),
        ),
        _ => BatchRecoveryWorkflowState::Ready,
    }
}

fn sample_input(value: &Value) -> BatchRecoveryWorkflowInput {
    let (request, pending, observation) = fixture();
    let command = match value["name"].as_str().unwrap_or("DecideMemory") {
        "BeginBatch" => BatchRecoveryWorkflowCommand::BeginBatch(request),
        "ObserveRecovery" => BatchRecoveryWorkflowCommand::ObserveRecovery(observation),
        "ResumeRecovery" => BatchRecoveryWorkflowCommand::ResumeRecovery(pending, observation),
        _ => {
            let budget = value["data"]["budget"].as_u64().unwrap_or(32768) as usize;
            BatchRecoveryWorkflowCommand::DecideMemory(RenderRequest::Context(
                vec![],
                Policy::default(),
                ContextOptions::new(budget),
            ))
        }
    };
    BatchRecoveryWorkflowInput::Command(command)
}

fn record_json(
    record: &BatchRecoveryWorkflowTransitionRecord,
    input: &Value,
    first: bool,
) -> Value {
    let reply = record.output.reply.as_ref().map(|reply| {
        variant(match reply {
            BatchRecoveryWorkflowReply::MemoryDecision(_) => "MemoryDecision",
            BatchRecoveryWorkflowReply::BatchPlan(_) => "BatchPlan",
            BatchRecoveryWorkflowReply::BatchSettled(_) => "BatchSettled",
        })
    });
    let rejection = record.output.rejection.as_ref().map(|rejection| {
        variant(match rejection {
            BatchRecoveryWorkflowRejection::MemoryRejected(_) => "MemoryRejected",
            BatchRecoveryWorkflowRejection::IllegalTransition => "IllegalTransition",
        })
    });
    json!({
        "scenario_start":first,
        "state_before":variant(state_name(&record.state_before)),
        "state_after":variant(state_name(&record.state_after)),
        "input":input,
        "output":{
            "next_state":variant(state_name(&record.output.next_state)),
            "events":[], "commands":[], "effects":[],
            "reply":reply, "rejection":rejection,
        },
        "source":{"file":record.source.file,"function":record.source.function,"branch":record.source.branch},
    })
}

fn description() -> Value {
    let states = ["Ready", "BatchPending", "Completed"]
        .into_iter()
        .map(|name| {
            json!({
                "name":name, "data_schema":{"type":"object","additionalProperties":false},
                "examples":[variant(name)],
            })
        })
        .collect::<Vec<_>>();
    let inputs = ["DecideMemory", "BeginBatch", "ObserveRecovery", "ResumeRecovery"]
        .into_iter().map(|name| {
            let data = if name == "DecideMemory" { json!({"budget":32768}) } else { json!({}) };
            let schema = if name == "DecideMemory" {
                json!({"type":"object","properties":{"budget":{"type":"integer","minimum":0,"maximum":32768}},"required":["budget"],"additionalProperties":false})
            } else { json!({"type":"object","additionalProperties":false}) };
            json!({"kind":"command","name":name,"data_schema":schema,"example":{"kind":"command","name":name,"data":data}})
        }).collect::<Vec<_>>();
    json!({
        "spec":"rms/machine-probe-description/v0.1",
        "machine":"BatchRecoveryWorkflowMachine",
        "initial_state":variant("Ready"), "states":states, "inputs":inputs,
    })
}

fn evaluate(request: &Value) -> Value {
    let results = request["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| {
            let state = sample_state(case["state"]["name"].as_str().unwrap_or("Ready"));
            let input = &case["input"];
            let record = transition_record(state, sample_input(input));
            json!({"id":case["id"],"record":record_json(&record,input,false)})
        })
        .collect::<Vec<_>>();
    json!({"spec":"rms/machine-probe-evaluation/v0.2","machine":"BatchRecoveryWorkflowMachine","results":results})
}

fn trace(request: &Value) -> Value {
    let mut state = initial_state();
    let records = request["steps"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(index, step)| {
            let input = &step["input"];
            let record = transition_record(state.clone(), sample_input(input));
            state = record.state_after.clone();
            record_json(&record, input, index == 0)
        })
        .collect::<Vec<_>>();
    json!({"spec":"rms/trace-bundle/v0.1","machine":"BatchRecoveryWorkflowMachine","records":records})
}

#[test]
fn probe_machine() {
    let (Ok(input), Ok(output)) = (
        std::env::var("RMS_PROBE_REQUEST"),
        std::env::var("RMS_PROBE_OUTPUT"),
    ) else {
        return;
    };
    let request: Value = serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
    let response = match request["operation"].as_str().unwrap_or("") {
        "describe" => description(),
        "evaluate" => evaluate(&request),
        _ => trace(&request),
    };
    std::fs::write(output, serde_json::to_vec_pretty(&response).unwrap()).unwrap();
}
