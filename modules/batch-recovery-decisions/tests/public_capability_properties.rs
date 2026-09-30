use batch_recovery_decisions::*;
fn generate_requests() -> Vec<BatchRecoveryWorkflowInput> {
    (0..512)
        .map(|budget| {
            BatchRecoveryWorkflowInput::Command(BatchRecoveryWorkflowCommand::DecideMemory(
                RenderRequest::Context(vec![], Policy::default(), ContextOptions::new(budget)),
            ))
        })
        .collect()
}
#[test]
fn public_capability_property() {
    for input in generate_requests() {
        let one = transition(initial_state(), input.clone());
        let two = transition(initial_state(), input.clone());
        assert_eq!(one, two);
        assert_ne!(one.reply.is_some(), one.rejection.is_some());
        assert!(one.effects.is_empty());
        assert!(one.events.is_empty());
        let record = transition_record(initial_state(), input);
        assert_eq!(record.output, one);
        assert_eq!(record.state_after, one.next_state);
        if one.rejection.is_some() {
            assert_eq!(record.state_before, record.state_after);
        }
    }
}
