# Repository IO authority evidence

`src/fs.rs` opens managed path components relative to directory descriptors with `NOFOLLOW`. It reads only regular files under declared byte limits. Replacements use a same-directory temporary file, a file flush, rename, and parent-directory flush. `src/journal.rs` applies these operations to a bounded private recovery record. `AcquireRepoLock` holds an advisory exclusive lock across the driver lifecycle.

Observed proof: `cargo +stable-aarch64-apple-darwin test --workspace --all-targets --quiet` and the equivalent locked Linux test from the source archive passed. `two_process_repository_serialization` tests cooperating process exclusion. The crash tests terminate processes at journal persistence, replacement, deletion, completion, cleanup, and rollback points and reopen to reconcile. Symlink entry, symlink ancestor, gitlink, and journal-limit tests reject unsafe input. `external_edit_conflict_preserves_memory_and_journal` and `completed_journal_preserves_external_edit_conflict` preserve conflict evidence.

Scope: These tests cover local macOS and Linux filesystems. External editors are not coordinated by the advisory lock.
