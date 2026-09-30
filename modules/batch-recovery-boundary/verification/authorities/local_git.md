# Local Git authority evidence

`src/git.rs` owns repository inspection, status, diff, log, object creation, conditional reference publication, and index synchronization. It uses `git2` without hooks, checkout, or network operations. It builds a commit tree from managed file bytes without staging through the caller's index.

Observed proof: the macOS and Linux workspace suites passed. `commit_keeps_git_index_clean_and_rejects_existing_staging` tests initial publication, clean index handling, and rejection of staged work. `commit_publication_recovers_after_process_death` terminates the process before and after reference publication, then reconciles the durable commit intent. `commit_recovery_preserves_external_staging` preserves unrelated staged work. Bare repository and gitlink tests reject unsupported Git forms.

Scope: Git publication still depends on supported local filesystem durability and an unchanged expected parent. An uncertain outcome stays explicit until reconciliation.
