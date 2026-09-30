# Implementation status

Memoria 0.1.0 is a standalone Rust library and CLI candidate. The agent chooses what to remember. Memoria validates, stores, versions, and compiles that memory into bounded context.

## Delivered behavior

- A `memoria-memory` public facade and separate `memoria` CLI crate.
- Validated relative paths, host policy, Markdown metadata, content hashes, and optimistic write preconditions.
- Deterministic context and tree projections with exact byte budgets.
- Directory-relative, no-follow filesystem access; one repository lock; a bounded durable batch journal; rollback and conflict-preserving recovery.
- Git history operations with staged-work protection, parent fencing, durable commit intent, and uncertain-publication reconciliation.
- Versioned CLI JSON results and stable error codes.

## Verification

- `just test` and `just format-check` pass on macOS arm64.
- The source archive passes all Cargo workspace targets in a Linux arm64 container on its own filesystem.
- A fresh external Cargo crate imports the packaged public API and completes init, write, and read.
- The packaged release CLI completes init → write → context → diff → commit → log.
- RMS pure, native, and composite focused verification passes. Pure nightly finite transitions and native nightly libFuzzer and Clippy lanes pass.
- `rms check --changes --root .` passes for the three affected RMS closures. This is partial coverage of the project, not a whole-repository or production certification.
- Both portable RMS module packages pass their package-integrity checks.

The tests cover crash phases, process contention, external-edit conflicts, symlinks, Git edge cases, context boundaries, malformed metadata, CLI JSON, and package consumption. The bounded fuzz lane checks the request parser oracle; it does not exhaust filesystem or Git interleavings.

The bootstrap commit is `b5318ee`. Package publication remains outside v1 delivery.
