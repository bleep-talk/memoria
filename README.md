# Memoria

Git-backed Markdown memory for agents. The agent decides what to remember. Memoria validates, stores, versions, and compiles that memory into bounded context.

The `memoria-memory` library and `memoria` CLI run on macOS and Linux. Focused RMS verification and packaged-consumer checks pass.

```sh
just test
just package
```

The source archive in `dist/` contains the complete Cargo workspace. An external crate can depend on `memoria-memory` by path after extraction. The workspace has not been published to crates.io.

See [implementation status](docs/STATUS.md) for completed work and verification scope.

## Agent skill

[`skills/memoria/SKILL.md`](skills/memoria/SKILL.md) is a portable operating guide for an agent that has access to the `memoria` CLI or public library. It teaches memory placement, hash-based edits, conflict handling, and the boundary between memory data and instructions. The skill is separate from the repository created by `memoria init`.

To make the skill available across projects, copy the same folder to your agent's personal skills directory. For local Codex or Claude Code, run the relevant pair from this checkout:

```sh
mkdir -p ~/.codex/skills/memoria
cp skills/memoria/SKILL.md ~/.codex/skills/memoria/

mkdir -p ~/.claude/skills/memoria
cp skills/memoria/SKILL.md ~/.claude/skills/memoria/
```

The agent also needs the `memoria` binary on its `PATH` and an explicit memory repository path. For an agent without skill discovery, provide the same `SKILL.md` as instructions. The file grants no filesystem permissions.

## Product boundary

- CLI: `memoria`.
- Public Rust library: `memoria-memory`.
- Platforms: macOS and Linux, on local filesystems.
- Memory stays inspectable through ordinary editors, filesystem tools, and Git.
- Search, LLM calls, automatic learning, network synchronization, checkout, and package publication are outside v1.

See [the implementation brief](docs/IMPLEMENTATION.md) for the accepted requirements. RMS module contracts own the executable semantics once applied.
