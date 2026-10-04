# Signed launcher updates

> Reference · desktop launcher maintainers · 2026-10-04

The desktop launcher can check a native-configured release feed, download a
package, and verify its Minisign signature and signed version before saving it.
Apply re-verifies saved bytes and can replace a native Mac application bundle or
hand off a Windows installer. The new compiled launcher version must acknowledge
startup before an update is reported installed. The production composition currently supplies no endpoint or signing key, so
its visible status is **Disabled**. Game-manifest signatures and legacy launcher
checksums remain separate mechanisms.

`updater_command` accepts schema 1 `inspect`, `check`, `prepare`, or `apply`. Check uses
the inspected updater `revision` and `operation_revision`. Prepare adds only
the opaque `offer_id`. No URL, key, filesystem path, executable arguments or
artifact bytes are accepted from the renderer. Apply uses the same opaque offer
ID and both inspected revisions as Prepare.
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
a successful update: reopen changes interrupted downloads to failed/interrupted,
and installing/restart states to reconciliation-required while retaining ownership.
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

The Apply fixture exercises real temporary Mac bundle renames and a spawned
fixture executable, final-rename/spawn-failure rollback, interrupted recovery,
foreign-stage preservation, and compiled-version acknowledgment. Shell tests
prove that a dropped renderer reply still invokes the native shutdown callback
once, including when the post-spawn atomic save fails before or after replacement.
Failed spawn never invokes shutdown, even though handoff intent is already durable.
Windows handoff state is fixture-tested;
actual NSIS/MSI installation, UAC, locking and durability remain native Windows gates.
These checks do not prove packaged Tauri IPC/layout/exit, production HTTPS,
replacement of the actual launcher, application health or production restart. Production key custody, updater-compatible CLI signing, release
publication, OS signing/notarization and release ordering remain separate gates.

## Settings integration

Settings provides Check, Download, Apply and restart, and Recheck controls. The application owns all
views for its lifetime; native operation-revision changes refresh updater
capabilities, so a completed game/setup operation does not strand an old offer
behind a stale revision. No update mutation is replayed automatically. The
composition UAT mounts the real views, completes a native fixture journal
operation while an offer is displayed, then downloads once using the refreshed
revision. Non-updater view payloads in that UAT remain inert fixtures.

## Native Apply and recovery

The running executable selects its installed target. A Mac target must be the
executable named by its enclosing bundle's `Contents/Info.plist`. Apply checks the
incoming bundle identifier, executable name and offered version before touching
that target. Tar extraction accepts one `.app` root, bounded entries/expanded
bytes, ordinary files/directories and contained relative framework symlinks;
unsafe paths, duplicate/case aliases, hard links and special files are rejected.
The stage is published beside the installed bundle by exclusive rename with an
operation-owner marker. Staging is on the target volume even when app data is on
another volume; there is no privileged or cross-volume copy fallback.

Apply records `installing` before filesystem effects, fingerprints original and
replacement trees with explicit directory/entry framing, atomically exchanges the installed and staged bundles with `RENAME_SWAP`, then
moves the swapped-out original to a unique sibling backup. The installed path
remains launchable if the process crashes between those steps.
A final rename or replacement-spawn failure restores a recognized original.
Unknown/colliding stage, target or backup contents are preserved and retain
reconciliation ownership. Cleanup never adopts a directory just because it has
the expected name. A pre-handoff interruption restores recognized old bytes.

After successful spawn, Apply attempts to persist `restart_required`; it does not
claim completion. Native shutdown belongs to the retained Apply worker and uses
an explicit successful-spawn notification, independent of the final save and IPC
response. A failure saving the restart state still shuts down the old process so
the replacement can acquire the state lock. The error remains an error, durable
ownership blocks duplicate Apply, and reopening requires reconciliation. Durable
handoff intent alone never triggers shutdown: it is written before spawn, which
can still fail. A Mac replacement receives a fixed restart
argument and waits up to 30 seconds for the old state owner to exit. No arbitrary
process is killed. On reopen, only the current executable's compiled version and
native target can acknowledge the expected release. Mac acknowledgment also
checks the replacement tree fingerprint. `installed` is persisted before
best-effort backup cleanup, so a crash during garbage collection cannot undo a
valid startup acknowledgment. This proves version startup, not app health.

Windows uses the signed `.exe` or `.msi` asset with native-only installer flags.
NSIS uses passive update/restart flags and the current executable's directory;
MSI uses the system `msiexec` path and launch-after-install properties.
`ShellExecuteExW` supports normal Windows elevation and detects rejected launch.
Installer handoff remains pending until the new compiled launcher starts. An old
version reopening cannot infer that the installer was cancelled or completed;
it preserves the owner and blocks game/setup changes. Real installer cancellation,
reboot, elevation and package upgrade identity must be exercised before release.

`frontend/updater-apply-native-uat.mjs` drives the production Effect/view through
an isolated native bundle/process fixture. It covers duplicate Apply suppression,
real replacement, persisted reopen, game exclusion and a fixture-supplied compiled
version acknowledgment. It does not replace packaged visual/IPC UAT or a real
old-to-new signed launcher upgrade.
