# Memoria

Git-backed Markdown memory for agents. The agent decides what to remember. Memoria validates, stores, versions, and compiles that memory into bounded context.

The `memoria-memory` library and `memoria` CLI run on macOS and Linux. Focused RMS verification and packaged-consumer checks pass.

```sh
just test
just package
```

The source archive in `dist/` contains the complete Cargo workspace. An external crate can depend on `memoria-memory` by path after extraction. The workspace has not been published to crates.io.

See [implementation status](docs/STATUS.md) for completed work, current verification limits, and the next blocker.

## Product boundary

- CLI: `memoria`.
- Public Rust library: `memoria-memory`.
- Platforms: macOS and Linux, on local filesystems.
- Memory stays inspectable through ordinary editors, filesystem tools, and Git.
- Search, LLM calls, automatic learning, network synchronization, checkout, and package publication are outside v1.

See [the implementation brief](docs/IMPLEMENTATION.md) for the accepted requirements. RMS module contracts own the executable semantics once applied.
