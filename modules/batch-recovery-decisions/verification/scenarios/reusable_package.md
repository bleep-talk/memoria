# Scenario Evidence: reusable package

Promise:

- Module `batch-recovery-decisions` is reusable through RMS capability `resolve-batch-recovery` and its declared public facade.
- Consumers depend on `provides.capabilities[]` and the public contract, not private representation, parser, adapter, or transition role files.
- RMS semantic packages are the portable reuse artifact; native package files are optional binding evidence only.

Command/tool:

- `rms package module.yaml` assembles, verifies, and records the canonical RMS package result.
- `rms verify-package dist/batch-recovery-decisions-0.1.0.rms` can independently verify package metadata, checksums, manifests, implementation binding, and conformance report.
- `rms compose --root <system-root>` verifies consumer `requires.capabilities[]` compatibility against this provider capability.

Expected result:

- The package contains `module.yaml`, contracts, implementation binding when present, declared public facade, evidence, conformance report, source revision, and checksums.
- Consumer modules import only the declared public facade or call through contract-shaped entrypoints.
- Expected-result prose alone is not production proof; `rms package` appends the exact recorded result after successful verification.

Source revision: recorded by git commit or strict audit provenance before production use.

<!-- rms:package-result:start -->

Recorded result:

- `rms package modules/batch-recovery-decisions/module.yaml --output dist/batch-recovery-decisions-0.1.0.rms`
- packaged RMS module at dist/batch-recovery-decisions-0.1.0.rms
- `rms verify-package dist/batch-recovery-decisions-0.1.0.rms`
- pass: RMS package verified dist/batch-recovery-decisions-0.1.0.rms
- pass [package.files.integrity] 33 checksummed payload file(s)

<!-- rms:package-result:end -->
