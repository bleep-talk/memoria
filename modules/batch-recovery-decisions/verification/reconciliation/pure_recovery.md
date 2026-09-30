# Recovery reconciliation evidence

Promise: `memoria-safe-batch-recovery`.

The pure recovery model treats each observed journal phase, file hash, completion marker, and operation ID as input. It selects rollback before durable completion and cleanup after completion. It rejects stale, contradictory, foreign, and externally changed observations. A repeated valid observation does not advance the operation twice.

Observed commands:

- `rms verify modules/batch-recovery-decisions/implementation.yaml` passed the generated `recovery_property` and the recorded decision trace.
- `cargo +stable-aarch64-apple-darwin test -p batch-recovery-decisions --test exhaustive_transitions --quiet` passed the 3-state × 4-command finite matrix.
- `cargo test --workspace --all-targets --locked --quiet` passed on Linux from the extracted source archive.

The generated history test iterates batch sizes 1–8 and each modeled interruption point. It checks rollback or cleanup settlement and duplicate observation handling. The finite matrix checks representative state and command variants. These observations do not prove every possible filesystem failure or an unbounded schedule.
