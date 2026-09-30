# Evidence: law proves canonical-effect-order

Scenario: journal-before-file effect ordering.

Observed command: `cargo +stable-aarch64-apple-darwin test --workspace --all-targets --quiet`.

Observed test: `journal_durability_is_a_barrier_before_file_application`.

Result: The transition test requires journal durability before file application.

Scope: macOS workspace tests passed. The extracted source archive also passed `cargo test --workspace --all-targets --locked --quiet` on Linux. These observations do not certify unrun failure schedules or the RMS composite closure.
