# CB-N05a implementation worknote

> **Type:** Reference
> **Audience:** Infrastructure coordinator and reviewers
> **Last updated:** 2026-10-14

## Reservation and scope

Recorded before implementation on 2026-10-14: mission **61000**, step **61001**,
objective **61002**, task **61003** are reserved solely for the sealed
`CIMMERIA_AUTHORING` test fixture. This is not a production quest range. The
production `MISSION_ADDITIONS` registry stays empty; no seed or acceptance flow
uses these IDs. See the [sealed contract](../../four-zone-planning/cb-n05a-seal.md).

## Evidence status

Complete-entry admission and the shared cache loader are implemented. The
production registry is empty. Resource guards cover exact source transformation,
baseline preservation, deterministic metadata, whole-batch refusal and logs;
base-session drives the real resync task and requires the 61000 transfer body.

## Automated checks

- `cargo fmt --all -- --check`: passed.
- Lane targeted clippy for resources and base-session, all targets, `-D warnings`:
  passed (2026-10-14).
- Lane nextest for resources and base-session: **649 passed, 0 skipped**.
- Literal fixture: 1139 bytes; SHA-256 matches the seal's `AC37C6BD...CF7D0FD`.
- Permutation checks use existing stock 1624 plus 61000 only in an empty synthetic
  category. Fresh full-cache loads use the sealed fixture alone.

## Insertion-only revert proof

Safe implementation committed as `4e96e2e84` before temporary edits. Only the
exact-byte insertion statement in `apply_mission_additions.rs::insert` was
replaced with a no-op; validation, fixtures, API, metadata and test stayed intact.

```powershell
pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-base-session -E 'test(authored_mission_catalog_reaches_stock_client)'
```

It compiled and **failed at runtime** (exit 100): the explicit assertion
`new mission 61000 must be transferred exactly once` observed **0 vs 1**.
Evidence: lane run `20261014-085332-5352`, failure at `resync.rs:182`.
The generic cache-equality assertion was not the failure.

Restored only `apply_mission_additions.rs` from the safe commit and refreshed its
mtime, then reran:

```powershell
pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-resources -p cimmeria-base-session
```

**649 passed, 0 skipped**; lane run `20261014-085422-43832`. The insertion is
restored. Production still loads the empty registry.

## Additional local checks

- Full workspace clippy, six CI exclusions, all targets, `-D warnings`: passed.
- Full workspace build, six CI exclusions, all targets: passed.
- Commands doctests: **4 passed, 0 failed**.
- `cargo hakari generate --diff` and `cargo hakari manage-deps --dry-run`: passed.
- Targeted Markdown lint (`npx markdownlint-cli2 --no-globs` on the three companions
  and this worknote): 18 existing warnings in the two engine docs, identical to
  the contract baseline; no warning in new sections or the worknote.
- Relative Markdown file links resolve; changed docs retain CRLF.
- `git -c core.whitespace=cr-at-eol diff --check`: passed. The flag treats required
  CRLF as an end-of-line marker; the two engine docs already contain tracked CRLF.
- Eight added tests are below the 717-test inventory regeneration threshold.

- Full workspace CI nextest, six CI exclusions: **11926 passed, 16 skipped**,
  with one leaky process reported (exit 0); lane run `20261014-090109-67968`.
  The lane summary does not identify the leaky test. This run had no
  `DATABASE_URL`, so live-DB bodies self-skip and are not DB evidence.

The live-DB tier, live client, lab lease and server restarts are not part of this
packet.

## Telemetry and remaining evidence

Success is INFO `cooked_data.mission_catalog_inserted`, after final metadata, with
category/mission ID/name, byte count and version. Refusal is WARN
`cooked_data.mission_catalog_rejected`, with stable reason and safe context; no XML
body is logged. Existing cooked-sync version/start/finish/miss events carry delivery.

Client presentation remains
**UATPending**: XML and ClientModel checks cannot attest gSOAP parsing, rendered
journal/dialog text, progression or relog. The first real authored quest provides
that evidence after owner-approved lab access.
