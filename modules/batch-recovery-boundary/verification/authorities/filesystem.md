# Native filesystem authority

`src/effects/release_lock.rs#execute` releases the repository lock through a native filesystem operation. Other effect executors use `Confined` directory handles for no-follow file reads, replacements, deletions, and durability flushes. The initial `execute_confined` facade opens the repository anchor; it does not enclose every later handle operation. The semantic authority rows therefore include the inferred `filesystem` effect where those later operations are reachable.

Observed proof: the macOS and Linux workspace suites passed. The process-serialization, symlink-entry, symlink-ancestor, journal-limit, process-death, and external-edit conflict tests exercise this boundary. The guarantee coordinates cooperating Memoria calls on supported local filesystems; external writers can still produce a typed conflict.
