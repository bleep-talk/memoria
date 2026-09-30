# Contract Evidence: parser to domain command

Promise:

- `BatchRecoveryBoundaryMachine` translates boundary input into an enveloped domain command or explicit rejection before core decisions run.

Command/tool:

- `rms verify implementation.yaml` runs generated parser and adapter tests.
- `rms trace show verification/traces/boundary_parse.yaml` shows the recorded parse/delegate/complete path.

Expected result:

- Parser success produces `BatchRecoveryBoundaryCommand` or `BatchRecoveryBoundaryCommandEnvelope` values.
- Parser rejection produces `BatchRecoveryBoundaryRejection` without domain delegation.

Source revision: recorded by git commit or strict audit provenance before production use.
