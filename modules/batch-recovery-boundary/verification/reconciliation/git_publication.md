# Git publication reconciliation evidence

Promise: `fenced-git-publication`.

Commit persists an intent with the expected parent, reference identity, and intended commit before conditional publication. If publication becomes uncertain, the next locked operation inspects the reference and intended object. It returns a reconciled result only when the observed Git state supports it. Existing unrelated staged work remains intact.

Observed proof:

- `rms property run modules/batch-recovery-boundary/implementation.yaml --profile smoke` passed `check_git_publication` and the uncertain-publication transition test.
- The macOS and Linux workspace suites passed `commit_publication_recovers_after_process_death`, `commit_recovery_preserves_external_staging`, and `commit_keeps_git_index_clean_and_rejects_existing_staging`.
- `rms trace run modules/batch-recovery-boundary/implementation.yaml --profile smoke` passed the recorded publication trace.

The tested process deaths occur before and after reference publication. An unresolved or conflicting reference remains a typed error, not an accepted commit.
