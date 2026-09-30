# Evidence: law proves fenced-git-publication

Scenario: Git parent fence and uncertain publication recovery.

Observed command: `cargo +stable-aarch64-apple-darwin test --workspace --all-targets --quiet`.

Observed test: `commit_publication_recovers_after_process_death`.

Result: Git publication tests reconcile death before and after ref publication and preserve unrelated staged work.

Scope: macOS workspace tests passed. The extracted source archive also passed `cargo test --workspace --all-targets --locked --quiet` on Linux. These observations do not certify unrun failure schedules or the RMS composite closure.
