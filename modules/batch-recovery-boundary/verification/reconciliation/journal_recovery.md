# Journal reconciliation evidence

Promise: `bounded-durable-recovery`.

The journal records operation ID, repository identity, before/after hashes, bounded recovery data, and a durable completion marker. On reopen, the native boundary supplies observed journal and file facts to the transition. An incomplete journal requests rollback. A complete journal requests cleanup. An unexpected external edit produces `RecoveryConflict` and preserves the journal as evidence.

Observed proof:

- `rms property run modules/batch-recovery-boundary/implementation.yaml --profile smoke` passed `check_crash_recovery` and `check_phase_protocol`.
- `rms property run modules/batch-recovery-boundary/implementation.yaml --profile ci` selected and passed `crash_restart_recovers` with one test.
- The macOS and Linux workspace suites passed `delete_and_rollback_crashes_recover_without_losing_memory`, `external_edit_conflict_preserves_memory_and_journal`, `completed_journal_preserves_external_edit_conflict`, `foreign_repository_identity_cannot_replay_journal`, and the encoded journal limit test.
- `rms trace run modules/batch-recovery-boundary/implementation.yaml --profile smoke` passed the recorded recovery and operation traces.

The tested schedules cover named crash points and selected external edits. They do not prove arbitrary storage failure or unbounded hostile concurrency.
