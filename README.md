# Memoria

Give your agent a memory.

Keep the preferences, decisions, and discoveries that make the next conversation better.

Memoria started with our own agents. We wanted something simple and minimal: a few commands, readable files, and no extra service to run. Markdown lets us see what our agents remember. Git lets us review how it changes.

## Features

- **Pick up where you left off.** Keep user preferences, project decisions, and useful discoveries ready for the next conversation.
- **Keep prompts focused.** Include essentials in each request. Search or browse other memory when needed.
- **Give agents shared project knowledge.** Agents on one machine can use the same memory folder. Memoria coordinates writes and rejects stale updates.
- **Review what your agent remembers.** Open the Markdown files, review changes, and record versions with Git.
- **Keep setup simple.** Install one local CLI. No database server, hosted service, or model API calls.

## Get started

Install with Homebrew on macOS or Linux:

```sh
brew install bleep-talk/tap/memoria
```

## Demo

Save a preference, then load it for your agent's next conversation:

```sh
memoria init ./memory
printf '%s\n' 'The user prefers short answers.' \
  | memoria --repo ./memory write system/user.md --expect absent
memoria --repo ./memory context
```

The `context` output contains your saved preference (formatted here for readability):

```json
{
  "entries": [
    {
      "path": "system/user.md",
      "metadata": {},
      "body": "The user prefers short answers.\n"
    }
  ]
}
```

Pass this output to your agent on its next request. Your agent chooses what to save and when to recall it.

<details>
<summary>Read the file and record its history</summary>

For commits, [configure your Git name and email](modules/batch-recovery-boundary/USAGE.md#git-history) first:

```sh
memoria --repo ./memory read system/user.md
memoria --repo ./memory commit 'Remember answer preference'
```

Writes save the files immediately; commits record their history.

</details>

## Connect your agent

Use Memoria with any agent that can run CLI commands. Give it the `memoria` command, the path to its memory folder, and the [Memoria skill](skills/memoria/SKILL.md). The skill teaches it how to save, find, and update memory.

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

Agents must write through Memoria to use its coordination and conflict checks. Do not edit files directly while agents are writing. On a conflict, read the file again before updating it.

Context has a size limit and reports an error if it is too large. Memory is data; your agent's instructions stay in charge. Memoria does not learn or sync automatically.

## Go deeper

- [CLI guide](modules/batch-recovery-boundary/USAGE.md)
- [Agent skill](skills/memoria/SKILL.md)
- [Implementation and verification status](docs/STATUS.md)

## Contributing

Bug reports, ideas, and focused pull requests are welcome. For bugs, include the command you ran, what happened, and what you expected. Discuss larger changes in an issue first.

To build from source, install from this checkout with [Rust and Cargo](https://www.rust-lang.org/tools/install):

```sh
cargo install --path modules/batch-recovery-boundary/cli --locked
```

For code changes, include tests for the behavior you change. Run these checks with `just` from this checkout:

```sh
just format-check
just test
```

## License

[MIT](LICENSE).
