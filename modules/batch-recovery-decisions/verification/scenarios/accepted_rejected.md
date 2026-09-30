# Scenario Evidence: accepted and rejected outcomes

Promise:

- `BatchRecoveryWorkflowMachine` exposes explicit accepted and rejected outcomes for expected failures.

Command/tool:

- `rms verify implementation.yaml` runs the generated binding tests.
- `rms trace show verification/traces/transition_trace.yaml` shows the recorded starter accepted/rejected sequence.

Expected result:

- Accepted input returns a declared `BatchRecoveryWorkflowReply` variant through `reply`.
- Rejected input returns `BatchRecoveryWorkflowRejection` through `rejection`, with no success reply, instead of throwing or hiding the failure.

Source revision: recorded by git commit or strict audit provenance before production use.
