# Contract Evidence: batch-recovery

Promise:

- Module `batch-recovery` publishes `batch-recovery` through `contracts/batch-recovery.v1.yaml`.
- `batch-recovery` is backed by `batch-recovery-boundary` through declared RMS composition or dependency.
- Expected failures are returned as explicit rejections instead of hidden throws or implicit states.

Command/tool:

- `rms verify <module.yaml|implementation.yaml>` verifies the command against declared module structure.
- `rms compose --root <system-root>` checks provider/consumer compatibility when the command crosses module boundaries.

Expected result:

- `batch-recovery` has a matching public contract artifact.
- Accepted and rejected paths are covered by the module's declared scenario or trace evidence.

Source revision: recorded by git commit or strict audit provenance before production use.
