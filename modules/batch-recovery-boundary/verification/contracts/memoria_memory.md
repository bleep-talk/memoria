# Evidence: contract proves memoria-memory

Scenario: public MemoryRepo behavior.

Observed command: `cargo +stable-aarch64-apple-darwin test --workspace --all-targets --quiet`.

Observed test: `external_consumer_and_cli`.

Result: An independent test calls the public facade for repository initialization, write, and read.

Scope: macOS workspace tests passed. The extracted source archive also passed `cargo test --workspace --all-targets --locked --quiet` on Linux. These observations do not certify unrun failure schedules or the RMS composite closure.
