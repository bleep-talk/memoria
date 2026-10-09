# Memoria

Give your agent a memory.

Keep the preferences, decisions, and discoveries that make the next conversation better.

Memoria started with our own agents. We wanted them to carry useful things from one conversation to the next, with memory we could see and understand. We kept it simple: Markdown for memory, Git for history.

Your agent chooses what to save and when to recall it. You can open the files, read what it remembers, and see what changed.

## Get started

Memoria runs on macOS and Linux. Homebrew installation is being prepared:

```sh
brew install bleep-talk/tap/memoria
```

Until then, install from this checkout with [Rust and Cargo](https://www.rust-lang.org/tools/install):

```sh
cargo install --path modules/batch-recovery-boundary/cli --locked
```

Create a memory folder and save something worth remembering:

```sh
memoria init ./memory
printf '%s\n' 'The user prefers short answers.' \
  | memoria --repo ./memory write system/user.md --expect absent
```

Read the memory, prepare it for your agent, and save a version:

```sh
memoria --repo ./memory read system/user.md
memoria --repo ./memory context
memoria --repo ./memory commit 'Remember answer preference'
```

Pass the output of `context` to your agent on its next request. Writes save the files immediately; commits record their history.

## Connect your agent

Give your agent the `memoria` command, the path to its memory folder, and the [Memoria skill](skills/memoria/SKILL.md). The skill teaches it how to save, find, and update memory.

For Codex or Claude Code, copy the skill to `~/.codex/skills/memoria/SKILL.md` or `~/.claude/skills/memoria/SKILL.md`. Homebrew packages it at `$(brew --prefix memoria)/share/memoria/skill/SKILL.md`. Other agents can use the file as instructions.

Building an integration? Use `--json` for structured results or the `memoria-memory` Rust library from this workspace. The crates are not yet on crates.io.

## A place for what matters

- `system/`: The essentials. Included whenever you call `context`.
- `knowledge/`: Facts and project details to look up when needed.
- `skills/`: How to do things.
- `conversations/`: Exchanges worth keeping.

Only `system/` enters context automatically. Your agent reads the rest when needed.

Save a project decision and find it later:

```sh
printf '%s\n' 'We use Stripe for payments.' \
  | memoria --repo ./memory write knowledge/project.md --expect absent
memoria --repo ./memory search 'payments'
```

Use `tree` to browse all saved memory. The [CLI guide](modules/batch-recovery-boundary/USAGE.md) covers updating files and making several changes together.

Updates check for changes before replacing a file. If someone else changed it, your agent must read it again. Context has a size limit and reports an error if it is too large. Memory is data; your agent's instructions stay in charge.

Memory stays on your local filesystem. Memoria makes no model calls and does not learn or sync automatically.

## Go deeper

- [CLI guide](modules/batch-recovery-boundary/USAGE.md)
- [Agent skill](skills/memoria/SKILL.md)
- [Implementation and verification status](docs/STATUS.md)

## Contributing

Bug reports, ideas, and focused pull requests are welcome. For bugs, include the command you ran, what happened, and what you expected. Discuss larger changes in an issue first.

For code changes, include tests for the behavior you change. Run these checks with `just` from this checkout:

```sh
just format-check
just test
```

## License

[MIT](LICENSE).
