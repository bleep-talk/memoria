# Contract Evidence: parent export

Promise:

- The composite parent public surface is backed by the declared child export.
- Public behavior changes keep parent contract evidence and child contract evidence aligned.

Command/tool:

- `rms compose --root <system-root>` verifies containment, visibility, and export backing.
- `rms verify <parent module.yaml>` rolls up child implementation verification when child bindings exist.

Expected result:

- Parent export is backed by the declared boundary child.
- Boundary child composes with the declared domain child through a contract-shaped capability.

Source revision: recorded by git commit or strict audit provenance before production use.
