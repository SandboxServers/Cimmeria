# Native desktop minimum-launcher admission

> **Type:** Reference
> **Audience:** Desktop launcher and release maintainers
> **Date:** 2026-10-04
> **Scope:** Native compatibility policy and Install/Launch admission only

The engine checks `min_launcher` from authenticated game manifests before a
new Install or Launch can persist its admission. An older stamped launcher gets
`IntentError::LauncherTooOld`; the native owner exposes `MinimumStatus` for a
future UI explanation. No updater endpoint, signing key, download or executable
replacement is introduced by this packet.

## Identity and ordering contract

`launcher_compatibility::Identity` records desktop package version, optional
source revision, legacy compatibility tag and build epoch separately. The
compiled package version currently comes from the engine package version;
release integration must keep it aligned with the shell/package SemVer. The
optional source is `CIMMERIA_DESKTOP_SOURCE_REVISION`. Compatibility uses
`CIMMERIA_LAUNCHER_TAG` and `CIMMERIA_LAUNCHER_BUILD_EPOCH`, as the legacy launcher
does. A date/hash tag is never converted to SemVer. Package version is display
metadata and does not determine the game compatibility minimum.

`DesktopState::open` installs the compiled identity and an empty known-release
list. `open_with_compatibility` is a native composition-root seam for release
ordering and tests, not an IPC command. Its immutable policy cannot be replaced
by preferences or renderer input. `KnownRelease` records a tag and publication
epoch; it does not fetch or authenticate release metadata. A later native feed
adapter must establish that trust before injecting ordering. No ordering cache
is persisted by this packet.

| Signed manifest minimum | Result |
|---|---|
| Missing/blank | Satisfied |
| Build tag/epoch missing or malformed | Development exemption |
| Minimum malformed | Explicit malformed status; allowed for legacy parity |
| Same tag, or older tag date | Satisfied |
| Later tag date | Too old; reject Install and Launch |
| Different tag on same date, known publication after build epoch | Too old |
| Same date, known publication at/before build epoch | Satisfied |
| Same date, ordering absent/offline | Explicit unknown-same-day status; allowed |

The tag parser intentionally preserves the legacy eight-digit/alphanumeric
format, including its lack of calendar validation. These exemptions preserve
legacy behavior; they are not strict minimum enforcement. Tests use a desktop
version independent of the legacy tag, including a very large package version
that still cannot override a later compatibility minimum.

## Offline evidence and admission boundaries

Install reads the native `VerifiedRelease`. Launch and
`installed_launcher_minimum()` reverify the retained original signature and
manifest bytes against the durable installation's manifest digest. Reopening
requires no catalog request. A stale but validly signed different manifest,
missing bytes or damaged bytes fail closed. This checks the installed signed
manifest, not freshness against an unavailable latest online manifest.

Launch uses a read-only installed-content view, including the older successful
Install fallback, so a blocked request cannot create the lazy installed-content
index. Tests compare the complete fixture tree and operation revision before
and after rejection. Existing operation ownership, stale-revision and duplicate
dispatch rules remain in place. An already-admitted idempotent launch retry does
not dispatch again. Repair, runtime preparation and recovery retain their own
admission rules; the minimum check does not strand those recovery operations.

## Validation and remaining gates

Policy unit tests cover date ordering, same-day publication thresholds, exact
tags, unrelated ordering, absent ordering, malformed minima, partial/development
stamps and package-version independence. Native temp-directory tests call real
Install/Launch admissions, reopen persisted state, and reject wrong-digest and
corrupt cached evidence without mutation. Fixtures are signed with the existing
test/development key and contain only inert helper bytes.

Validation completed on macOS with pinned Rust 1.98.1, through the build lane:

- Engine library suite: 322 passed, 15 ignored (337 total).
- Engine all-target Clippy with warnings denied: passed.
- Desktop workspace formatting: passed.
- Revert verification: removing both minimum admission checks made the signed
  Install and offline Launch regression tests fail; restoring the checks passed
  the four admission tests again.

The 15 existing ignored tests were not exercised. No new ownership mutation is
introduced; the existing library ownership guards passed with the full suite.

Shell error mapping and shared documentation/index integration belong to the
coordinator. No frontend is changed here, so JS REPL/visual UAT is deferred until
visible minimum-status wiring lands. Production release stamping, SemVer/package
version validation, trusted release discovery, signed updater artifacts,
platform installation/relaunch, rollback and native Windows proof remain
separate release gates. No live game, production network, telemetry or updater
execution is exercised.

## Shell integration

The native minimum rejection maps to `launcher_too_old` at IPC. Install and Play
show an explicit update requirement rather than a transport failure. Frontend
guards assert one dispatch and unchanged state. Network update/download/apply
controls remain a separate packet. Migration corruption decoding was also
corrected during combined integration after independent review.
