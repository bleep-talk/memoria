# Evidence: contract proves batch-recovery

Scenario: CLI command and JSON behavior.

Observed command: `cargo +stable-aarch64-apple-darwin test --workspace --all-targets --quiet`.

Observed tests: `json_success_and_request_errors_are_separate_streams` and `invalid_utf8_stdin_is_a_typed_request_error`.

Result: CLI integration tests cover commands, JSON envelopes, error codes, and failure exits.

Scope: macOS workspace tests passed. The extracted source archive also passed `cargo test --workspace --all-targets --locked --quiet` on Linux. These observations do not certify unrun failure schedules or the RMS composite closure.
