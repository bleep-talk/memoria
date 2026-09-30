# Scenario Evidence: composed capability

Promise:

- Invariant `public-command-is-child-backed` holds for composite parent `batch-recovery`.
- The parent exports `batch-recovery` through internal boundary child `batch-recovery-boundary`.
- The boundary child depends on internal domain child `batch-recovery-decisions` for pure decisions.

Evidence:

- `rms compose --root <system-root>` verifies containment, internal visibility, and parent export backing.
- `rms verify <this module.yaml>` rolls up composition and child implementation verification when child bindings exist.

Source revision: recorded by the verifier or conformance report at runtime.
