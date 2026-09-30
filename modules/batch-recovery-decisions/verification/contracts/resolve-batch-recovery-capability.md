# Contract Evidence: resolve-batch-recovery-capability

Promise:

- Module `batch-recovery-decisions` publishes `resolve-batch-recovery-capability` through `contracts/resolve-batch-recovery-capability.v1.yaml`.
- Expected failures are returned as explicit rejections instead of hidden throws or implicit states.

Command/tool:

- `rms verify <module.yaml|implementation.yaml>` verifies the command against declared module structure.
- `rms compose --root <system-root>` checks provider/consumer compatibility when the command crosses module boundaries.

Expected result:

- `resolve-batch-recovery-capability` has a matching public contract artifact.
- Accepted and rejected paths are covered by the module's declared scenario or trace evidence.

Source revision: recorded by git commit or strict audit provenance before production use.
