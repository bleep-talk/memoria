# Evidence: law proves bounded-durable-recovery

Scenario: bounded durable journal and restart.

Observed command: `cargo +stable-aarch64-apple-darwin test --workspace --all-targets --quiet`.

Observed test: `crash_restart_recovers`.

Result: Process-death tests recover at recorded journal phases; oversized encoded journals reject before applying files.

Scope: macOS workspace tests passed. The extracted source archive also passed `cargo test --workspace --all-targets --locked --quiet` on Linux. These observations do not certify unrun failure schedules or the RMS composite closure.
