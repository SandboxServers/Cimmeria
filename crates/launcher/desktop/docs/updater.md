# Signed launcher updates

> Reference · desktop launcher maintainers · 2026-10-11

The desktop launcher can check a native-configured release feed, download a
package, and verify its Minisign signature and signed version before saving it.
Apply re-verifies saved bytes and can replace a native Mac application bundle or
swap the Windows launcher's files in place from the release zip. The new compiled
launcher version must acknowledge startup before an update is reported installed.
On Windows the feed is compiled in as the `launcher-current` release's
`latest.json` and the Minisign public key comes from `LAUNCHER_UPDATER_PUBKEY` at
compile time; a build without the key, and every macOS build, shows **Disabled**. Game-manifest signatures and legacy launcher
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
An installed prerelease counts as its base version: a same-day rebuild
`2026.10.11-2` ([release](release.md#version-and-tag)) is not offered
`2026.10.11` or anything older, which SemVer alone would order above it, and is
offered `2026.10.12`.

Verification follows [Tauri updater v2.13.1's pinned implementation](https://github.com/tauri-apps/plugins-workspace/blob/e51128438011755f9e7277bad29b8c0978cf281c/plugins/updater/src/updater.rs):
`minisign-verify` 0.2.4 verifies both artifact and global signatures before reading
the trusted comment. Exactly one tab-separated `version:` field must equal the
announced SemVer. Thus an old signed artifact relabeled as a new release fails.
The implementation rejects duplicate version fields, more strictly than Tauri's
first-match parser. The release workflow signs with this field and checks the
result with the engine's own verification before publishing (LX-15,
[release](release.md#the-signed-feed)).

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
summary consent remains editable. Log upload and client maintenance go through
`ensure_no_active_operation()`, which calls `ensure_updater_idle()` and also
refuses while any operation, Play included, is nonterminal. Client-option saves
call `ensure_updater_idle()` and refuse any nonterminal operation except Play,
which reads them only at launch. New mutation paths must preserve this rule. A dropped
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
The Windows zip swap is tested against temporary install folders: a full swap,
a rename failure injected mid-swap restored byte for byte, a failed relaunch
restored, unsafe zip entries refused before any write, and startup cleanup of
`.old` files. Replacing the real running launcher, antivirus locks and the
pid wait before the single-instance guard remain native Windows gates (LX-26).
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
can still fail. The replacement, Mac or Windows, receives`
`--launcher-update-restart <old pid>`, waits up to 30 seconds for that process to
exit, then registers the single-instance guard; opening the state lock also
retries for 30 seconds. No arbitrary process is killed. On reopen, only the current executable's compiled version and
native target can acknowledge the expected release. Mac acknowledgment also
checks the replacement tree fingerprint. `installed` is persisted before
best-effort backup cleanup, so a crash during garbage collection cannot undo a
valid startup acknowledgment. This proves version startup, not app health.

### Windows: zip swap (D-LX3)

On Windows the package is the release zip, `StargateWorlds-Launcher-windows-x64.zip`:
one root `.exe` with `windows/` and `graphics/` beside it. There is no installer;
an offer whose URL is not a `.zip` is refused. After the Minisign and signed
version checks, Apply:

1. Validates every zip entry before writing anything. It refuses `..`, absolute
   and drive-letter paths, backslash separators, trailing dots or spaces, device
   names, links, duplicate or case-aliased names, a file that is also a folder,
   names ending in `.old`, 8.3 short names (`~` then a digit), `CONIN$` and
   `CONOUT$`, more than one root `.exe`, and archives past the entry and
   expanded-size bounds.
2. Extracts into `<exe dir>\.update-<version>-<attempt>\`, created exclusively
   with an owner marker. An existing stage is never adopted. Because the name
   is per attempt, a stage a failed attempt could not delete (antivirus holding
   a new `.exe`) never blocks a retry; Apply and every start remove any marked
   stage that is not the in-flight attempt's.
3. Checks every destination: absent or a plain file, under plain folders (no links
   or junctions). A leftover `<name>.old` from an earlier update is deleted now, or
   Apply stops with nothing changed.
4. Saves the plan (each file, and whether it replaces one) before the first rename.
5. For each file in turn, renames the installed copy to `<name>.old` and renames
   the new copy in. Windows lets a running executable be renamed but not
   overwritten. The executable moves last, so a launcher that starts from the
   installed path with the new compiled version proves every other file moved.
   The root `.exe` is installed under the running executable's name, whatever
   the player called it.
6. Starts the new executable with `--launcher-update-restart <old pid>` through
   `CreateProcess`, then exits. Nothing is elevated, and a missing executable
   fails synchronously, so no shell error dialog can stall the handoff.

If any rename or the relaunch fails, Apply moves each new file off its name to
`<name>.new-<attempt>` (a loaded file can be renamed but not deleted), renames
each `.old` back over its name, deletes the moved-aside new files where it can,
removes the folders the update created once empty, and removes the stage: the installed files are restored byte for byte and the
status is **Failed**. A rollback step that cannot complete keeps reconciliation
ownership. A process that dies mid-swap is rolled back the same way, from the
saved plan, the next time the old launcher opens. Files in the install folder
that the zip does not name are left alone.

Known limits, both recovered by hand:

- Between renaming `<name>.exe` to `<name>.exe.old` and renaming the new one in,
  no launcher executable exists at its name. A crash or power loss in that
  window leaves nothing to double-click, so nothing reopens to roll back. Rename
  `<name>.exe.old` back to `<name>.exe`; the next start rolls the rest back from
  the saved plan.
- A new release that crashes at start cannot roll itself back: the old
  executable is already `<name>.exe.old` and only the new version acknowledges.
  Delete or rename the broken `<name>.exe`, then rename `<name>.exe.old` back to
  `<name>.exe`.

The restarted launcher waits up to 30 seconds for the old process id to exit,
then registers the single-instance guard as a normal start does, so a later
manual launch focuses its window. It then acknowledges startup as the Mac path
does: its compiled version must equal the offered version and the installed
executable must match the applied one. After `installed` is saved it deletes
the `.old` files the plan names and the stage. A file it cannot delete yet (an
antivirus scan, say) is retried on the next start and before the next update.
An old version reopening after the handoff cannot acknowledge and keeps
reconciliation ownership.

`frontend/updater-apply-native-uat.mjs` drives the production Effect/view through
an isolated native fixture: a Mac bundle exchange with a spawned fixture executable,
or on Windows a zip swap in a temporary install folder with the relaunch recorded
instead of spawned. It covers duplicate Apply suppression, real replacement,
persisted reopen, game exclusion, a fixture-supplied compiled version
acknowledgment and, on Windows, `.old` and stage cleanup. It does not replace
packaged visual/IPC UAT or a real old-to-new signed launcher upgrade.
