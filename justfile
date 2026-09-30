toolchain := if os() == "macos" { "stable-aarch64-apple-darwin" } else { "stable" }

build:
    cargo +{{toolchain}} build --workspace --all-targets

test:
    cargo +{{toolchain}} test --workspace --all-targets

format-check:
    cargo +{{toolchain}} fmt --all -- --check

package:
    mkdir -p dist
    tar --no-xattrs -czf dist/memoria-source-0.1.0.tar.gz Cargo.toml Cargo.lock README.md modules/batch-recovery-decisions/Cargo.toml modules/batch-recovery-decisions/src modules/batch-recovery-decisions/tests modules/batch-recovery-boundary/Cargo.toml modules/batch-recovery-boundary/src modules/batch-recovery-boundary/tests modules/batch-recovery-boundary/fuzz/Cargo.toml modules/batch-recovery-boundary/fuzz/Cargo.lock modules/batch-recovery-boundary/fuzz/fuzz_targets modules/batch-recovery-boundary/fuzz/corpus modules/batch-recovery-boundary/tools modules/batch-recovery-boundary/USAGE.md modules/batch-recovery-boundary/cli/Cargo.toml modules/batch-recovery-boundary/cli/src modules/batch-recovery-boundary/cli/tests
    cargo +{{toolchain}} build --release -p memoria
    cp target/release/memoria dist/memoria
