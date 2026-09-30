# Process authority

`src/journal.rs#crash` can abort the current process when `MEMORIA_TEST_CRASH` matches a named injection point. `src/effects/apply_filesystem.rs#execute` is one declared effect executor that reaches it. The canonical effect rows include `process` for each transitive path. The helper never performs compensation or retry; a later Memoria call reopens the journal and asks the pure transition for recovery work.

Observed proof: `crash_restart_recovers`, `delete_and_rollback_crashes_recover_without_losing_memory`, and `commit_publication_recovers_after_process_death` terminate child processes at journal and Git points and reconcile after restart on macOS and Linux.
