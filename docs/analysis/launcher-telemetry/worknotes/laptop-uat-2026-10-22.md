# Laptop UAT: light in-place adoption (2026-10-22)

The owner-UAT row "light adoption of the laptop install" was run on the second laptop with launcher `desktop-2026.10.22` (PR LA #1983), folder `C:\Temp\sgw`. **Result: FAIL.** The adoption bricked the folder half-way. The evidence is the user's three screenshots plus SigNoz (`service.name = cimmeria-client`, `cimmeria.session_kind = launcher`, `install_id 75a61e68-…`, session `f3157d4f-…`).

## Timeline (SigNoz, UTC)

| Time | Event |
|---|---|
| 13:10:08 | Refused `unsafe_file`, reason `name`: `Working/binaries/wxRes/TerrainEdit_Gradient másolata.bmp` |
| 13:10:46 | Mode `in_place` chosen again, after the user dealt with the file |
| 13:12:27 | Light plan: 12 of 13 patches pending, 283,778 bytes |
| 13:12:28 | Index compared: stock 5975, patched 4, prepared 2, changed 5, missing 119, extra 1593 |
| 13:12:28 | Replayed 12 patches, 17 replacements, 0 ops skipped |
| 13:13:35 | Confirmed |
| 13:16:48 | ERROR `in-place adoption: the owned tree does not match the signed reference` → `Storage(Corrupt)` → step `finished`, `corrupt_state` |
| 13:21:33 | Same ERROR again (Recover, or the restart re-check). Recovery cannot succeed. |

## Findings

### F1 (blocker): light in-place adoption fails at its final digest check and cannot be recovered

`in_place.rs::finish` moves the files (phase `Promoted`), runs `client_setup::prepare` and saves the ledger. It then calls `validate()`, which scans `game/` and requires its `content_digest` to equal `provenance.reference_digest`. For a light preview, that digest is `light.rs::light_comparison` → `published_digest(tree, …)`: a *predicted* tree built from the scan, the replacements, the cooked-tier placement, `restore_stock_case` and the simulated `apply_setup`. If any prediction misses by one byte, the folder ends up `Promoted` with `Corrupt`:
- Abandon is not offered after `Promoted`, by design.
- Recover reruns the same `validate()` and fails the same way (13:21:33).
- The user's folder is now split: the release files are in `C:\Temp\sgw\game\` and the extras sit at the old place. No path out exists in the UI.

This contradicts D-LA9 ("light adoption never blocks"). Candidate causes, **none verified**:
- the cooked-tier rename (`place`);
- the case restore against the real `client_setup::prepare` rename;
- `apply_setup` against the real setup (ASLR flag, LoginInternal.lua, `launcher-installed.json`);
- the ledger bytes with `incomplete` patches;
- extras (1593) moved into `game/` that the tree did or did not expect.

**Telemetry gap (rule: debuggable from SigNoz alone).** The mismatch logs no path. On mismatch, log counts plus capped samples of three kinds: expected but absent, present but not expected, and hash differs. `plan` already carries `expected_paths`.

**Design fix options (needs a decision):**
- (a) Validate the predicted tree before any move: dry-run setup in the prep folder, then compare.
- (b) For `FileCheck::Light`, publish when the digest differs, and record the diff in provenance.
- (c) Offer Abandon after `Promoted` for light adoptions: put the files back, and restore setup files from the stored copies.

Recommend (a) plus the telemetry first, and (c) as the escape hatch. A regression test that drives a light preview through `finish` on a fixture with a non-trivial setup and cooked-tier placement must fail today.

### F2 (high): a stock client file is refused by the ASCII name rule

`Working/Binaries/wxRes/TerrainEdit_Gradient másolata.bmp` ships with the client; it is in the QA client tree. With the `name` rule, every stock in-place adoption is refused until the player renames or deletes a game file. Options:
- exempt paths the published index lists;
- treat non-ASCII names in non-release paths as extras left in place;
- allow NFC UTF-8 names that round-trip on NTFS.

### F3 (high): contradictory status after the interruption (screenshot 3)

- Adoption panel: "Recover to finish moving…", with a Recover adoption button.
- Install panel (`install-view.ts:371`): "An interrupted compatibility operation was found. Files are preserved; recovery is not available in this build." This is wrong on both counts: it was an adoption, and recovery is offered.
- Play panel: "Resolve any folder-access prompt, then recheck status. Play was not retried." This is the generic `s.error` text in `launch-view.ts:8`, and no folder-access prompt was involved.
- There are three recheck buttons ("Recheck adoption status", "Recheck Play status", "Recheck status") and an enabled-looking **Install Stargate Worlds** primary while an adoption owns the folder. Pressing it risks a second download into the same place.
- Fix: one interrupted-operation banner keyed on `operation.kind == adopt`. Install, Play and their texts defer to it.

### F4 (medium): review-screen copy (screenshot 1)

- Paths show the verbatim prefix `\\?\C:\Temp\sgw`. Strip `\\?\` for display.
- For in-place adoption, "Game folder to adopt" and "Launcher-owned installation" repeat the same path. Show one line.
- The full 64-hex manifest digest is shown, and "patches in signed order" starts at 008 and includes `bm-ui-overlay-17d66e08e3f9`. The Downloads line shows the internal key `…@sgw_game`. Show friendly patch names, and put the hashes behind a details toggle.
- "Stargate Worlds found in sgw." reads oddly.
- Four count summaries don't reconcile for a player:
  - 7564 files move;
  - 17 updated;
  - 1 of 13 patches applied;
  - 5975 / 4 / 2 / 5 / 119 / 1593 against the index.
- "▶ Show the files" shows a closed marker while the list is expanded.
- "Another operation owns the game. Wait for it to finish." appears while only the review is open. Nothing is running, so it should say that an adoption review is open.

### F5 (low): the review's telemetry line (screenshot 2)

- A missing space: "nothing imported.This desktop build" (`adoption-view.ts`, the `telemetry:` line; still on `origin/main`).
- "This desktop build cannot send game telemetry" sits beside launcher diagnostics that *did* reach SigNoz in this run. Say which telemetry is meant.
- The consequences text is clipped at the right edge, and a small duplicate of the panel ("Use an exist…") overlaps the bottom-right corner. Unverified: it may be a Windows thumbnail rather than launcher UI.

### F6 (low): maintenance controls

- "Recover adoption…" is greyed out while its own confirmation panel is open. The press was acknowledged, but the control looks dead.
- "Recover adoption" and "Go back" have no gap between them.

## Laptop state and owner steps

`C:\Temp\sgw` is half-adopted. Recover will keep failing until F1 is fixed. If it is a scratch copy, close the launcher, delete the folder and the launcher's adoption state, then re-copy. The launcher log in `<app data>\logs\` on the laptop, and the adoption record in the launcher state folder, would show which files differ. Ask the user for them before guessing at F1's cause.

## Follow-up: why SigNoz could not answer this (2026-10-22, later)

- **G1.** `in_place.rs::validate()` hashes the whole `game/` tree, compares one digest, and drops the scan. It never computes which files differ (F1).
- **G2.** Launcher telemetry carries only the fixed event list and fields. That is correct, and it does not block a mismatch event with relative paths: `changed_sample` already carries them.
- **G3.** **Upload debug logs** refuses with `busy` while an operation needs reconciliation, which is the state where it is needed most.
- **G4.** No release has log upload. `LAUNCHER_LOG_SAS_URL` is neither a repo secret nor a `launcher-release` environment secret. Run 39781783769 (2026-10-22) built Windows and macOS with it empty. The Settings panel then hides the button and leaves an empty "Debug logs" section (user photo, laptop), which contradicts `docs/client/sgw-launcher.md` ~510 (greyed button plus note).

## Proposal (user direction): log upload ingests into SigNoz, not into a blob

User: "We should be sending these logs to signoz on upload. Not the whole file but ingesting it in."

Reuse the launcher session (AU-11..15): `/api/auth/dev-session` kind `launcher` → `/api/telemetry/upload-chunk` → `launcher_rows.rs` filter and scrub → `cimmeria-client`, `session_kind = launcher`.

1. **New targets:** `client.launcher.log` (the launcher's own rolling log in `<app data>/logs/`) and `client.launcher.game_log` (the game's debug and session logs the zip collects today). They go in `launcher_targets.txt`, `LAUNCHER_TARGETS` and the golden tests together (D-LT5).
2. **One row per line:** level, original target, message, timestamp, and the parsed scalar fields. The server keeps the R12 caps (16 fields, 256-byte values) and the R14 scrub, so absolute paths, IPs and accounts are removed server-side as now. Multi-line entries are folded into one row, and over-long messages are truncated with a marker.
3. **`upload_id` on every row,** shown in the UI after the press ("Sent 3,412 log lines. Reference 7f3a…"). One SigNoz filter then pulls the whole upload, and the player can quote the reference.
4. **Incremental:** the `uploaded-logs.json` ledger changes from file digests to per-file byte offsets, so a second press sends only new lines.
5. **Budget:** a per-upload line cap (proposal: 20,000 lines, newest first), plus a server-side per-install daily budget for the two log targets, separate from the event budget (AU-15 pattern).
6. **Runs in any state:** reading logs is read-only, so it is allowed during reconciliation (fixes G3). The adoption record is not uploaded as a file. A failed check instead emits the F1 mismatch event: counts plus capped relative-path samples with expected and actual size and hash.
7. **No build-time secret:** the button shows whenever the launcher can reach the server (fixes G4). The `LAUNCHER_LOG_SAS_URL` blob path is retired, or kept only for crash dumps (decision D-LU2).

Decisions needed:
- **D-LU1, consent.** Does the explicit button press consent for that one upload when "Send game diagnostics" is off? Proposed: yes, with the panel text saying what is sent.
- **D-LU2.** Retire the blob path, or keep it for crash dumps.
- **D-LU3.** Game debug logs can hold chat and character names. The R14 scrub covers IPs, accounts and paths, not free-text chat. Options: send launcher logs only in v1; or add a chat-line drop rule for `client.launcher.game_log`.
- **Security re-review against R1-R16.** The launcher token's row volume and its field shape both widen.