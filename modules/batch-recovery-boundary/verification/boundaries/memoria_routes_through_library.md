# Boundary Evidence: runnable surface routes through boundary

Promise:

- Runnable surface `memoria` enters module `batch-recovery-boundary` through declared RMS command `batch-recovery`.
- Entrypoint `cli/src/main.rs` delegates to `cli/src/main.rs#run` before pure decisions run.
- Launch entrypoint `<none>` reaches the declared entrypoint when present.
- Launch scripts/assets `local scripts discovered from the launch entrypoint` do not duplicate parser or domain decisions outside declared RMS roles.
- Product behavior is not reimplemented only in the runnable surface.

Command/tool:

- `rms surface check implementation.yaml --strict`
- `rms structure implementation.yaml`
- `rms verify implementation.yaml`

Expected result:

- Surface wiring references the declared boundary adapter, parser, or public entrypoint.
- Malformed boundary input is parsed/rejected before domain delegation.
- Declared boundary effects remain behind adapter, port, or effect-executor roles.
- Any executable launch asset is either wired through the declared entrypoint or remains non-semantic UI glue.

Source revision: recorded by git commit or strict audit provenance before production use.
