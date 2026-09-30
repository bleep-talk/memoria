# Evidence: law proves parsed-host-policy

Scenario: host policy at the public boundary.

Observed command: `cargo +stable-aarch64-apple-darwin test --workspace --all-targets --quiet`.

Observed test: `paths_policy_frontmatter_and_exact_context_budget`.

Result: Public integration tests reject protected and unsafe paths and verify metadata and context budgets.

Scope: macOS workspace tests passed. The extracted source archive also passed `cargo test --workspace --all-targets --locked --quiet` on Linux. These observations do not certify unrun failure schedules or the RMS composite closure.
