# Fuzz Evidence: malformed boundary input

Promise:

- Fuzz target `BatchRecoveryBoundary-malformed-input-stops-before-domain` proves raw boundary input becomes an enveloped command or typed rejection before delegation.

Input space:

```yaml
raw_boundary_input:
  - empty objects
  - missing labels
  - wrong field types
  - unknown command tags
  - long strings and punctuation-heavy strings
```

Oracle:

- malformed input returns `BatchRecoveryBoundaryRejection` through the rejection channel
- malformed input does not emit delegated domain commands
- accepted input produces only declared `BatchRecoveryBoundaryCommand` values
- no parser case bypasses boundary validation or throws an ambient expected failure

Command/tool:

- `rms property run implementation.yaml --profile smoke`
- `rms verify implementation.yaml`

Expected result:

- Deterministic generated malformed inputs stop at the boundary.
- Any failing generated case can be recorded as `rms/property-counterexample/v0.1` and replayed.

Source revision: recorded by git commit or strict audit provenance before production use.
