---
name: CI iteration speed boundaries
description: Parallel Python contracts, CI-only debug overrides, runtime caches and portable container dependency layers.
type: reference
---

# CI iteration speed (2026-10-23)

- Design and sampled GitHub timings: [docs/analysis/ci-iteration-speed.md](../../../docs/analysis/ci-iteration-speed.md). Samples are examples, not percentiles or promised savings.
- `test.yml` runs stock-Python contracts independently under the same code/full condition as the Rust full job. Both `ci gate` and nightly reporting depend on contracts. Generator, seed/telemetry preflight, default/chaos and live-DB validation remain.
- Lab, client-launch, DLL and runtime-probe workflows disable dev/test debug information only in CI. Independent client-launch and DLL tests/builds no longer wait on clippy.
- Hosted runtime-tool caches write only on main; obsolete PR runs cancel, main completes. Keep probe, player telemetry and lab-bridge targets distinct.
- Docker GHA exports layers, not RUN cache mounts. Pinned cargo-chef 0.1.78 cooks server and login-probe dependencies separately into portable layers; mounts are seeded on an empty/new recipe. Restore real sources before actual release builds. Prove cold and fresh-builder reuse before claiming a saving.
- Queue/rulesets, runner fleet, hosted publication, signing and security/smoke gates are unchanged. No container release/deployment, lab lease or client run is authorized by CI tuning.

Companions: [build-system.md](../../../docs/architecture/build-system.md), [docker/README.md](../../../docker/README.md), [pre-pr-checks.md](../../../docs/agents/pre-pr-checks.md).
