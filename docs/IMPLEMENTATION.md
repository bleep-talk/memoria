# Memoria implementation brief

This brief records the accepted product requirements. Applied RMS module contracts own implementation semantics and evidence obligations. This document is not proof of implemented behavior.

## Ownership

The working files own current memory. Git owns committed history. Context and tree output are derived views. Host-supplied policy owns API permissions. Direct filesystem access remains owner authority; Memoria is not an operating-system sandbox.

The default memory roots are `system/`, `knowledge/`, `skills/`, and `conversations/`. Only eligible `system/` Markdown enters compiled context. Conversation history does not automatically become durable knowledge.

Pure roles own validation, preconditions, projections, and recovery decisions. Native roles own directory-relative filesystem access, locking, durability operations, and Git. Effect executors return typed observations. The driver owns lifecycle progression and retains transition records.

## Library and CLI

One `MemoryRepo` facade exposes `init`, `open`, `read`, `search`, `write`, `delete`, `exists`, `list`, `read_metadata`, `apply`, `build_context`, `tree`, `status`, `diff`, `commit`, and `log`.

Search uses literal case-sensitive matching over managed Markdown. It returns ordered path, line, and text matches with an explicit `limited` flag. Search does not modify memory or persist an index.

Reads return a content hash. Mutations require an expected content hash or an explicit absent precondition. The CLI has no policy bypass. The default policy protects `system/identity.md` and `system/policy.md` from mutation.

Markdown is UTF-8. Frontmatter is optional. Present frontmatter must be a YAML mapping; a present `description` must be a string. Writes preserve the supplied bytes and unknown metadata.

The CLI delegates to the library. `write` reads stdin. Commands accept `--repo`; otherwise they use the current repository. Programmatic output supports versioned JSON, stable error codes, nonzero error exits, and stderr diagnostics.

Initialization creates Git and the standard directories without an automatic memory commit. V1 excludes bare repositories, linked worktrees, submodules, network mounts, and synchronized folders.

## Safety and recovery

Reject absolute paths, traversal components, reserved Git paths, unsupported encodings, symlinks, and non-regular memory files. Use directory-relative, no-follow operations rather than check-then-access path handling.

A repository lock serializes cooperating Memoria calls. A batch validates all paths, policies, content, preconditions, and target conflicts before writing memory. It durably records a bounded journal and staged contents, applies changes, persists completion, and then removes recovery artifacts. Journals live privately under Git and do not duplicate memory history.

Recovery rolls back an incomplete batch and cleans up a completed batch. Unexpected external edits produce `RecoveryConflict`; preserve evidence instead of overwriting them. An operation ID correlates the active journal and diagnostics. Journal removal ends the durable retry-identity lifetime. A missing response requires reconciliation, not an unconditional retry.

External tools may observe intermediate files and must not write concurrently. Durability claims apply only to supported local filesystem behavior.

Git apply and commit remain separate operations. The Git adapter uses `git2`. Commits validate managed changes, reject pre-existing staged changes and unresolved Git operations, and compare the expected parent revision before publishing. An uncertain publication is reconciled against Git history before retry. Hooks, network operations, and checkout remain outside the interface.

## Derived views

Context recursively discovers eligible Markdown under `system/`, sorts repository-relative UTF-8 paths by bytes, and serializes ordered path, metadata, and body entries as deterministic JSON text. The default budget is 32768 bytes of complete serialized output. Return all entries or `ContextTooLarge`; do not truncate or omit. Invalid eligible files report their path. Empty system memory produces a valid empty context.

Tree output is deterministic and can include descriptions. It excludes Git internals and recovery artifacts. No persistent index is created.

## Delivery and proof

1. Apply RMS contracts, effects, roles, recovery transitions, and evidence obligations.
2. Implement the reusable library and focused native boundary.
3. Bind and test the thin CLI.
4. Verify packaged artifacts through an external consumer with no workspace-private imports.

Proof covers path attacks, protected files, frontmatter, Unicode, byte preservation, stale writes, conflicts, exact context budgets, deterministic views, process contention, process death, every journal phase, repeated recovery, external-edit conflicts, Git interoperability, uncertain commit publication, CLI JSON, and the complete user journey.

Use generated properties and replayable failure schedules for decisions. Use real filesystem and Git tests on macOS and Linux. Record limitations; do not claim unexecuted proof.

Local bootstrap and implementation commits are authorized. Package publication is not authorized.
