# Signed launcher package checks

> Reference · desktop launcher maintainers · 2026-10-04

The desktop launcher can check a native-configured release feed, download a
package, and verify its Minisign signature and signed version before saving it.
It cannot install a package, replace the running launcher, relaunch, or roll back.
The production composition currently supplies no endpoint or signing key, so
its visible status is **Disabled**. Game-manifest signatures and legacy launcher
checksums remain separate mechanisms.

`updater_command` accepts schema 1 `inspect`, `check`, or `prepare`. Check uses
the inspected updater `revision` and `operation_revision`. Prepare adds only
the opaque `offer_id`. No URL, key, filesystem path, executable arguments or
artifact bytes are accepted from the renderer. There is no Apply command.
Snapshots expose version and bounded notes as text, never signature material or
artifact locations. An available offer is an unverified feed announcement;
**Ready** means a verified saved package, not an installed update.

The static feed has Tauri's `version`, optional `notes`, and `platforms` map;
the native target selects `darwin-aarch64`, `darwin-x86_64`, or `windows-x86_64`.
A platform contains `url` and the base64-encoded Minisign signature document.
HTTP 204 means no offer. Stable SemVer must have greater precedence than the
compiled native version; build metadata does not turn an equal version into a
newer release. Prerelease offers are not accepted in this stable channel.

Verification follows [Tauri updater v2.13.1's pinned implementation](https://github.com/tauri-apps/plugins-workspace/blob/e51128438011755f9e7277bad29b8c0978cf281c/plugins/updater/src/updater.rs):
`minisign-verify` 0.2.4 verifies both artifact and global signatures before reading
the trusted comment. Exactly one tab-separated `version:` field must equal the
announced SemVer. Thus an old signed artifact relabeled as a new release fails.
The implementation rejects duplicate version fields, more strictly than Tauri's
first-match parser. Release automation must still pin and test a Tauri CLI that
emits this field; no CLI, keys, endpoints or publishing are configured here.

Transport requires HTTPS, port 443, an exact native host allowlist, no URL
credentials or fragments, and at most four followed redirects. Every redirect
is checked before following. Connect timeout is 10 seconds; total request time
is 30 seconds for a feed and 300 seconds for a package. Feeds are capped at
64 KiB, notes at 4 KiB, URLs at 2 KiB, signatures at 8 KiB and packages at
256 MiB. Advertised length and every streamed chunk are bounded. Downloads
accumulate in bounded native memory; there is no partial resume. Verification
finishes before an atomic native staging write.

`launcher-update.json` records revisioned checking, available, downloading,
verifying, ready and failed states. It shares DesktopState's process lock and
mutex. Updater admission rejects every nonterminal game/setup operation,
including reconciliation-required. Install, Play, Repair, uninstall, runtime setup, legacy import and failed-install
cleanup call `ensure_updater_idle()` before preparatory writes. Repair-backup
cleanup uses the same gate. Directory changes are blocked while updates run;
summary consent remains editable. New mutation paths must preserve this rule. A dropped
renderer request does not cancel the native task. An interrupted process is not
a successful update: reopen changes an in-progress phase to failed/interrupted.
Ready inspection rereads and reverifies the native staging file, including after
reopen; missing, modified or no-longer-newer packages lose Ready status. An
uncertain persistence commit requires reopening and blocks new admissions.

## Verification and remaining release gates

Rust tests cover actual ephemeral signed local feeds, tampered payloads and
staging, old signed artifacts with inflated versions, altered trusted comments,
missing/duplicate signed versions, host/redirect policy, size/deadline failures,
stale revisions/offers, cross-operation exclusion and restart recovery.
`frontend/updater-native-uat.mjs` exercises production Effect/view logic against
a Rust fixture process with actual HTTP downloads and temporary disk persistence.
It covers duplicate clicks, literal notes, verified reopen and tamper rejection.

This does not prove packaged Tauri IPC/layout, production HTTPS, Windows native
lock/durability behavior, installer completion, restart, application health or
rollback. Apply/recovery work must retain native ownership until safe handoff,
reverify saved bytes, record durable intent, and prove failure recovery on each
platform. Production key custody, updater-compatible CLI signing, release
publication, OS signing/notarization and release ordering remain separate gates.

## Settings integration

Settings provides Check, Download and Recheck controls. The application owns all
views for its lifetime; native operation-revision changes refresh updater
capabilities, so a completed game/setup operation does not strand an old offer
behind a stale revision. No update mutation is replayed automatically. The
composition UAT mounts the real views, completes a native fixture journal
operation while an offer is displayed, then downloads once using the refreshed
revision. Non-updater view payloads in that UAT remain inert fixtures.
