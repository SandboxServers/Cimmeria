# Native launcher update Apply

> Reference · launcher implementation and release owners · 2026-10-04
> Base: `a806e8b79` · isolated branch: `launcher/updater-apply`
> Feature commit: `dc97a395` · companion commit wires shared dependencies/startup
> Integration: coordinator-owned; this packet does not publish or replace a live app.

The desktop updater now has a native Apply path after signed package preparation.
It re-verifies saved artifact bytes/version immediately before effect, resolves
the installed target from the current executable, and records installation intent
before changing files. The Settings action has no path, URL, key, bytes or command
arguments in its renderer contract. Production endpoint/key configuration remains
absent, so the action stays unavailable in ordinary development builds.

## Delivered behavior

Mac application tarballs are bounded and validated before target mutation. A
single bundle with matching identifier, executable name and version is staged on
the installed bundle's volume. An owner-marked stage is published exclusively;
original and replacement contents have framed tree fingerprints. An atomic Mac bundle exchange keeps the installed path launchable; the swapped-out
original is then moved to its backup name. Final-rename and spawn
failures restore recognized original bytes. Unknown/colliding contents are left
alone with reconciliation ownership retained.

Windows signed EXE/MSI packages are staged natively and handed to the platform
shell, with a system-resolved MSI executable and fixed native installer arguments.
A successful process handoff is pending, never an installed claim. The new
compiled launcher must reopen at the same native target and acknowledge its
expected version. Older reopen retains ownership because the installer may still
be active. Native shutdown is part of the retained Apply task, independent of the
renderer promise. The new Mac process can wait for the old state owner to exit.

The implementation reference is
[`crates/launcher/desktop/docs/updater.md`](../../../../../crates/launcher/desktop/docs/updater.md).
No production key, release endpoint, signing secret, updater feed or live app was
created or modified. Game/framerate state was untouched.

## Verification

Native temp-filesystem/process tests cover real Mac bundle replacement and child
execution, final-rename fault rollback, spawn-failure rollback, interrupted old
rename recovery, tampered saved bytes, duplicate/stale ownership, wrong target,
foreign stage/installer preservation, and tree-fingerprint structural ambiguity.
Archive tests reject duplicate/case aliases, extra bundles, escaping links,
hardlinks and special entries; contained framework symlinks remain supported.
Shell tests cover schema/input rejection and dropped-reply shutdown exactly once,
with no shutdown after failed spawn. Frontend tests cover lost Apply replies
without mutation replay.

Both native Effect UAT scripts exercise disk-backed state. The download script
retains real signed local HTTP coverage. The Apply script invokes actual temp
bundle replacement and a fixture child process, checks that game mutations stay
blocked through reopen, and supplies a simulated new compiled version for the
acknowledgment boundary. Its acknowledgment is not a second compiled production
binary and is not evidence of packaged launcher health.

Actual macOS Tauri IPC/layout/exit, signed old-to-new packaged application restart,
unwritable/cross-volume installation locations, Windows native installer/UAC/
cancellation/reboot, clean-machine tests, OS signing/notarization and production
release/key provisioning remain gates. Download partial resume remains absent.
Coordinator indexes and acceptance checklist must retain these limits.

### Local results

- Full standalone desktop engine/shell library and binary suite through the
  build lane: **410 passed, 23 ignored**, job `20261004-124608-32169`.
  Ignored native UAT bridges were exercised separately by their drivers.
- Focused updater and host suite: **27 passed, 2 ignored**,
  job `20261004-124507-30967`.
- Strict engine/shell all-target Clippy: passed,
  job `20261004-124542-31727`; formatting and diff checks passed.
- Frontend tests: **55 passed**; TypeScript/build passed.
- `uat:updater` and `uat:updater-apply`: passed against the native temporary-state
  fixture binary. No real application bundle or game process was launched.

Coordinator review identified and this packet corrected three issues: shutdown
formerly depended on response delivery; the initial tree hash lacked directory
framing; and recovery treated a derived stage pathname as ownership. Regressions
exercise the actual ambiguous trees, foreign collision bytes and dropped native
reply. A separate self-review replaced the two-step publication gap with atomic
Mac exchange, so a crash cannot remove the visible installed application name.

Follow-up: [updater-handoff-fix.md](updater-handoff-fix.md) covers shutdown when
post-spawn persistence fails, with engine-to-host fault injection.
