# Evidence: contract proves resolve-batch-recovery-capability

Scenario: native to pure decision routing.

Observed command: `cargo +stable-aarch64-apple-darwin test --workspace --all-targets --quiet`.

Observed test: `check_provider_routing`.

Result: The native boundary forwards policy and transition decisions through the pure provider.

Scope: macOS workspace tests passed. The extracted source archive also passed `cargo test --workspace --all-targets --locked --quiet` on Linux. These observations do not certify unrun failure schedules or the RMS composite closure.
