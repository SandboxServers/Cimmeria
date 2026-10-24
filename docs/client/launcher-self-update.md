# Launcher self-update

> **Type:** Reference
> **Audience:** Launcher contributors and release maintainers
> **Last updated:** 2026-10-23
> **Companions:** [Desktop updater contract](../../crates/launcher/desktop/docs/updater.md) (the full contract), [release guide](../../crates/launcher/desktop/docs/release.md#the-signed-feed), [player guide](launcher-guide.md#launcher-updates), [distribution setup](launcher-distribution-setup.md#part-3--launcher-updater-signing-key)

How the desktop launcher replaces itself with a newer release. This page is the
summary; the [updater contract](../../crates/launcher/desktop/docs/updater.md)
beside the code is the source of truth for every limit and recovery rule, and
wins where the two differ.

The decisions behind it are in the
[launcher consolidation ledger](../analysis/launcher-consolidation/README.md):
the updater swaps the release zip in place, with no installer (D-LX3); releases
publish a rolling `launcher-current` release beside the dated ones (D-LX7); the
compiled version is SemVer from the release date and daily slot (D-LX15); and a Minisign key
pair signs the feed (D-LX9).

## Contract

**Identity.** A release build uses stable `YYYY.M.(D*100+N)` from UTC,
with daily slots 1..99 (`2026.10.2301`, `2026.10.2302`, ...), stamped as
`CIMMERIA_LAUNCHER_VERSION`. Slot 100 is refused. Every release is signed
and offered; missing signing configuration blocks publication. The same value is the exe's file version, the About
line in Settings and what the updater compares. A build without it (any local or
PR build) is `0.1.0`. See [release: Version and tag](../../crates/launcher/desktop/docs/release.md#version-and-tag).

**Configuration.** On Windows the feed URL is compiled in as the
`launcher-current` release's `latest.json`, and the Minisign public key comes
from the repository variable `LAUNCHER_UPDATER_PUBKEY` at compile time. A build
without the key shows **Disabled** and makes no request. Every macOS build also
shows **Disabled**: the feed offers only the two Windows keys, and the shell enables
the updater on Windows only.

**Feed.** `latest.json` is Tauri v2's static format: `version`, optional
`notes`, `pub_date` and a `platforms` map whose `windows-x86_64` and `windows-aarch64` entries each hold the
zip's URL and the base64 of its Minisign signature document. One feed carries
both keys. A launcher reads only its own: an arm64 launcher updates to the arm64
zip and an x64 launcher to the x64 zip, including an x64 launcher running on a
Windows on Arm PC, which has no automatic cross-grade (D-WA3). The URL names the
zip on the **dated** release, not on `launcher-current`, because a dated
release's assets never change, so a feed read mid-refresh still points at the
bytes its signature covers.

**Ordering (never downgrade).** An offer must be stable SemVer with greater
precedence than the compiled version; prerelease offers are refused. An
installed same-day rebuild counts as its base version, so `2026.10.11-2` is
offered the new stable daily-slot versions, including `2026.10.1101`.
Old stable installs upgrade too; same-day slots and month/year rollovers
increase under SemVer. Build metadata cannot increase precedence. The
rolling feed refuses version regressions, including an older run rerun.
Delayed older macOS retries may repair their dated download, but preserve
the newer rolling macOS assets and release notes. Equal-version retries can
refresh them; the workflow summary distinguishes these outcomes.

**Trust.** The launcher verifies the zip's Minisign signature and the global
signature with `minisign-verify` before it reads the trusted comment, which must
carry exactly one `version:` field equal to the announced version. An old signed
zip relabelled as a new release fails. Transport is HTTPS on port 443 to an
exact host allowlist, with at most four checked redirects and bounded sizes and
deadlines. The release job signs with the same engine code that verifies
(`engine/examples/release_feed.rs`), so a feed a launcher would refuse fails the
release before anything is published.

**Player steps.** The main page checks on startup and holds Play until the check
finishes. When an update is known, **Update and restart** downloads, verifies and
applies it in one press; Play stays blocked until the replacement starts and
acknowledges its version. A saved **Ready** package skips the download. Startup
does not download or apply anything automatically. Settings retains **Check for
updates**, **Download update**, **Apply and restart** and recheck controls for
recovery. Updates refuse while any game or setup operation runs, Play included.

**Check failures.** A failed recheck retains an existing offer and keeps Play
blocked. If no update is known, a failed startup check permits Play after the
bounded request finishes and displays failure/retry feedback. Disabled builds
remain subject only to their other Play gates. The [desktop contract](../../crates/launcher/desktop/docs/updater.md#startup-and-main-page-integration)
defines the startup sequence and native admission rules.

**Swap (Windows, D-LX3).** Apply checks every zip entry before writing anything,
extracts into a per-attempt stage beside the exe, saves the plan, then for each
file renames the installed copy to `<name>.old` and renames the new one in, the
executable last. Windows lets a running exe be renamed but not overwritten. It
then starts the new exe with `--launcher-update-restart <old pid>` and exits.
The new process waits up to 30 seconds for the old pid, takes the single-instance
guard, and acknowledges the update only when its compiled version equals the
offered one; then it deletes the `.old` files and the stage.

**Single-executable Windows packages.** New Windows ZIPs keep the public asset
name and contain one root launcher EXE with its three Play pieces embedded.
The existing swap accepts that package when updating an older multi-file
installation; older loose sidecars are left alone. On replacement startup the
new executable verifies or repairs its app-local
`payloads/<content-digest>/windows/` cache. Active versions hold shared leases
and are skipped by cleanup. Cache contents come from trusted embedded bytes,
not from an old installation's sidecars. These files are separate from the
updater's replacement stage and game content. See the
[native resource contract](../../crates/launcher/desktop/docs/launch.md#embedded-windows-play-resources).
The feed, ZIP signature, restart handoff and rollback rules stay the same.
macOS continues to distribute its existing app bundle and uses manual updates.

**Rollback.** If a rename or the relaunch fails, Apply puts every `.old` file
back and reports **Failed**; the installed files are restored byte for byte. A
process that dies mid-swap is rolled back from the saved plan the next time the
old launcher opens. Two windows need a hand fix, because nothing is left to
reopen: a crash between renaming the exe away and renaming the new one in, and a
new release that crashes at start. Both are recovered by renaming
`<name>.exe.old` back; the [player guide](launcher-guide.md#a-launcher-update-failed-or-the-launcher-is-gone-after-an-update)
says how.

**`min_launcher`.** The content manifest's optional `min_launcher` is a separate
gate: a launcher older than the named release tag refuses to play that release
and tells the player to update. It is described with the manifest in the
[launcher guide](launcher-guide.md#minimum-launcher-version-min_launcher).

**Keys.** Rotating the updater key strands every launcher built with the old
public key: it can no longer verify a release and never updates again, so its
players replace it once by hand. Launchers built before the key existed are in
the same position. See [distribution setup](launcher-distribution-setup.md#part-3--launcher-updater-signing-key).

## The retired egui updater

The egui launcher (`sgw-launcher.exe`, retired in LX-18) had its own updater: it
scanned the GitHub releases API for `launcher-*` releases carrying
`sgw-launcher-<tag>.exe` and its `.sha256`, verified the SHA-256 and swapped the
single exe. No release publishes that asset any more, so installed copies are
never offered an update, and `launcher-current` has no date in its tag, which
that updater skips. Players move to the zip by hand once (D-LX2, the
[announcement](../analysis/launcher-consolidation/announcement.md)). Its design
is kept in the archived [sgw-launcher.md](sgw-launcher.md).

## Tests

The engine's updater tests sign ephemeral local feeds and cover tampered
payloads and staging, old artifacts with inflated versions, altered trusted
comments, missing and duplicate signed versions, host and redirect policy, size
and deadline failures, stale revisions and offers, cross-operation exclusion and
restart recovery. The Windows zip swap is tested against temporary install
folders: a full swap, a rename failure injected mid-swap restored byte for byte,
a failed relaunch restored, unsafe zip entries refused before any write, and
startup cleanup of `.old` files. `npm run uat:updater` and
`npm run uat:updater-apply` drive the production Effect workflow against native
fixtures. The release workflow's PR dry run signs and verifies a feed with a
throwaway key.

Not proven yet: replacing a real running launcher from release N-1 to N, with
antivirus and the single-instance guard (LX-26 step 11 of the
[acceptance gates](../analysis/launcher-consolidation/acceptance.md)), and any
macOS self-update, which is a follow-up packet.
