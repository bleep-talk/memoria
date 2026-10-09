# Memoria CLI

Build the nested CLI crate with the platform's native Rust toolchain. On Apple Silicon, run `cargo +stable-aarch64-apple-darwin build --manifest-path cli/Cargo.toml`. On Linux, use the installed native stable toolchain.

```sh
memoria init ./memory
printf '%s\n' '---' 'description: User preference' '---' 'The user prefers Rust.' | memoria --repo ./memory write system/user.md --expect absent
memoria --repo ./memory context --max-bytes 32768
memoria --repo ./memory tree --descriptions
memoria --repo ./memory --json search 'Rust' --limit 100
memoria --repo ./memory diff
memoria --repo ./memory commit 'Remember user preference'
memoria --repo ./memory log
```

`read` returns the current Markdown bytes and SHA-256 content hash. To replace a file, pass that hash to `write --expect <hash>`. To delete a file, pass the hash to `delete --expect <hash>`. `apply <batch.json>` accepts a JSON array of tagged create, replace, and delete mutations. The library validates the entire batch before writing a journal.

`search <query>` matches a nonempty, case-sensitive literal on each line of managed Markdown. Results are sorted by path and line. `--limit` defaults to 100 and accepts 1 through 1000. Each returned line is at most 512 UTF-8 bytes; `truncated` marks an excerpt. `limited` reports when more matching lines exist.

```json
[
  {"kind":"create","path":"knowledge/example.md","content":"Example\n"}
]
```

Each command accepts `--repo <directory>`. Without it, Memoria discovers the containing Git repository from the current directory. `--json` prints successful results as `{"schema_version":1,"data":...}` on stdout. It prints failures as `{"schema_version":1,"error":{"code":"...","message":"..."}}` on stderr and exits nonzero.

## Git history

`commit` uses your Git name and email. After initializing memory, configure them
for that repository. Replace the example values with your own:

```sh
git -C ./memory config user.name "Your Name"
git -C ./memory config user.email "you@example.com"
```

An existing global Git identity also works. These local settings apply only to
this memory repository.
