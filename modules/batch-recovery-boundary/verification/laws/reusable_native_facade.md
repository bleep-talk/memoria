# Evidence: law proves reusable-native-facade

Scenario: public API use outside the CLI.

Observed command: `cargo +stable-aarch64-apple-darwin test --workspace --all-targets --quiet`.

Observed test: `external_consumer_and_cli`.

Result: A separate consumer crate imported memoria_memory from the extracted source archive and completed init, write, and read.

Scope: macOS workspace tests passed. The extracted source archive also passed `cargo test --workspace --all-targets --locked --quiet` on Linux. These observations do not certify unrun failure schedules or the RMS composite closure.
