# Randomness authority

`src/fs.rs#Confined::ensure_repository_id` generates a UUIDv4 repository identity. `Confined::replace` generates UUIDv4 temporary filenames before atomic rename. `src/effects/read_snapshot.rs#execute` reaches the identity path; other declared executor rows include `randomness` when replacement or identity creation is reachable. Random values identify artifacts; they do not decide policy, expected-content checks, or recovery action.

Observed proof: initialization, reopen, repository-identity mismatch, concurrent-process, replacement, and crash-recovery tests passed on macOS and Linux. The journal records the repository identity and rejects a record from another repository.
