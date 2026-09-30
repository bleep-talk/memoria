# Native Git authority

`src/effects/inspect_git.rs#execute` reaches the pinned `git2` library. The source-linked analyzer reports the raw `git` effect separately from Memoria's declared `local-git` facade. Commit publication and reconciliation also call local Git operations. Their canonical semantic rows declare both effects. No hook execution, checkout, or network operation is exposed.

Observed proof: the macOS and Linux workspace suites passed. Tests cover Git index preservation, pre-existing staged work rejection, unsupported bare repositories and gitlinks, parent fencing, and process death before and after reference publication. Uncertain publication stays explicit until reconciliation.
