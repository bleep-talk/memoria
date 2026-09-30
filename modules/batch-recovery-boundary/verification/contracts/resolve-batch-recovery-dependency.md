# Dependency Contract Evidence: resolve-batch-recovery-capability

Promise:

- Module `batch-recovery-boundary` depends on `batch-recovery-decisions` through required capability `resolve-batch-recovery-capability`.
- The required contract is `contracts/resolve-batch-recovery-capability.v1.yaml` and must stay compatible with the provider contract.
- Boundary input is translated into the required domain command before domain delegation.

Command/tool:

- `rms compose --root <system-root>` compares required and provided capability contracts.
- `rms verify implementation.yaml` exercises parser/delegation evidence when a binding exists.

Expected result:

- Required capability `resolve-batch-recovery-capability` resolves to provider `batch-recovery-decisions` with the same contract reference.
- Malformed boundary input stops before domain delegation.
- Accepted boundary input produces an enveloped domain command compatible with `resolve-batch-recovery-capability`.

Source revision: recorded by git commit or strict audit provenance before production use.
