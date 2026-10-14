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
- Twelve added tests are below the 717-test inventory regeneration threshold.

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

## Review repair verification

The declaration and lexical guards both failed against the original validator
at runtime, reaching `stock_id_collision` instead of the required refusal
(lane `20261014-091433-74552`, 2 failures). Declaration cases cover duplicates,
ordering, standalone values, missing separators, unknown attributes and
placement. Lexical cases cover `]]>`, non-XML whitespace, comment controls and
invalid characters in attributes/names. Every case uses valid-first/bad-second
and checks the unchanged category plus WARN reason, category, mission and bytes.

The tightened oversize entry is exactly `MAX_MISSION_XML_BYTES + 1` bytes.
Runtime mutation proofs compiled and failed as required:

- Admit one extra byte: `xml_too_large` guard failed (`20261014-091831-66872`).
- Disable required-child check: `missing_required_child` guard failed
  (`20261014-091838-51316`), covering empty steps/objectives/display children.
- Demote admission WARN to TRACE: refusal-log guard failed
  (`20261014-091846-47496`).

All temporary changes were restored byte-for-byte. Final lane results:

- Format: `status=ok exit=0 ran=7.660s` (`20261014-091616-47300`).
- Targeted clippy: `status=ok exit=0 ran=8.027s pruned=253MB`
  (`20261014-091658-47496`).
- Restored resources/base-session nextest: `status=ok exit=0 ran=19.163s
  pruned=435MB`, **653 passed, 0 skipped** (`20261014-091925-54772`).

The insertion-only runtime proof above remains applicable: insertion, fixture
and the explicit one-transfer/body/client-entry assertions are unchanged, and
the real resync guard passed in the final run. This repair changes admission
and tests only; the coordinator's previous full-workspace results stand as
recorded above. Client presentation remains **UATPending**.
Targeted Markdown lint: 0 issues in the two changed documents; file links and
CRLF checks passed. `git -c core.whitespace=cr-at-eol diff --check` passed.

## Attribute-separator repair verification

quick-xml's attribute iterator accepts adjacent quoted attributes without XML S.
Admission now scans each bounded start/empty tag before the iterator and refuses
missing separators with stable reason `invalid_attribute_syntax`. Both quote
styles, XML S around equals signs, entity escapes and literal opposite quotes
remain accepted without changing the stored bytes.

The valid-first/bad-second root/start/empty-tag matrix pins WARN and unchanged
keys, bytes and metadata. Against the previous validator it compiled and failed
at runtime: `stock_id_collision` instead of `invalid_attribute_syntax`
(lane `20261014-093200-80880`). Final required checks passed in order:

- Format: `status=ok exit=0 ran=7.019s` (`20261014-093339-35736`).
- Targeted clippy: `status=ok exit=0 ran=4.662s pruned=253MB`
  (`20261014-093354-13816`).
- Resources/base-session nextest: `status=ok exit=0 ran=19.674s pruned=425MB`,
  **655 passed, 0 skipped** (`20261014-093406-76616`).

The insertion-only proof above remains applicable and its transport guard
passed. Client presentation remains **UATPending**.

## Review ledger and final gates

Three fresh independent reviews examined the full diff from base `19eab88c9` through final `b1d93ba0`, against the tracked packet and seal. Round one found two P2 XML admission gaps and one P3 public-API documentation gap; the BigWorld/client review independently confirmed the declaration P2. The test review identified three guard gaps: exact MAX+1 oversize coverage, WARN severity/fields, and missing-required-child validation. Repair commit `e5d370fb` closed these findings and guards.

Round two found one P2 gap: adjacent quoted XML attributes without required whitespace. Repair commit `b1d93ba0` added separator validation and atomic refusal coverage. Round three, a fresh Astra review of the full base-to-final diff, reported no findings. Review evidence is recorded in ignored `.claude/reports/four-zone/cb-n05a-*.md` reports. These reviews inspected source and tests; they do not attest live-client behavior.

The final full workspace CI nextest result supersedes the earlier count above: **11,932 passed, 16 skipped, 1 leaky process** (exit 0), lane run `20261014-094016-47288`. Final workspace clippy passed (`20261014-093840-73828`), workspace build passed (`20261014-093912-15016`), and commands doctests passed, 4/0 (`20261014-094233-47992`). Repository Markdown lint reported 4,820 issues across 551 files; it is warn-only and nonblocking. Live-DB and figure checks were skipped as inapplicable to this packet. No live-client test or presentation claim was made; client presentation remains **UATPending**.
