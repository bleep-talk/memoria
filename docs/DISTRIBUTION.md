# Homebrew distribution

The intended public install command is:

```sh
brew install bleep-talk/tap/memoria
```

It becomes available after `bleep-talk/memoria` has a release and
`bleep-talk/homebrew-tap` contains its formula. Neither is published by this tooling.

## Prepare

Run the `Prepare distribution` GitHub Actions workflow on the reviewed release
commit. It tests and builds on Apple Silicon, Intel macOS, and ARM64 and x86-64
Linux. It produces native archives, checksums, and `Formula/memoria.rb`.

The builds vendor Git and TLS dependencies. Packaging rejects binaries that link
to non-system libraries and checks a real memory write, context read, and commit.
Each archive includes the CLI, agent skill, MIT license, and dependency notices.

For a local native build, use Python 3.11+ and a matching Rust toolchain:

```sh
python3 scripts/distribution.py build
```

After collecting all four platform archives in `dist/releases/`, generate the formula:

```sh
python3 scripts/distribution.py formula
```

macOS packages target macOS 13+. Linux packages use Ubuntu 22.04 and require
glibc 2.35 or newer. Other operating systems and older environments are not covered
by these binary builds.

## Publish and verify

1. Complete the source and history review before creating the public repositories.
2. Tag the exact build commit as `v<version>` and attach the four archives and
   checksum files to its GitHub release. The CLI crate owns the version.
3. Copy the generated `Formula/memoria.rb` into `bleep-talk/homebrew-tap`.
4. On each supported platform, run `brew install bleep-talk/tap/memoria`,
   `brew test bleep-talk/tap/memoria`, and the README example. Verify installed
   dependencies and platform compatibility on clean machines.
5. Mark Homebrew as available in the README only after these checks pass.

The installed skill is at `$(brew --prefix memoria)/share/memoria/skill/SKILL.md`.
Users can copy its folder into their agent's personal skills directory.

For every update, build fresh archives and regenerate the formula from those
archives. Never replace assets for an existing version: its checksums identify
the files users install.
