---
name: Wine-backed adoption test fixture
description: Why adopted-copy Play admission is only reachable in supervised tests, and the inputs those need
type: reference
---

Verified 2026-10-04 on `launcher/effective-settings`.

macOS Play admission refuses a Native extraction backend, and the adoption
engine only records a Wine backend through `start_preview_wine`. That path calls
`mac_runtime::prepare`, which verifies the runtime tree against a pinned digest,
and then runs the Windows archive helper for the seed. There is no fixture seam,
so an adopted copy that can pass admission needs the real pinned runtime and
helper.

Two tiers follow:

- Always-run tests use a real Native-backed adoption of inert ZIP content. They
  reach the settings, resource-policy and signed-minimum gates, and stop at the
  platform backend gate.
- `#[ignore]` tests (`effective_settings::wine_tests`, shell
  `host::launch::adopted_tests::wine_*`) run the real helper in the test's own
  headless prefix. They need `CIMMERIA_WINE_HELPER`,
  `CIMMERIA_WINE_HELPER_SHA256` and `CIMMERIA_WINE_RUNTIME_TREE` (a verified
  `wine-r17-…` tree, cloned into the test's state so nothing is downloaded).

The fixture ZIP seed is accepted by the Windows helper; a stored-RAR fixture is
not required. After adoption the tests move their private runtime copy aside, so
the retained prerequisite and Play workers stop at their resource claim and
record `Cancelled` / `NotStarted` without starting Wine again. Prerequisite
success in these tests is recorded evidence, not a prepared prefix.

Shared builders live in `effective_settings::fixtures`, behind the engine's
`test-support` feature so the shell tests use the same ones.
