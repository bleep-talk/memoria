# Property Evidence: transition properties

Promise:

- Property `BatchRecoveryWorkflow-transition-output-is-declared` proves the transition model returns only variants declared by the current canonical machine.

Input space:

- Generated valid values accepted by the declared command constructors.
- Generated invalid values rejected by validated constructors or the typed rejection channel.

Oracle:

- every transition output has a declared `BatchRecoveryWorkflowState` next state
- accepted commands emit only declared `BatchRecoveryWorkflowEvent` and `BatchRecoveryWorkflowReply` variants
- rejected commands return declared `BatchRecoveryWorkflowRejection` variants instead of throwing
- replay records keep state before, state after, input, output, and source branch consistent

Command/tool:

- `rms property run implementation.yaml --profile smoke` executes the exact declared runner.
- `rms trace run implementation.yaml --profile smoke --record` records execution-derived traces.

Expected result:

- Generated cases execute the declared operation and oracle independently.
- Any failing case is identified by property id and can be recorded as `rms/property-counterexample/v0.1`.

Source revision: resolved from the committed candidate by strict audit.
