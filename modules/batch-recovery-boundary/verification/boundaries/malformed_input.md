# Boundary Evidence: malformed input

Promise:

- `BatchRecoveryBoundaryMachine` parses and validates untrusted input before pure decisions run.
- Malformed input is rejected with an explicit failure path.

Command/tool:

- `rms verify implementation.yaml` runs generated boundary tests.
- `rms trace check verification/traces/malformed_input_trace.yaml` validates the malformed-input trace.

Expected result:

- Valid input progresses through parsed/delegated/completed boundary states.
- Malformed input produces `BatchRecoveryBoundaryRejection` through the rejection channel and does not delegate a domain command.

Source revision: recorded by git commit or strict audit provenance before production use.
