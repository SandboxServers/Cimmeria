# Launcher UAT fixes (laptop light-adoption UAT, 2026-10-22)

> **Type:** Reference
> **Audience:** Launcher contributors and owner
> **Last updated:** 2026-10-23
> **Companions:** [Resume card](RESUME.md), [player guide](../../client/launcher-guide.md), [unified UAT](../../guides/unified-uat.md#launcher-uat-fixes)

Ledger for the fixes that came out of the owner's laptop UAT of light in-place adoption ([launcher-telemetry/worknotes/laptop-uat-2026-10-22.md](../launcher-telemetry/worknotes/laptop-uat-2026-10-22.md); F1-F6, G1-G4 and the log-ingest proposal). Packets: [work-packets.md](work-packets.md) (rules, contract, waves 1-2) and [work-packets-logs.md](work-packets-logs.md) (log ingest, waves 3-5, close-out). Resume card: [RESUME.md](RESUME.md). Prefix `LF`. Designed 2026-10-22 against `origin/main` `3c10f3624`, where the launcher, admin-api telemetry routes, workflows and `docs/client` match the `lt-light-integration` tip `6440be457` (PR LA #1983's content). Every packet re-greps its anchors at its own base.

## Adoption follow-up (2026-10-23)

[PR #1992](https://github.com/SandboxServers/Cimmeria/pull/1992) merged as
`16ff568b8`; its worktree is retired. Windows and macOS native CI and the
merge-queue tests passed. The owner subsequently reports successful in-place
adoption in launcher `2026.10.2302`; that report does not attest the other UAT rows.

The [approved no-relocation design](design-adopt-in-place.md) adds the main-page
adoption entry and direct-root adoption on Windows and macOS. The owner permits
explicit per-file setup/patch changes, cooked normalization, and replacement
backups at that root; bulk copying or relocation is prohibited. Schema-2 intent
and pinned plan select direct-root layout, while schema-1 successful installs
retain their layout. Interrupted legacy movers refuse recovery and offer
metadata-only abandon with bytes retained for manual inspection.

Follow-up steps AIP-U1 through AIP-U5 are mirrored in
[unified UAT](../../guides/unified-uat.md#launcher-uat-fixes). They supplement the
historical U1-U6 rows below and do not attest a laptop, lab, native macOS, or
released-build run.

| ID | Action | Expected |
|---|---|---|
| AIP-U1 | With no valid install, press main-page Adopt an install once; cancel, then try a picker error | Immediate visible feedback; the existing workflow handles cancel/error and can retry |
| AIP-U2 | Adopt an approved fixture at its selected root on each supported native platform; reopen | No new content relocation into game; unchanged matched files retain names, bytes, and file IDs; approved individual changes/backups are recorded |
| AIP-U3 | Check valid managed/adopted installs with unavailable runtime/PhysX or a launcher minimum gate | Footer adoption button stays hidden; content validity remains separate from Play readiness |
| AIP-U4 | Run Repair, Update, then selective Uninstall on an adopted fixture with an unrelated extra file | Per-file maintenance preserves extras; Uninstall retains root, extras, and original-file backups |
| AIP-U5 | Reopen interrupted schema-1 adoption and abandon; separately interrupt/abandon schema-2 adoption | Legacy recovery refuses moving and metadata-only abandon retains bytes; schema-2 abandon rolls back approved individual changes |

## CI build-time follow-up (2026-10-23)

| Scope | Design | Validation status |
|---|---|---|
| Native Rust cache reuse, cancellation of superseded PR runs, and faster release-only CLI builds | [CI speed design](design-ci-speed.md) | PR #1994 merged as `380e904`; native/package CI and review passed; launcher release `2026.10.2303` completed from that revision |

This build-only follow-up has its own PR, separate from the bundled player-facing
path-display and startup-update changes. The owner separately authorized the
release after #1994 merged; that release completed. A further release needs owner GO.

## Purpose

1. **Unbrick light adoption (F1, blocker).** A light in-place adoption must never end in an unrecoverable `Corrupt`. The final check must be right, explain itself in SigNoz, and have an escape hatch.
2. **Accept the stock client (F2).** A file that ships with the client must not stop adoption.
3. **One clear story in the UI after an interruption, and honest review copy (F3-F6).**
4. **Debuggable from SigNoz alone (G1-G4).** A failed check logs which files differ. "Upload debug logs" ingests the launcher log into SigNoz as rows (user direction, 2026-10-22), works in any state, and needs no build-time secret.

## Findings

| # | Finding | Evidence | Packet |
|---|---|---|---|
| F1 | `in_place::finish` (`in_place.rs:542`) moves the files, writes `Phase::Promoted` (`:581`), runs `client_setup::prepare` and saves the ledger, then `validate` (`:506`) compares one whole-tree digest with `plan.provenance.reference_digest`. For Light that digest is a prediction (`light.rs:218-221`). A miss is `StorageError::Corrupt`. `abandonable` is `phase == Staged` only (`in_place_abandon.rs:9`), and Recover (`publication_recovery.rs:99`) reruns `finish` and fails the same way. The claim in the UAT note holds at every step. | code read; UAT timeline 13:16:48, 13:21:33 | LF-01, LF-02, LF-03 |
| F1a | Most likely cause (not reproduced): on a case-insensitive disk, a file a patch creates takes the patch's directory spelling in the prediction (`light_replay.rs:224-227`), but the real copy lands under whatever spelling `game/` already has. `LoginInternal.lua` behaves the same way when the folder lacks it (`light.rs:332-337` vs `install_layout.rs:46`). A whole-tree digest over spelled keys cannot survive that. | code read | LF-02 |
| F1b | A prep-folder dry run (the UAT note's option a) cannot fix F1a: the directory spellings come from the order of 7,564 in-place moves into an empty `game/`, which a dry run of setup does not reproduce. | design | D-LF1 |
| F1c | Nothing is backed up before setup after `Promoted`: `SGW.exe` is never copied before the ASLR change, and Light does not back up `LoginInternal.lua` (`in_place_replace.rs:286`, `light.rs:283`). An Abandon after `Promoted` needs new backups. | code read | LF-03 |
| F1d | The real `login_servers::write` refuses an invalid server list; the prediction's `render` does not (`light.rs:332`). That is a second way to fail after `Promoted`. | code read | LF-05 |
| F1e | `provenance.reference_digest` is read only by the two `validate` functions and the copy path (`publication.rs:282`, `:418`). Nothing downstream needs the light prediction to be exact. | grep | D-LF1 |
| F2 | `index_name_allowed` (`inventory.rs:123`, ASCII only) is applied to every scanned name (`inventory.rs:314` → `survey.rs:80-82`, reason `Name`). The stock `Working/Binaries/wxRes/TerrainEdit_Gradient másolata.bmp` fails it. The published file index can never list such a name (`file_index/mod.rs:257`), so "exempt indexed paths" cannot work. | code read | LF-04 |
| F3 | For an `adopt` operation in `reconciliation_required`, `install-view.ts:98` (not :371) prints the generic "compatibility operation … recovery is not available" text, because `can_resume`/`can_reconcile` exist only for install. `launch-view.ts:8` prints the generic Play error, and `:17` says "Another operation owns the game" while only a review is open. Three recheck buttons show (`#inspect-adoption`, `#inspect-launch`, `#inspect-install`). The Install primary is **disabled**, not enabled (`install-view.ts:55-57`), but it is still shown. | code read | LF-06 |
| F4 | The review shows `Review.source`/`destination` verbatim (`adoption-view.ts:215`), with no `\\?\` stripping anywhere in the frontend. It prints the full 64-hex digest and patch ids (`:120`) and download ids (`:124`). Four count summaries are spread over `:121`, `:154`, `:155` and `adoption-index-view.ts:16`. "▶" is the browser's default `<details>` marker, and `style.css` has no `summary` rule. | code read | LF-07, LF-08 |
| F5 | The missing space is in the template at `adoption-view.ts:127`. The sentence depends on `review.game_telemetry_available` (the DLL is bundled), not on launcher diagnostics. The clipping and overlap in screenshot 2 are unverified. | code read | LF-09; UAT row |
| F6 | The "Recover adoption…" trigger is `disabled` while its own confirmation is open (`adoption-view.ts:205`). The confirmation's two buttons have no `.settings-actions` wrapper (`index.html:42`); `#repair-confirmation` (`:12`) and `#game-update-review` (`:24`) share the same markup. | code read | LF-09 |
| G1 | `validate` logs one error with an absolute `game` path and no `event`. No path-level diff exists, and there is no generic predicted-vs-scanned tree diff helper. | code read | LF-01, LF-02 |
| G3 | `Upload` calls `ensure_no_active_operation` (`log_upload/mod.rs:182`). `ReconciliationRequired` is not `terminal()` (`operations.rs:34`), so the upload refuses `busy` in exactly the state it is needed. | code read | LF-15 |
| G4 | The store is `option_env!("LAUNCHER_LOG_SAS_URL")` (`store.rs:21-23`). With it empty, `begin_log_upload` returns `Disabled` and the view hides the button and empties the status, leaving a bare "Debug logs" heading (`log-upload-view.ts:48-50`). | code read | LF-15, LF-16 |
| G5 | The server cannot take a log upload as it stands. A launcher session gets 300 rows for its whole life (`launcher_budget.rs:24`), 200 rows per chunk (`launcher_gate.rs:140-150`), and string values of at most 256 bytes. Its R14 scrub replaces the **whole** value when any word looks like a path, URL, `@`, GUID or 24+ hex digits (`launcher_rows.rs:144-193`), which would blank most log lines. The replayed row's body is the target name, not the message (`replay_native.rs:337`). | code read | LF-10, LF-11, LF-12 |
| G6 | The launcher's own log is `launcher.*.log`, a daily appender keeping 7 (`shell/src/telemetry.rs:28-29`, `:66-72`), under `<state root>/../logs`. The game logs are `sgwdebuglog*` plus `sessions/**/*.log` under the binaries folder (`launcher-core/src/logs.rs:44-82`). | code read | LF-13 |
| G7 | The legacy-launcher text in `docs/client/sgw-launcher.md:484-516` (`uploaded.json` beside the exe, a greyed-out button) is stale for the desktop launcher. | code read | LF-17 |

## Decisions

| ID | Decision | Status | Reason |
|---|---|---|---|
| D-LF1 | **The Light final check is a per-file diff, not one digest.** `finish` persists the predicted tree in the stage (`expected-tree.json`, written in `admit`). After setup, it compares that tree with the scan, matching paths case-insensitively. The setup files (`SETUP_FILES`) are presence-only, because the launcher wrote them itself. **Missing** or **changed** files fail the check. **Case-only** differences and **unexpected** files are accepted with a WARN. When the stage holds no expected tree (an adoption started before LF-02), a Light adoption is accepted with a WARN (`outcome = unrecorded`). The verified in-place path keeps its digest check and gains only the event. | Decided (architect) | F1a, F1b, F1e. It keeps D-LA9 ("never blocks") for spelling noise, still catches a real failed move, and lets Recover finish the laptop's half-adopted folder once the user has a build with LF-02. Rejected: option (a), the dry run (F1b), and option (b), publishing whatever is there (it would hide a lost file). |
| D-LF2 | **Abandon after `Promoted`, Light only.** Between the moves and the `Promoted` write, `finish` copies whichever setup files exist into `<stage>/setup-originals/`, then writes a manifest last. `abandonable` becomes `Staged`, or `Promoted` + Light + manifest present. Abandon restores the setup files from that copy (deleting the ones that did not exist before), then runs today's `Way::Abandon` moves. Stock-case renames are not undone (the bytes are unchanged). | Decided (architect) | F1c; the escape hatch the UAT note asked for as option (c). An adoption that has no manifest is offered Recover only, as today. |
| D-LF3 | **Scan names:** any valid-Unicode name with no control characters, no `:` and no `\`, at most 4,096 bytes, is scanned like any other. The ASCII rule stays only where an index is built or read (`file_index::check_path`). | Decided (architect) | F2. Non-ASCII names round-trip on NTFS. Plan JSON, the tree and telemetry carry UTF-8, and the server maps non-ASCII to `?`. |
| D-LF4 | **An invalid login-server list refuses the light preview before any move**, with the new rejection check `login_servers_invalid`. | Decided (architect) | F1d. |
| D-LF5 | **The adoption panel is the only owner of an adopt operation's state.** While an `adopt` operation is not terminal, Install hides its primary, its recovery text and `#inspect-install`. Play shows one adopt-specific line ("An adoption review is open…" while it runs, "Finish or abandon the interrupted adoption…" once reconciliation is required) and hides `#inspect-launch`. | Decided (architect) | F3: one banner, and one recheck button. |
| D-LF6 | **Hashes and ids go behind a "Release details" toggle**: the manifest digest, patch ids and download ids. The summary shows counts and sizes. There are no friendly patch names: LF-08 confirms the manifest has no display name, and if it has one, LF-08 uses it. | Decided (architect) | F4. |
| D-LF7 | **Log ingest uses a launcher session minted with `purpose = "logs"`** (C-LF4). The server picks limits, row rules and budgets from that claim before it reads the body (R7). Log rows and event rows never share a session, and each kind of session refuses the other kind's targets. | Decided (architect; ingest is the user's direction, 2026-10-22) | G5. It keeps R1-R16's separation by claim, not by row content. |
| D-LF8 | **Budgets** (env overrides in C-LF6): 40,000 lines per upload (20,000 launcher + 20,000 game, LF-19), newest first; 1,000 rows per chunk, 512 KiB decompressed and 64 KiB compressed; 40,000 rows per log session; 80,000 log rows per install per UTC day; 400,000 per address per UTC day; 40,000 log rows per minute in total. A log chunk is admitted whole or refused with 429. | Decided (user proposal; the security pass may lower them). Chunk rate: architect ruling 2026-10-22 (wave 3 D1) | G5. The server keeps 20 chunks per log session per minute, and the per-install window of 20 per minute is shared with the install's event session. The launcher paces log chunks instead (LF-14): at least 3.5 s between sends, so at most 18 in any minute and the event session keeps headroom. About 40-60 chunks take 2.5-3.5 minutes, well inside the 15-minute token; a 429/503 still ends `Partial`. No server limit changes. A whole-or-nothing chunk lets the launcher advance its offsets only for rows it knows landed. |
| D-LF9 | **Message scrub per word.** The server applies R14's patterns to each whitespace-separated word of `message`, replaces a matching word with `<removed>`, and caps the message at 2,048 bytes (the launcher's own line cut, LF-13; a 4,096-byte row still fits). Over-long lines are marked, never dropped. Version 1 sends no parsed fields; the forwarded events already carry structured ones. | Decided (architect) | G5: a whole-value scrub would blank most lines. |
| D-LF10 | **Incremental upload** uses a new `uploaded-log-offsets.json` holding per-file byte offsets. The old `uploaded-logs.json` stays until LF-18. | Decided (architect) | Proposal item 4, without migrating a ledger the blob path still reads. |
| D-LF11 | **Security re-review.** `network-security-auth` runs a parallel read-only pass against R1-R16 (the launcher-session security review, `security-review-launcher-session.md` in the launcher-adopt-ux worknotes, not committed) on wave 3 and on LF-14. Its notes are numbered `A<k>`, and both passes must say `ship`. | Decided (process) | The launcher token's row volume and its field shape both widen. |
| D-LU1 | **Consent: yes.** Pressing "Upload debug logs" consents for that one upload even when "Send game diagnostics" is off. The panel says what is sent. | Decided (user, 2026-10-22) | LF-15 availability ignores `opted_in`; LF-16 shows the what-is-sent line. |
| D-LU2 | **Retire the blob path.** `LAUNCHER_LOG_SAS_URL`, `store.rs`, the zip and its ledger, the workflow secret plumbing and the docs all go (LF-18). No blob store remains. | Decided (user, 2026-10-22) | Nothing collects crash dumps today, and no release ever had the secret (G4). |
| D-LU3 | **Game logs are ingested unfiltered.** `client.launcher.game_log` carries `sgwdebuglog*` and `sessions/**/*.log` lines; chat and character names are acceptable. Only the per-word scrub of D-LF9 (paths, IPs, URLs, `@`, GUIDs, long hex) applies. | Decided (user, 2026-10-22) | Troubleshooting needs the game's own log. The server target list carries both targets from wave 3, so LF-19 needs no second deploy. |
| D-LU4 | **The security review hardens; it does not remove data.** The wave-3 and LF-14/LF-19 `network-security-auth` passes may tighten abuse limits, token scope, budgets and scrub correctness. A note that drops a target, drops game logs, or cuts the message below 2,048 bytes is out of scope: the reviewer records it as a risk for the owner, and the packet ships without it. | Decided (user, 2026-10-22) | "We must get this data for best troubleshooting and debugging." |
| D-LU5 | **Release order.** Merge all PRs first (no deploy-before-U4 gate), then cut both releases (the colo server and the launcher) together, the server live before or with the launcher. | Decided (owner, 2026-10-22; GO given) | Nobody uses log upload today, so retiring the blob path breaks nothing in the field. |

Still open from launcher-telemetry: D2 (the `inPlaceMaintenance.abandon` wording after a light abandon). LF-03 makes it real; LF-09 does not decide it.

## Packet status

Statuses: Ready / BlockedDependency / BlockedDecision / Writing / Review / Integrated / UATPending / Done. Final state (2026-10-22): every code packet is merged and reads `UATPending`, because no live UAT has run; U1-U6 below are owed. Nothing here reaches `Done` until the owner reports those rows.

| Wave | Packet | Title | Agent | Depends on | Status |
|---|---|---|---|---|---|
| 1 | LF-01 | `tree_diff`: case-insensitive tree diff, samples, `adoption_tree_checked` helper | packet-coder | — | UATPending, merged in #1986 (U1) |
| 1 | LF-04 | Scan accepts valid-Unicode names; telemetry truncation on a char boundary | packet-coder | — | UATPending, merged in #1986 (U1) |
| 1 | LF-05 | An invalid login-server list refuses the light preview | packet-coder | — | UATPending, merged in #1986 (U1) |
| 1 | LF-02 | Light final check by diff; expected tree in the stage; Recover finishes old adoptions | rust-gameserver-dev | LF-01, LF-05 merged into `lf-w1` | UATPending, merged in #1986 (U1) |
| 1 | LF-03 | Setup originals and Abandon after `Promoted` (Light) | rust-gameserver-dev | LF-02 merged into `lf-w1` | UATPending, merged in #1986 (U1) |
| 2 | LF-06 | One owner for an interrupted adoption: Install and Play defer | packet-coder | — | UATPending, merged in #1984 (U2) |
| 2 | LF-07 | Display paths without `\\?\`; one folder line; "found" wording | packet-coder | — | UATPending, merged in #1984 (U2) |
| 2 | LF-08 | Release details toggle, reconciled counts, summary marker | packet-coder | LF-07 merged into `lf-w2` | UATPending, merged in #1984 (U2) |
| 2 | LF-09 | Telemetry line, maintenance trigger, button gaps, UAT script | packet-coder | LF-08 merged into `lf-w2` | UATPending, merged in #1984 (U2) |
| 3 | LF-10 | Dev-session `purpose = "logs"` for launcher sessions | packet-coder | — | UATPending, merged in #1985 (U3) |
| 3 | LF-11 | Log row rules, per-word scrub, log-targets file | packet-coder | — | UATPending, merged in #1985 (U3) |
| 3 | LF-12 | Log-session limits, budgets and replay body | rust-gameserver-dev | LF-10, LF-11 merged into `lf-w3` | UATPending, merged in #1985 (U3) |
| 4 | LF-13 | Log line reader and offsets ledger (engine) | packet-coder | `lf-w4` created from the `lf-w3` tip | UATPending, merged in #1987 (U4) |
| 4 | LF-14 | Log rows and the log sender (engine) | rust-gameserver-dev | `lf-w4` created | UATPending, merged in #1987 (U4) |
| 4 | LF-15 | Upload state machine, shell command and TS schema | rust-gameserver-dev | LF-13, LF-14 merged into `lf-w4` | UATPending, merged in #1987 (U4) |
| 4 | LF-16 | Debug-logs view copy, first press, tests, UAT | packet-coder | LF-15 merged into `lf-w4` | UATPending, merged in #1987 (U4) |
| 4 | LF-19 | Game-log reader and `client.launcher.game_log` rows (engine) | rust-gameserver-dev | LF-15 merged into `lf-w4` | UATPending, merged in #1987 (U4) |
| 4 | LF-17 | Docs for log ingest (server and launcher) | documentation-writer | LF-19 merged into `lf-w4` | UATPending, merged in #1987 (U4) |
| 5 | LF-18 | Retire the blob upload path | packet-coder | PR U4 merged | UATPending, merged in #1988 (U5), review fixes in X |
| close | LF-20 | Close-out: UAT rows, status docs, retire | documentation-writer | every PR merged | Review (this PR, X) |

## PRs and merge order

| PR | Contents and merge order into its integration branch | Branch (worktree) | Base |
|---|---|---|---|
| U1 | **#1986 merged.** Wave 1: LF-01, LF-04, LF-05, LF-02, LF-03 | `launcher-uat-fixes/adoption-engine` (`lf-w1`) | `origin/main` |
| U2 | **#1984 merged.** Wave 2: LF-06, LF-07, LF-08, LF-09 | `launcher-uat-fixes/adoption-ui` (`lf-w2`) | `origin/main` |
| U3 | **#1985 merged.** Wave 3: LF-10, LF-11, LF-12 | `launcher-uat-fixes/log-ingest-server` (`lf-w3`) | `origin/main` |
| U4 | **#1987 merged.** Wave 4: LF-13, LF-14, LF-15, LF-16, LF-19, LF-17 | `launcher-uat-fixes/log-ingest-launcher` (`lf-w4`) | the `lf-w3` tip (stacked; `ship.ps1` restacks after U3 merges) |
| U5 | **#1988 merged.** LF-18 | `launcher-uat-fixes/lf-18-retire-blob` (`lf-18`) | `origin/main` after U4 |
| X | This PR (number on opening). LF-20, plus the reviewed LF-18 follow-up fixes (launcher-guide operator text, the unused `gethostname` dependency, the `sgw-launcher.md` wording). | `launcher-uat-fixes/post-merge-fixes` (`lf-fix-w6`) | `origin/main` after the rest |

Into `main`: U1 first (the blocker), U2 at any time, then U3 → colo deploy (owner) → U4 → a launcher release (owner) → U5 → X. Waves 1, 2 and 3 can run at the same time; wave 4 waits for U3 to be integrated, not merged.

## Owner UAT owed (now rows U1-U6 in [unified-uat.md](../../guides/unified-uat.md#launcher-uat-fixes); ask before any laptop or lab use)

- U1: on the laptop's half-adopted `C:\Temp\sgw`, a build with PR U1 → Recover finishes (`adoption_tree_checked outcome = unrecorded`).
- U2: a fresh copy → a light adoption completes, and `adoption_tree_checked` shows `matched` or `accepted` with samples.
- U3: the stock `másolata.bmp` file is not refused.
- U4: interrupted adoption → one banner and one recheck button; Install is hidden.
- U5: the review shows one folder line without `\\?\`, and the hashes sit behind "Release details". Recheck screenshot 2's clipping and overlap (F5, unverified).
- U6: "Upload debug logs" during a reconciliation → "Sent N lines. Reference …". One SigNoz filter on that reference shows the lines. A second press sends only the new lines. With the game run once, the same reference also shows `client.launcher.game_log` rows.

## Close-out (2026-10-22)

All code packets (LF-01..LF-19) are merged: U1 #1986, U2 #1984, U3 #1985, U4 #1987, U5 #1988. This PR (X) carries LF-20 and the reviewed LF-18 follow-up fixes. The campaign closes here; the colo server and the launcher are released together by the owner (D-LU5 in [RESUME.md](RESUME.md)).

### Tested and untested

| Tested | Not tested |
|---|---|
| Unit and flow tests per packet; per-packet and wave reviews; security passes on wave 3, LF-14 and LF-19 (all `ship`); engine (924) and shell (147) runs for U5 | **No live UAT of any kind.** No laptop run, no lab run, no colo run. Rows U1-U6 are owed to the owner. |
| The admission test for the operation gate fails when reverted (LF-18) | The `adoption_tree_checked` result on the real half-adopted folder (U1) and on a fresh light adoption (U2) |
| | The Debug logs flow end to end against a deployed server, and the rows in SigNoz (U6) |
| | F5's clipping and overlap in the laptop screenshot (U5): still unverified |

### Owner risks (accepted under D-LU3 and D-LU4; nothing was removed to avoid them)

1. **Chat and names leave the machine.** Game logs are ingested unfiltered. Only the per-word scrub applies (paths, IPs, URLs, `@`, GUIDs, long hex).
2. **2,048-byte floor.** A message is cut at 2,048 bytes by design. A secret or a spaced name inside a line is not caught by the per-word scrub.
3. **Rows over 4,096 bytes after escaping are dropped by the server.** The launcher marks over-long lines, but a line that escapes past the limit is lost.
4. **Older lines are not uploaded past 20,000 new launcher lines and 20,000 new game lines per press.** Newest first; the rest stay on disk.
5. **Daily ceilings.** 80,000 log rows per install and 400,000 per address per UTC day, and 40,000 per minute in total. A busy day ends `Partial`.
6. **Secrets shorter than 32 characters, or split by dots, are kept** by the scrub.
7. **Log rows are forgeable** by anyone who holds a log token. The server cannot tell a real launcher line from a made-up one.
8. **Budgets are speed bumps, not walls.** The install and address budgets slow abuse; they do not stop a determined sender.
9. Scrub gaps found by the security passes were accepted under D-LU4 (the passes' notes are in the PR threads).

### Status docs

`docs/project-status.md` and `docs/gap-analysis*.md` have no launcher row, so this close-out leaves them unchanged (R2: status docs change once per campaign, and there is nothing to change). The launcher campaign's own ledger is [launcher-telemetry/RESUME.md](../launcher-telemetry/RESUME.md), which now points here for F1-F6 and G1-G4.

## Repeat-release follow-up

[Design: every signed launcher release](design-repeat-releases.md) records
owner-approved D-LX15 and the applied-content identity logging follow-up.
[PR #1991](https://github.com/SandboxServers/Cimmeria/pull/1991) merged as
`7be8413b0`; its worktree is retired. See the [validation worknote](worknotes/repeat-releases-validation.md).

## Path display and startup update follow-up (2026-10-23)

PR #1993 merged as `7459ef2cc`; its worktree is retired. Final Windows/macOS
CI and merge-queue checks passed. Launcher `2026.10.2303` includes this change.
Packaged/player acceptance below remains separate from those automated checks.

The owner requests one PR for [Windows path display](design-path-display.md) and
[startup checking with mandatory Update and restart](design-startup-update.md).
Formatting strips the Windows verbatim prefix only from visible folder strings;
storage, identity and native command paths retain their exact spelling. Startup
checks for updates without downloading; a known offer blocks Play until updated.
A failed check without a known offer permits Play with retry feedback; a failed
recheck retains a known offer and its gate. The [updater contract](../../../crates/launcher/desktop/docs/updater.md#startup-and-main-page-integration)
is the detailed behavior reference.

These owner UAT rows are **pending**. Frontend tests do not attest packaged
layout, a native picker, real replacement/restart or game behavior. The later owner-authorized `2026.10.2303` release includes this follow-up; these
rows still need packaged/player acceptance. Ask before laptop/lab use. Rows are mirrored in
[unified UAT](../../guides/unified-uat.md#path-display-and-startup-update-follow-up-pdu).

| ID | Action | Expected |
|---|---|---|
| PDU-U1 | Open Settings and completed adoption status for verbatim drive and UNC fixture paths; open maintenance confirmations | All visible absolute folders use normal Windows spelling; saving consent and reopening retains the exact native path and Show folder target |
| PDU-U2 | Start a configured Windows launcher with a newer signed offer | Startup checks once with visible feedback; Play is disabled and the main page offers Update and restart; no download occurs before the press |
| PDU-U3 | Press Update and restart once, then try a duplicate press; repeat with a saved Ready package | One inspect/prepare/apply sequence; immediate feedback; saved Ready skips download; Play stays gated until replacement startup |
| PDU-U4 | Fail a feed check with no known offer, then fail a recheck with an existing offer | First case shows retry feedback and permits Play after the bounded check; second retains the offer and keeps Play blocked, including after refresh |
| PDU-U5 | Fail preparation or lose its reply; reopen with active/reconciliation state | No automatic Apply or replay; Play remains blocked for owned update work; Settings exposes recovery status |

## Main-page layout follow-up (2026-10-23)

The [design](design-main-layout.md) moves the three existing preference controls
and native Changes to your client summary onto Play. View all changes expands
in place; Settings keeps maintenance, folders, servers and recovery. The Patch
Notes tab and its UI fetch are removed. Native `LaunchStatus.setup_complete`
hides the main-page archive and Recheck Play status buttons independently of
the admission-only `installation_id` and launcher-update gate. Settings retains
a separate Check Play status action with adjacent result and first-press feedback. Completed adoption no longer masquerades as unfinished compatibility setup.
Adoption now asks for `SGW.exe` in a native file picker, resolves its installation
root and retains the existing confirmation review before any changes.
Visible branding is Cimmeria; the window title is Cimmeria Launcher. Legacy disk
paths, identifiers and release asset filenames retain their compatibility names.

Implementation merged in [PR #1996](https://github.com/SandboxServers/Cimmeria/pull/1996)
as `b7364c5d0`; its worktree is retired. All six desktop CI jobs and the full
merge queue passed. Automated and fixture validation is recorded in the
[worknote](worknotes/main-layout-validation.md); packaged acceptance is pending. No further
release or laptop/lab use is authorized by this follow-up. The matching rows in
[unified UAT](../../guides/unified-uat.md#main-page-layout-follow-up-ml) use these ids.

| ID | Action | Expected |
|---|---|---|
| ML-U1 | Open a packaged launcher at normal and small window sizes; open Settings and return | Cimmeria Launcher title and gate tagline; no Patch Notes tab; Play, preferences and change summary remain reachable by scrolling |
| ML-U2 | Reopen a setup-complete adopted client, then inspect a setup-complete managed client; start Play and use Settings Check Play status; separately open a missing or interrupted install | Native setup_complete hides main-page recheck/archive without the false compatibility warning, including during a known non-reconciling launch; Settings gives first-press feedback and an adjacent result; genuine setup, errors and recovery remain visible |
| ML-U3 | Toggle client patches, launcher diagnostics and game diagnostics once each; reopen; fail one save | Immediate feedback; confirmed choices persist across reopen; failure restores confirmed state; a directly adopted client follows its saved patch choice on the next Play; a legacy-bound adopted client retains its reviewed patch binding until Repair |
| ML-U4 | Read Changes to your client and expand View all changes; perform an operation that changes native state | Main-page summary and detailed native rows refresh; errors or unreadable records remain explicit |
| ML-U5 | Offer a newer signed launcher while setup is complete; inspect through Settings Check Play status | Setup prompts stay hidden even when admission installation_id is withheld; Settings shows immediate feedback and its inspection result; Update and restart still blocks Play until replacement startup |
| ML-U6 | Press Adopt an install or Settings Choose SGW.exe; select the nested Working/Binaries/SGW.exe, then cancel and try a different executable | Native SGW.exe picker gives immediate feedback; the correct executable resolves its installation root and opens the existing review; cancellation and a wrong executable change no installation files; selection alone never applies changes |

## Embedded Windows payload follow-up (2026-10-23)

The owner authorizes the [embedded-payload design](design-embedded-payload.md):
keep the public Windows ZIP and put one launcher EXE inside it, embedding the
three pinned Play pieces. Startup verifies and repairs a private app-local
payload cache; version leases preserve active payloads during cleanup. Old
loose sidecars stay untouched. macOS already bundles its resources inside
Cimmeria Launcher.app and keeps that layout. Game content, WebView2 and the
Mac Wine runtime remain separate.

Implementation is active in `.claude/worktrees/launcher-embedded-payload`,
branch `feat/launcher-embedded-payload`. Tests and packaging checks remain
implementation evidence until reported; real packaged/game acceptance remains
pending. The owner gave GO to merge this PR and release the launcher from main after merge, retaining the Windows ZIP. This does not authorize a live game or lab session. The
[release contract](../../../crates/launcher/desktop/docs/release.md#what-the-windows-job-builds)
and [native resource contract](../../../crates/launcher/desktop/docs/launch.md#embedded-windows-play-resources)
describe the packaging and runtime boundaries.

These packaged acceptance rows are pending; ask before laptop or lab use.
They are mirrored in [unified UAT](../../guides/unified-uat.md#embedded-windows-payload-follow-up-ep).

| ID | Action | Expected |
|---|---|---|
| EP-U1 | Unzip the new Windows ZIP into a writable clean folder and start its sole EXE | Launcher prepares verified Play resources without companion folders; no game launch or download occurs merely from opening it |
| EP-U2 | With all launchers closed, remove one ordinary cached payload file and reopen; keep another launcher open while starting a build with changed payload bytes | Missing file is repaired from embedded bytes; the active version remains available while the new version uses its own cache |
| EP-U3 | Update an older multi-file Windows launcher with a signed single-EXE ZIP using Update and restart; separately unzip the Mac release | Windows replacement starts with verified cached resources and leaves old sidecars alone; Mac still presents one Cimmeria Launcher.app with bundled resources |
