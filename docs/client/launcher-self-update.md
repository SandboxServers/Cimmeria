# Launcher self-update

> **Type:** Reference
> **Audience:** Launcher contributors and release maintainers
> **Last updated:** 2026-10-04
> **Companions:** [Launcher design](sgw-launcher.md#self-update-srcself_update), [player guide](launcher-guide.md#launcher-updates), [distribution setup](launcher-distribution-setup.md)

This describes the existing Windows launcher's updater. Desktop-launcher updater
migration remains a separate integration gate.

## Contract

The launcher replaces itself with a newer launcher release. Maintainer
decisions of 2026-09-29: discovery and trust through GitHub Releases, a
one-click prompt, and an optional `min_launcher` gate in the manifest.
Player-facing behaviour: [launcher-guide.md](launcher-guide.md#launcher-updates).

**Identity.** The release workflow stamps the tag
(`launcher-YYYYMMDD-<sha7>`) and the job's start time (Unix seconds)
before it builds, and passes them to the launcher build as
`CIMMERIA_LAUNCHER_TAG` and `CIMMERIA_LAUNCHER_BUILD_EPOCH`
(`option_env!`, `build_info.rs`). A build missing either, or with a
malformed value, is a development build: it makes no request and offers
nothing. `build.sh verify` fails a release build that does not embed its
tag.

**Discovery.** One unauthenticated `GET
api.github.com/repos/SandboxServers/Cimmeria/releases?per_page=100` per
start (and per **Check for updates**), with a `sgw-launcher/<tag>`
User-Agent. `/releases/latest` is not enough: server releases (`v2026-…`)
and the `content-current` prerelease share the repository. Candidates are
non-draft, non-prerelease `launcher-*` releases that carry both
`sgw-launcher-<tag>.exe` and `sgw-launcher-<tag>.exe.sha256`; the newest
by `published_at` wins. 403 with `x-ratelimit-remaining: 0` and 429 are
reported as rate limiting, connection failures as offline; both are one
status-log line.

**Ordering (never downgrade).** A release is newer than the running
build when the tags differ, the release's tag date is not earlier, and
it was published after the running build's stamped start time. The last
rule orders two releases cut on one day. It is sound because the
`launcher-release` concurrency group runs one release job at a time and
each job publishes at its end: every later release is published after
this build started, every earlier one before. The rule and its tests
are in `version.rs`.

**`min_launcher`.** An optional top-level manifest field, a launcher tag
(no schema bump). A different day decides by date; the same day by the
required release's `published_at`, looked up in the last release list,
and the player is let through when it is not known. Malformed values are
ignored with a WARN; development builds are exempt. When it blocks,
Install / Update and every launch button are off and the banner is
mandatory.

**Download and verify.** The download goes to
`.sgw-launcher-update-<tag>.exe.part` beside the running exe, through
the install pipeline's Range-resuming downloader. It must match the size
the release lists and the SHA-256 in the `.sha256` asset (sha256sum
format; a file name in it must be the exe's) before anything else
happens; a mismatch deletes it. The updater's HTTP client is https-only
and follows redirects only to `github.com`,
`objects.githubusercontent.com` and
`release-assets.githubusercontent.com`; asset URLs are checked against
the same list before the first request. A directory the launcher cannot
write (a `Program Files` install) stops the update before any download,
and the banner links the release page.

**Swap.** Windows lets a running image be renamed but not replaced:

1. delete a leftover `<exe>.old`;
2. rename the running `<exe>` to `<exe>.old`;
3. rename the verified download to `<exe>`; if that fails, rename
   `<exe>.old` back;
4. start `<exe>` with the same arguments, `SGW_LAUNCHER_UPDATED_FROM`
   (the old tag) and `SGW_LAUNCHER_UPDATED_FROM_PID` (the old pid) set;
   if it will not start, delete it, rename `<exe>.old` back and keep
   running with the lock still held;
5. release `launcher.lock` and exit the process at once, from the worker
   thread (`self_update/handoff.rs`).

Step 5 does not wait for the window. `launcher-20260929-676f314` closed
its window instead, which in egui only happens on the next frame; with
no mouse input that frame never came, the old launcher kept the lock,
and the new one gave up with "another instance appears to be running".
So updating **from** `launcher-20260929-676f314` may show that box once
(the observed 676f314 to 4fcae33 update did); the update itself has
worked, and clicking OK and starting the launcher again finishes it.
Later launchers do not. Nothing touches install state between releasing the lock and exiting,
and the launcher saves nothing on exit (config and telemetry state are
written when they change), so the hard exit loses nothing.

Every path comes from `std::env::current_exe`, so a renamed launcher
stays renamed. The relaunched process sees `SGW_LAUNCHER_UPDATED_FROM`
and waits up to 10 s for the lock instead of failing with "another
instance". If the old launcher still holds it (an older launcher waiting
for its frame), the new one ends that process, the pid from
`SGW_LAUNCHER_UPDATED_FROM_PID` or, from launchers that predate it, its
parent process, but only when that process's image is this exe or
`<exe>.old`, then waits up to 10 s more. If that fails too, it says the
previous launcher window is still open rather than "another instance". A
normal start, without the variable, still refuses a second launcher at
once. The new process then deletes `<exe>.old` in the background,
retrying while the old process exits (the next start retries if it stays
locked).

**Telemetry.** Every step logs on target `launcher.update` with an
`event` field: `update_check` (`outcome` = `dev_build`, `no_release`,
`up_to_date`, `available`), `update_check_failed`, `release_skipped`
(DEBUG), `update_download_started`, `update_verified`, `update_swapped`,
`update_handoff_started` (`old_pid`), `update_relaunched` (the new
`pid`), `update_lock_released`, `update_rollback`, `update_failed`; in
the new process `update_relaunch_lock_acquired` (`waited_ms`, `killed`),
`update_relaunch_lock_timeout`, `update_fallback_kill` (`outcome` =
`killed` or `skipped`), `update_relaunch_lock_failed`,
`update_started_new`, `update_old_removed`, `update_old_remove_failed`.
Refusals and failures carry `reason` (`rate_limited`, `offline`,
`http_status`, `parse_failed`, `dir_not_writable`, `untrusted_url`,
`bad_checksum_file`, `http_failed`, `io_failed`, `size_mismatch`,
`sha256_mismatch`, `stale_old_locked`, `move_aside_failed`,
`install_failed_rolled_back`, `rollback_failed`, `relaunch_failed`,
`lock_not_held`, `old_launcher_still_running`, `lock_held_after_wait`,
`no_old_pid`, `own_pid`, `open_failed`, `not_our_exe`, `image_unknown`,
`terminate_failed`, `no_exe_path`, `unsupported_platform`,
`held_after_kill`, `still_locked`).

**Tests.** `self_update/` pins the ordering (same-day, same tag, no
downgrade, dev builds), the release filter over a realistic releases
fixture (`testdata/releases.json`), size and SHA-256 mismatches deleting
the download, the host allow-list and redirects, the swap and its
rollbacks in a temp directory, `.old` cleanup (including a locked file),
the relaunch lock wait, the handoff order against a real file lock
(start, then release, then exit, with no UI involved; a failed start
keeps the lock), a relaunch that outlives the first wait and ends the
old launcher, a normal start still refusing a second instance, ending a
real process running our exe and leaving one that is not, rate limiting
and offline against a loopback
stub, and `min_launcher` parsing and gating. `app/update_banner.rs` pins
the banner state machine.

Not done: a code-signed exe (deferred with the rest of launcher signing)
and a signed update manifest (trust is HTTPS plus GitHub, as for a manual
download). Only the first 100 releases are scanned, so a launcher release
buried under more than 100 newer server releases is not seen.

---

