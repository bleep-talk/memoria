use batch_recovery_decisions::*;
use std::collections::BTreeMap;

fn generate_cases() -> Vec<(
    BatchRecoveryWorkflowState,
    BatchRecoveryWorkflowInput,
    &'static str,
)> {
    let path = MemoryPath::new("knowledge/exhaustive.md").unwrap();
    let request = BatchRequest::new(
        "exhaustive".into(),
        vec![Mutation::Create {
            path: path.clone(),
            content: "# Exhaustive\n".into(),
        }],
        BTreeMap::from([(path.clone(), None)]),
        Policy::default(),
    )
    .unwrap();
    let pending = PendingBatch::new(request.prepare().unwrap());
    let observation = RecoveryObservation::new(
        "exhaustive".into(),
        pending.batch().digest().clone(),
        0,
        PlanAction::Prepare,
        CompletionMarker::Absent,
        ObservationResult::Confirmed,
        BTreeMap::from([(path, None)]),
        true,
        true,
    );
    let states = [
        BatchRecoveryWorkflowState::Ready,
        BatchRecoveryWorkflowState::BatchPending(pending.clone()),
        BatchRecoveryWorkflowState::Completed(
            SettledBatch::new(
                "exhaustive".into(),
                BatchOutcome::Applied,
                observation.clone(),
            )
            .unwrap(),
        ),
    ];
    let expected = [
        [
            "ready_memory_valid",
            "begin_valid_batch",
            "observe_without_batch",
            "resume_valid_recovery",
        ],
        [
            "pending_memory_valid",
            "overlapping_batch",
            "observation_advances_stage",
            "pending_resume_valid",
        ],
        [
            "completed_memory_valid",
            "completed_begin_illegal",
            "terminal_observation_duplicate",
            "completed_resume_illegal",
        ],
    ];
    states
        .into_iter()
        .enumerate()
        .flat_map(|(row, state)| {
            let commands = [
                BatchRecoveryWorkflowCommand::DecideMemory(RenderRequest::Context(
                    vec![],
                    Policy::default(),
                    ContextOptions::default(),
                )),
                BatchRecoveryWorkflowCommand::BeginBatch(request.clone()),
                BatchRecoveryWorkflowCommand::ObserveRecovery(observation.clone()),
                BatchRecoveryWorkflowCommand::ResumeRecovery(pending.clone(), observation.clone()),
            ];
            commands
                .into_iter()
                .enumerate()
                .map(move |(column, command)| {
                    (
                        state.clone(),
                        BatchRecoveryWorkflowInput::Command(command),
                        expected[row][column],
                    )
                })
        })
        .collect()
}

#[test]
fn exhaustive_transitions() {
    let cases = generate_cases();
    assert_eq!(cases.len(), 12);
    for (state, input, expected_case) in cases {
        let first = transition(state.clone(), input.clone());
        let second = transition(state.clone(), input.clone());
        let record = transition_record(state.clone(), input);
        assert_eq!(first, second);
        assert_eq!(first.case, expected_case);
        assert_eq!(record.source.branch, expected_case);
        assert_eq!(record.output, first);
        assert_eq!(record.state_after, first.next_state);
        assert_ne!(first.reply.is_some(), first.rejection.is_some());
        assert!(first.effects.is_empty());
        assert!(first.events.is_empty());
        assert!(first.commands.is_empty());
        if first.rejection.is_some() {
            assert_eq!(first.next_state, state);
        }
    }
    if let Some(path) = std::env::var_os("RMS_HUNT_OUTPUT") {
        std::fs::write(
            path,
            r#"{"spec":"rms/hunt-lane-result/v0.1","status":"pass","metrics":{"cases":12},"artifacts":[]}"#,
        )
        .unwrap();
    }
}
