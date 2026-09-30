use crate::*;
fn output(
    state: BatchRecoveryWorkflowState,
    reply: Option<BatchRecoveryWorkflowReply>,
    rejection: Option<BatchRecoveryWorkflowRejection>,
    case: &'static str,
) -> BatchRecoveryWorkflowTransition {
    BatchRecoveryWorkflowTransition {
        next_state: state,
        events: Vec::new(),
        commands: Vec::new(),
        effects: Vec::new(),
        reply,
        rejection,
        case,
    }
}
fn rejected(
    state: BatchRecoveryWorkflowState,
    error: MemoryError,
    case: &'static str,
) -> BatchRecoveryWorkflowTransition {
    output(
        state,
        None,
        Some(BatchRecoveryWorkflowRejection::MemoryRejected(error)),
        case,
    )
}
fn illegal(
    state: BatchRecoveryWorkflowState,
    case: &'static str,
) -> BatchRecoveryWorkflowTransition {
    output(
        state,
        None,
        Some(BatchRecoveryWorkflowRejection::IllegalTransition),
        case,
    )
}
fn render(
    state: BatchRecoveryWorkflowState,
    request: RenderRequest,
    valid: &'static str,
    invalid: &'static str,
) -> BatchRecoveryWorkflowTransition {
    match render_memory(&request) {
        Ok(value) => output(
            state,
            Some(BatchRecoveryWorkflowReply::MemoryDecision(value)),
            None,
            valid,
        ),
        Err(error) => rejected(state, error, invalid),
    }
}
fn resume(
    state: BatchRecoveryWorkflowState,
    pending: PendingBatch,
    observation: RecoveryObservation,
    valid: &'static str,
    invalid: &'static str,
) -> BatchRecoveryWorkflowTransition {
    match pending.resume(observation) {
        Ok(RecoveryDecision::Continue(next, plan) | RecoveryDecision::Inspect(next, plan)) => {
            output(
                BatchRecoveryWorkflowState::BatchPending(next),
                Some(BatchRecoveryWorkflowReply::BatchPlan(plan)),
                None,
                valid,
            )
        }
        Ok(_) => rejected(state, MemoryError::InvalidRecoveryRecord, invalid),
        Err(error) => rejected(state, error, invalid),
    }
}
pub fn transition(
    state: BatchRecoveryWorkflowState,
    input: BatchRecoveryWorkflowInput,
) -> BatchRecoveryWorkflowTransition {
    use BatchRecoveryWorkflowCommand as C;
    use BatchRecoveryWorkflowReply as R;
    use BatchRecoveryWorkflowState as S;
    let BatchRecoveryWorkflowInput::Command(command) = input;
    match (state.clone(), command) {
        (S::Ready, C::DecideMemory(request)) => {
            render(state, request, "ready_memory_valid", "ready_memory_invalid")
        }
        (S::BatchPending(_), C::DecideMemory(request)) => render(
            state,
            request,
            "pending_memory_valid",
            "pending_memory_invalid",
        ),
        (S::Completed(_), C::DecideMemory(request)) => render(
            state,
            request,
            "completed_memory_valid",
            "completed_memory_invalid",
        ),
        (S::Ready, C::BeginBatch(request)) => match request.prepare() {
            Ok(batch) => {
                let pending = PendingBatch::new(batch);
                match pending.plan() {
                    Ok(plan) => output(
                        S::BatchPending(pending),
                        Some(R::BatchPlan(plan)),
                        None,
                        "begin_valid_batch",
                    ),
                    Err(error) => rejected(state, error, "begin_invalid_batch"),
                }
            }
            Err(error) => rejected(state, error, "begin_invalid_batch"),
        },
        (S::BatchPending(_), C::BeginBatch(_)) => illegal(state, "overlapping_batch"),
        (S::Completed(_), C::BeginBatch(_)) => illegal(state, "completed_begin_illegal"),
        (S::Ready, C::ObserveRecovery(_)) => illegal(state, "observe_without_batch"),
        (S::BatchPending(pending), C::ObserveRecovery(observation)) => {
            match pending.observe(observation) {
                Ok(RecoveryDecision::Continue(next, plan)) => output(
                    S::BatchPending(next),
                    Some(R::BatchPlan(plan)),
                    None,
                    "observation_advances_stage",
                ),
                Ok(RecoveryDecision::Inspect(next, plan)) => output(
                    S::BatchPending(next),
                    Some(R::BatchPlan(plan)),
                    None,
                    "observation_requires_reinspection",
                ),
                Ok(RecoveryDecision::Duplicate(plan)) => output(
                    state,
                    Some(R::BatchPlan(plan)),
                    None,
                    "observation_duplicate",
                ),
                Ok(RecoveryDecision::Settled(settled)) => output(
                    S::Completed(settled.clone()),
                    Some(R::BatchSettled(settled)),
                    None,
                    "cleanup_confirmed",
                ),
                Err(error) => rejected(state, error, "observation_invalid"),
            }
        }
        (S::Completed(settled), C::ObserveRecovery(observation)) => {
            if settled.is_duplicate(&observation) {
                output(
                    state,
                    Some(R::BatchSettled(settled)),
                    None,
                    "terminal_observation_duplicate",
                )
            } else {
                illegal(state, "terminal_observation_illegal")
            }
        }
        (S::Ready, C::ResumeRecovery(pending, observation)) => resume(
            state,
            pending,
            observation,
            "resume_valid_recovery",
            "resume_invalid_recovery",
        ),
        (S::BatchPending(current), C::ResumeRecovery(pending, observation)) => {
            if current.batch() != pending.batch() {
                rejected(state, MemoryError::ForeignBatch, "pending_resume_invalid")
            } else {
                resume(
                    state,
                    pending,
                    observation,
                    "pending_resume_valid",
                    "pending_resume_invalid",
                )
            }
        }
        (S::Completed(_), C::ResumeRecovery(_, _)) => illegal(state, "completed_resume_illegal"),
    }
}
pub fn transition_record(
    state: BatchRecoveryWorkflowState,
    input: BatchRecoveryWorkflowInput,
) -> BatchRecoveryWorkflowTransitionRecord {
    let result = transition(state.clone(), input.clone());
    BatchRecoveryWorkflowTransitionRecord {
        state_before: state,
        input,
        state_after: result.next_state.clone(),
        source: BatchRecoveryWorkflowSourceProvenance {
            file: "src/transition.rs",
            function: "transition",
            branch: result.case,
        },
        output: result,
    }
}
