# Lab navigation telemetry: wave 1b packets (owner decisions D-LN2, D-LN3)

Ledger: [README.md](README.md) · Main packets and rules R1-R8: [work-packets.md](work-packets.md) · Resume: [RESUME.md](RESUME.md)

Architect pass, 2026-10-22, after the owner decided D-LN2 (b) and D-LN3 (b, "remove the quarantine"). Every packet is prescriptive: if the code does not match what a packet says (a function moved, a line is gone, a grep finds an extra user), stop and return `blocked` with what you found. Do not redesign. Line numbers are from `main` on 2026-10-22.

## Dispatch and shipping

- **Packets.** LN-20a → LN-20b → LN-20d are code and stack in that order (each worktree is cut at the previous packet's reviewed head). LN-20c and LN-20e are docs only, cut from `main`, and run in parallel with LN-20a. LN-12b is independent.
- **LN-20 ships as one integration PR** from branch `lab-nav-telemetry/wave1b` (integration worktree `ln-w1b`, cut from `main`). `ops` merges the packets into it with `merge-noff.ps1` in the order LN-20a, LN-20b, LN-20d, LN-20c, LN-20e; the wave review runs there; one PR ships it. The quarantine lift must land at once: a half-lift (dialogs served but the cell still suppressing the Banker offer, or the docs still saying "quarantined") must never reach `main`.
- **LN-12b ships its own PR**, after LN-11 has merged (LN-11 commits the memory entry LN-12b appends to).
- **Merge order against wave 1.** LN-20e edits `docs/gameplay/inventory-system.md` and `docs/architecture/observability-target-catalog.md`, which LN-05 also edits (other hunks). Merge the wave-1b PR after LN-05; if LN-05 is still in review, `ops` rebases `lab-nav-telemetry/wave1b` onto the new `main` before the PR.
- **Wave review.** One packet-reviewer holds all six packets (LN-20a..e on the integration tree, LN-12b by its branch range). It checks that no `quarantin` mention outside `docs/analysis/`, `docs/gap-analysis*` and `docs/project-status.md` still describes 60100/60101/60104/60110 as held back, and that `VAULT_EXPAND_DIALOG_SERVED`, `QUARANTINED_DIALOG_OVERRIDES`, `dialog_quarantined` and `expand_offer_suppressed` have zero hits in `crates/` and `docs/` (outside `docs/analysis/`).
- **No lab, no publish.** The owner skips the lab for both changes and validates after release. No packet starts a client, re-signs `MANIFEST.txt` or publishes the overlay. The Castle_CellBlock entry crash (2026-09-27, cause never named) may return with LN-20a; that is the accepted risk (D-LN3).
- **Agents.** All six: `packet-coder` (Haiku). Host: windows for all.
- **Report and return.** Report at `<worktree>\.claude\reports\lab-nav-telemetry\<packet>-coder.md`; return exactly `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`. The report states what was tested and what was not.

## Rules carried by these packets

- R1. PowerShell only, never Bash. Every cargo call goes through the lane, from your worktree, in the foreground: `Set-Location <worktree>; pwsh tools/build-lane/lane.ps1 cargo <cmd> -p <crate> ...`, in the order fmt, clippy, nextest. Never call cargo directly. Read the lane summary and its failures file; don't rerun to see output.
- Server set for crate `<c>`: `cargo fmt -p <c> --check` (on failure run `cargo fmt -p <c>` through the lane, then recheck), `cargo clippy -p <c> --all-targets -- -D warnings`, `cargo nextest run -p <c>`.
- R2. One worktree. No `git stash`, no `git worktree prune`, no push. Commit locally once the checks are green, with the attribution lines from your brief.
- R3. Files: soft cap 500 lines, hard cap 700. `dialog_overrides/mod.rs` is 677 lines: it must shrink in LN-20a and LN-20b, never grow.
- R4. `docs/**/*.md` are CRLF. After editing one, convert with the absolute path: `$p='<abs path>'; $t=[IO.File]::ReadAllText($p) -replace "(?<!\r)\n","`r`n"; [IO.File]::WriteAllText($p,$t)`.
- R6. A runtime change needs a test that fails when the change is reverted ([TESTING.md](../../../TESTING.md)). Log rows are pinned with the crate's existing `LogCapture` helpers, the way the neighbouring tests do it.
- R8. Edit only the files your packet names.
- R9. Seeds are the source of truth: do not edit `db/resources/` or add `db/scripts/` migrations. No pak rebuild: `data/cache/CookedDataDialogs.pak` stays as committed. Served overrides reach the client through the existing runtime push at login pak sync (`DIALOG_OVERRIDES` regenerates the Dialogs category and the versionInfo InvalidKeys push names the ids), exactly as 60150/60151 do today.

---

### LN-12b BM overlay clock: reopen searches again, Search re-enables

Host: windows · Agent: packet-coder · Branch `lab-nav-telemetry/ln-12b-bm-clock` · Worktree `ln-12b` · Commit `LN-12b: count BM overlay time from frame deltas so reopen searches again`

Why: F8, D-LN2 (b). RE (LN-12, static): `BlackMarketMod.now()` returns `worldGetDeltaSeconds()`, which is the **per-frame delta** (about 0.016-0.033 s), not a clock. So after the first search `searchSuspended` never clears (the 4 s `SEARCH_DELAY` never elapses), every reopen hits `openSession`'s suspended branch and sends nothing, Search/Next/Prev stay disabled, and the 15 s `PENDING_TIMEOUT` never fires. The test stub `stubs.lua:200` returns a monotonic clock, which hid the bug.

Rules: R2, R4, R6, R8. No cargo lane. Do not touch `MANIFEST.txt`, signatures or any publish script: re-signing and publishing the overlay are owner steps, and the owner waived the lab-load-before-publish rule for this change.

**Line endings.** `crates/client-patches/overlay/**` Lua files are CRLF and marked `-text` in `.gitattributes` (git will not convert them). After editing each of the three Lua files, run the R4 snippet on its absolute path, then check `([regex]::Matches([IO.File]::ReadAllText($p), "(?<!\r)\n")).Count` is 0.

1. `crates/client-patches/overlay/Content/UI/Core/BlackMarket/BlackMarket.lua`:
   - Replace `BlackMarketMod.now()` (L125-131) with an accumulated clock:
     ```lua
     -- worldGetDeltaSeconds() is the frame delta, not a clock (LN-12; every stock
     -- caller uses it as a delta). onPreRender accumulates it here.
     BlackMarketMod.clock = 0
     function BlackMarketMod.now()
         return BlackMarketMod.clock
     end
     ```
   - `onPreRender(window, view)` (L983): before `local now = BlackMarketMod.now()`, add:
     ```lua
     local ok, dt = pcall(worldGetDeltaSeconds)
     if ok and type(dt) == 'number' and dt > 0 and dt < 1 then
         BlackMarketMod.clock = BlackMarketMod.clock + dt
     end
     ```
     (A frame of 1 s or more, a hitch or a load, is not counted.)
   - `openSession()` (L495): set `BlackMarketMod.searchSuspended = false` before the `if BlackMarketMod.searchSuspended then` check at L511, with a comment: each open is a fresh session, and the clock stops while the window is hidden (PreRender only runs while subscribed).
   - Change nothing else (`performSearch` L925-937, `updateSearchNavButtons` L1326 stay).
2. `crates/client-patches/overlay/test/stubs.lua` L199-200: replace with `env.frameDelta = 0` and `env.worldGetDeltaSeconds = function() return env.frameDelta end`, with a comment that the real binding returns the frame delta.
3. `crates/client-patches/overlay/test/run.lua`:
   - `tick(env, seconds)` (L80-83) splits time into 0.25 s frames:
     ```lua
     local function tick( env, seconds )
         local left = seconds
         while left > 0 do
             local dt = math.min(0.25, left)
             env.frameDelta = dt
             env.BlackMarketWin:fire('Events.PreRender')
             left = left - dt
         end
     end
     ```
     No other caller changes: every `tick(env, 5)` and `tick(env, 11)` keeps its meaning.
   - The scenario at L554-566 ("reopening while open refreshes; reopening during the search delay says so"): rename it "reopening while open refreshes; reopening after a close searches again", and replace L564 (`eq(status(env), 'Press Search to list auctions.', 'open inside the delay')`) with a count check: take `local before2 = callCount(env)` just before the `onOpen` at L563, then `eq(callCount(env), before2 + 1, 'a reopen after a close searches again')` and `eq(lastCall(env).op, 'search', 'the reopen sends search')`. Keep L565.
   - New scenario after it, `'F8: open, reply, close, reopen sends a second search'`: `newClient()`, `openWithResults(env, items(1, 2), 2)`, `tick(env, 1)` (inside the delay), `env.BlackMarketWin:hide()`, `local before = callCount(env)`, `env.CimmeriaBM.onOpen(4242)`, then `eq(callCount(env), before + 1, ...)` and `eq(lastCall(env).op, 'search', ...)`.
   - New scenario `'Search re-enables after 4 s of frame deltas, not after one long frame'`: `openWithResults(env, items(1, 2), 2)`; `env.frameDelta = 5; env.BlackMarketWin:fire('Events.PreRender')` then `eq(env.BlackMarket_SearchButton.enabled, false, 'a 5 s hitch is not counted')`; then `tick(env, 4)` and `eq(env.BlackMarket_SearchButton.enabled, true, 'Search after 4 s of frames')`.
   - Use only the helpers already in the file (`newClient`, `openWithResults`, `items`, `callCount`, `lastCall`, `status`, `tick`, `eq`, `scenario`).
4. Test (logic UAT, the overlay's Lua harness): `Set-Location <worktree>; python -c "import lupa"`; if that fails, `python -m pip install lupa`. Then `python -I crates/client-patches/overlay/test/run_lupa.py`. It must print `N scenarios, 0 failed` with N two higher than on `main` (run it once before your edits and record N). Revert proof: put the old `now()` body back for one run (it returned `worldGetDeltaSeconds()` directly), record which scenarios fail (expected: the two new ones, the renamed one and the existing "Search after the delay" check at L197), then restore the fix. Put both counts in the report.
5. Docs:
   - `crates/client-patches/overlay/README.md`: in `## What changed in BlackMarket.lua` (L42) add one bullet for the clock (frame deltas accumulated in `onPreRender`; each open clears the search delay; LN-12). In the Logic UAT section (L88-111) add one sentence: the stub's `worldGetDeltaSeconds` returns `env.frameDelta`, and `tick` advances time in 0.25 s frames. Keep the file's existing line endings (check before and after, as above).
   - `docs/reverse-engineering/findings/black-market-client-io.md` § 5 (L163; table rows U10-U12 at L178-180): add a row U13 in the same columns: the clock bug, its effect (no `BMSearch` on reopen, Search/Next/Prev dead, pending timeout never fires), "fixed in the overlay (LN-12b); static evidence (LN-12); live check owed after the owner publishes". R4.
   - `docs/gameplay/black-market.md` `## Remaining Work` (L279): one bullet: the LN-12b overlay fix needs the owner to re-sign and publish the overlay manifest, then check that a reopen sends a second `BMSearch` (server `wire.in`). R4.
   - `.claude/agent-memory/game-archaeology-specialist/bm-overlay-clock-reopen-ln12.md` (committed by LN-11): append one line, "Fixed in the overlay by LN-12b (2026-10-22); live check owed after the owner publishes." If the file is not in your worktree, return `blocked` (LN-11 has not merged).

---

### LN-20a Debug-hub dialogs 60100/60101/60104 served

Host: windows · Agent: packet-coder · Branch `lab-nav-telemetry/ln-20a-hub-dialogs` · Worktree `ln-20a` · Commit `LN-20a: serve the debug-hub dialogs 60100, 60101 and 60104 (D-LN3)`

Why: F1, D-LN3. The base refuses the cooked-data miss for 60100/60104 as `unknown_key` because those entries sit in `QUARANTINED_DIALOG_OVERRIDES` (#943, the 2026-09-27 Castle_CellBlock map-load crash), so they are never served and the client has no entry. The owner chose to lift the quarantine and accept the crash risk.

Rules: R1 (server set for `cimmeria-resources`; fmt and clippy for `cimmeria-cell-catalog` and `cimmeria-cell-content`), R2, R3, R6, R8, R9.

1. New file `crates/resources/src/base/dialog_overrides/overrides_debug_hub.rs`, in the pattern of `overrides_cellblock.rs`:
   - `//!` header: the debug-hub dialogs (templates and chains in `docs/content/debug-hub.md`), served since 2026-10-22 (D-LN3), quarantined 2026-09-27 to 2026-10-22 after a Castle_CellBlock map-load crash whose cause RE never named (`docs/reverse-engineering/findings/cooked-dialog-override-crash.md`).
   - `use super::{...};` with exactly the types the moved entries use (`DialogOverride`, `DialogScreen`, `DialogButton`).
   - `pub(super) const STASIS_DIALOG: DialogOverride` (60100, `mod.rs` L280-312), `pub(super) const STASIS_ANSWER_DIALOG: DialogOverride` (60101, L313-328), `pub(super) const MAIL_CLERK_DIALOG: DialogOverride` (60104, L329-351). Move each entry **verbatim** (every field and string byte-identical), with its comment above it as a `///` doc comment.
2. `crates/resources/src/base/dialog_overrides/mod.rs`:
   - Add `mod overrides_debug_hub;` beside `mod overrides_cellblock;` (L52-88).
   - Append at the END of `DIALOG_OVERRIDES` (after `overrides_cellblock::PRISONER_CHOICE_DIALOG`, L245): a comment `// D-LN3 (2026-10-22): the debug-hub dialogs, out of quarantine. See overrides_debug_hub.rs.` then `overrides_debug_hub::STASIS_DIALOG,`, `overrides_debug_hub::STASIS_ANSWER_DIALOG,`, `overrides_debug_hub::MAIL_CLERK_DIALOG,`.
   - Delete the three entries from `QUARANTINED_DIALOG_OVERRIDES`; it keeps only 60110 (L352-380). In its doc (L248-278) add one paragraph: the debug-hub entries were restored on 2026-10-22 by owner decision D-LN3 with the crash cause still unknown; only the Banker's Expand dialog 60110 remains here.
   - `DIALOG_OVERRIDES` doc: L173-174 becomes "The debug-hub entries (`overrides_debug_hub.rs`) are checked against the seed by `override_seed_agreement_debug_hub`."; L182-183 "those dialogs are now quarantined" becomes "those dialogs were quarantined from 2026-09-27 and served again from 2026-10-22 (D-LN3)".
   - Test `debug_hub_dialogs_are_quarantined` (L629-655) becomes `debug_hub_dialogs_are_served`:
     ```rust
     let served = |id: u32| DIALOG_OVERRIDES.iter().filter(|ov| ov.dialog_id == id).count();
     for id in [60100, 60101, 60104] {
         assert_eq!(served(id), 1, "debug-hub dialog {id} is served exactly once (D-LN3)");
     }
     let held: Vec<u32> = QUARANTINED_DIALOG_OVERRIDES.iter().map(|ov| ov.dialog_id).collect();
     assert_eq!(held, [60110]);
     ```
     The `xml` closure finds the id in `DIALOG_OVERRIDES.iter().chain(QUARANTINED_DIALOG_OVERRIDES)`; keep all five button asserts (L646-654) unchanged. Update its doc comment.
   - Leave `all_overrides_emit_their_ids_and_text`, `quarantined_dialogs_are_not_served` and `the_expand_dialog_flag_matches_the_served_list` as they are (still true with 60110 held).
3. `override_seed_agreement_debug_hub.rs`: `//!` L19-21 say the definitions are served from `overrides_debug_hub.rs` (60110 still held until LN-20b); the import (L25) adds `DIALOG_OVERRIDES`; `hub_override` (L39-46) looks in `DIALOG_OVERRIDES.iter().chain(QUARANTINED_DIALOG_OVERRIDES)` and its panic text says "must be defined in DIALOG_OVERRIDES".
4. `overrides_cellblock.rs` header (L1-5): drop "Multi-screen or NPC-speaker authored dialogs stay quarantined"; say these two keep the one-screen speaker-0 shape, and the multi-screen NPC-speaker debug-hub dialogs are served from `overrides_debug_hub.rs` since D-LN3.
5. `crates/resources/src/base/resources/tests/dialog_overrides.rs` (L82-108): in the expected ascending id list insert `60100, 60101, 60104` between `10734` and `60150`. After the quarantine loop (L94-108), add a loop over `[60100u32, 60101, 60104]` that asserts each **is** in `dialogs.elements`, using the same membership expression the quarantine loop uses, negated, with the message "debug-hub dialog {id} must be regenerated into the Dialogs category (D-LN3)". If another test pins the override count or the bumped Dialogs metadata to a fixed value, update it and name it in the report.
6. Comments only (no code): grep `quarantin` in `crates/cell-catalog/src/cell/spawner/tests/live_db_mail_clerk.rs` (L18-19), `live_db_debug_hub.rs` (L20-21), and in `crates/cell-content/src/cell/content/chain_replay_tests/` `debug_hub.rs` (L20-24) and `debug_hub_mail_clerk.rs` (L17-21). Say the dialog is served since 2026-10-22 (D-LN3), and drop "not served" / "shows nothing". Keep the test logic.

Test (unit, in-crate): `debug_hub_dialogs_are_served` and the `dialog_overrides.rs` id list and elements loop fail if any of the three goes back into the quarantine list (revert proof: the id is missing from `DIALOG_OVERRIDES` and from the regenerated category).

Lanes: the server set for `cimmeria-resources`; then `cargo fmt -p cimmeria-cell-catalog --check`, `cargo clippy -p cimmeria-cell-catalog --all-targets -- -D warnings`, and the same two for `cimmeria-cell-content`.

Docs: none in this packet (LN-20c, LN-20e).

---

### LN-20b Banker Expand dialog 60110 served; quarantine list and flag removed

Host: windows · Agent: packet-coder · Branch `lab-nav-telemetry/ln-20b-vault-expand` · Worktree `ln-20b`, cut at LN-20a's reviewed head · Commit `LN-20b: serve the vault Expand dialog and drop the dialog quarantine (D-LN3)`

Why: D-LN3. 60110 (the Banker's "Expand vault" offer, BV-05 / #479) is the last quarantined dialog; the cell suppresses the offer while `VAULT_EXPAND_DIALOG_SERVED` is false. Serving it makes the offer show on the first Banker visit with a full vault.

Rules: R1 (server set for `cimmeria-resources`, `cimmeria-wire`, `cimmeria-cell-interactions`, `cimmeria-cell`, `cimmeria-cell-methods`, in that order), R2, R3, R6, R8, R9.

0. Check first: `Grep` for `VAULT_EXPAND_DIALOG_SERVED` and `QUARANTINED_DIALOG_OVERRIDES` in `crates/`. Expected hits: only the files named below plus the comment-only files of LN-20d (`base-methods` org_vault `expand.rs`, `base-session` `bank_expand/gm_tests.rs`, `cell-console` `bank.rs`, which do not name these symbols) and the cell-catalog comments LN-20a already rewrote. Any other code user: return `blocked`.
1. New file `crates/resources/src/base/dialog_overrides/overrides_bank.rs`: `//!` header (the Banker's Expand vault offer, BV-05 / #479, served since D-LN3); `use super::{...}` with the types the entry uses; `pub(super) const VAULT_EXPAND_DIALOG: DialogOverride` = the 60110 entry moved **verbatim** from `mod.rs` (L352-380), its comment as `///`, with "Authored under quarantine" replaced by "Served since 2026-10-22 (D-LN3)".
2. `mod.rs`:
   - `mod overrides_bank;`; append at the END of `DIALOG_OVERRIDES`, after LN-20a's three entries: `// BV-05 / #479: the Banker's Expand vault offer (60110), served since D-LN3. See overrides_bank.rs.` and `overrides_bank::VAULT_EXPAND_DIALOG,`.
   - Delete `QUARANTINED_DIALOG_OVERRIDES` and its whole doc comment. In the `DIALOG_OVERRIDES` doc, remove every intra-doc link to it (L173, L183): keep the history as plain text ("quarantined 2026-09-27 to 2026-10-22, #943; see the crash finding").
   - Tests: delete `quarantined_dialogs_are_not_served`. Replace `the_expand_dialog_flag_matches_the_served_list` with `the_expand_dialog_is_served`: `use cimmeria_wire::cell::vault::VAULT_EXPAND_DIALOG_ID;` and assert `DIALOG_OVERRIDES` holds `VAULT_EXPAND_DIALOG_ID as u32` exactly once. In `debug_hub_dialogs_are_served`, add 60110 to the served loop, delete the `held` assert, and make the `xml` closure search `DIALOG_OVERRIDES` only. In `all_overrides_emit_their_ids_and_text` (L518) drop `.chain(QUARANTINED_DIALOG_OVERRIDES)`.
3. `override_seed_agreement_debug_hub.rs`: import only `DialogOverride` and `DIALOG_OVERRIDES`; `hub_override` looks in `DIALOG_OVERRIDES` only; `//!` says all four are served.
4. `override_seed_agreement_cellblock_62050.rs` and `_62060.rs`: drop `QUARANTINED_DIALOG_OVERRIDES` from the import (L12), delete the not-quarantined assert (62050 L66-71, 62060 L69-72) and the words "and not quarantined" in the doc above it (L51, L53). Keep the committed-pak check.
5. `crates/resources/src/base/resources/tests/dialog_overrides.rs`: insert `60110` after `60104` in the expected list; delete the quarantine loop (L94-108); add `60110` to LN-20a's must-be-served loop; drop "and no quarantined one" from the message.
6. `crates/wire/src/cell/vault.rs`: delete `VAULT_EXPAND_DIALOG_SERVED` and its doc (L30-37); in the module doc (L14-17) say 60110 is served in `DIALOG_OVERRIDES` (`overrides_bank.rs`). Keep `VAULT_EXPAND_DIALOG_ID` and `VAULT_EXPAND_BUTTON_ID`.
7. `crates/cell-interactions/src/cell/interactions/bank/expand.rs`: drop `VAULT_EXPAND_DIALOG_SERVED` from the import (L35); delete the `if !VAULT_EXPAND_DIALOG_SERVED { ... return; }` block (L150-172), so `offer_vault_expansion` always calls `show_expand_offer`; in the `offer_vault_expansion` doc (L93-103) and the `show_expand_offer` doc (L183-186) remove the quarantine wording. Keep `show_expand_offer`'s visibility.
8. `crates/cell-interactions/src/cell/interactions/bank/expand_tests.rs`: replace `a_quarantined_dialog_is_recorded_but_not_shown` (L144-186) with `an_offer_is_recorded_and_shown_at_once`. Same setup and the same `offer_vault_expansion(PLAYER, PLAYER_ID, banker, 40, 100, &tx, &mut mgr)` call; no `const { assert!(...) }`. Assert: the recorded offer is `OFFER_40`; `methods` equals exactly the two-item vector `showing_an_offer_sends_the_expand_dialog_and_the_price` (L191-230) expects (the `ON_DIALOG_DISPLAY` and the `ON_PLAYER_COMMUNICATION` price line); `offered_dialogs` contains the Expand id; exactly one `expand_offered` DEBUG row with `bank_slots` 40 and `price` 100; zero `expand_offer_suppressed` rows. Revert proof: with the suppression block back, `methods` is empty.
9. `crates/cell/src/cell/service/base_messages/tests/bank.rs`: `//!` L1-3 drop the quarantine; in `an_expansion_offer_reaches_the_vault_session`, L46-47 `assert!(rx.try_recv().is_err());` becomes `assert!(rx.try_recv().is_ok(), "the Expand offer is shown at once");` with its comment updated.
10. `crates/cell-methods/src/cell/cell_methods/player/interaction/bank_dispatch_tests.rs`, `the_expand_dialog_answer_reaches_the_purchase_path_once`: delete the manual `show_expand_offer` call and its "because quarantined" comment (L195-201); `offer_vault_expansion` alone now records and shows the offer. If the test then fails because it expected an empty channel before the answer, drain the channel at that point and say so in the report; any other failure: `blocked`.
11. Re-grep `crates/` for `VAULT_EXPAND_DIALOG_SERVED`, `QUARANTINED_DIALOG_OVERRIDES`, `dialog_quarantined`, `expand_offer_suppressed`: zero hits.

Lanes: the server set for each crate in the order of the Rules line.

Docs: none in this packet (LN-20c, LN-20e).

---

### LN-20d Comment sweep for the lifted quarantine

Host: windows · Agent: packet-coder · Branch `lab-nav-telemetry/ln-20d-comments` · Worktree `ln-20d`, cut at LN-20b's reviewed head · Commit `LN-20d: drop the dialog-quarantine wording from bank comments`

Why: three comment blocks still say the Expand dialog is quarantined.

Rules: R1 (fmt and clippy only, for the three crates below), R2, R8. Comments only: no code, no test change.

1. `Grep` `quarantin` (case-insensitive) in `crates/base-methods/src/base/world_entry/methods/org_vault/`, `crates/base-session/src/base/bank_expand/` and `crates/cell-console/src/cell/console/bank.rs`. Expected: org_vault `expand.rs` :11, `gm_tests.rs` :2, cell-console `bank.rs` :29 and :164. Ignore Black Market `QUARANTINED` auction-status hits (unrelated).
2. Rewrite each: the personal-vault Expand dialog (60110) is served since 2026-10-22 (D-LN3); `.bankexpand` stays the GM path. For org_vault `expand.rs`, keep the fact that the org vault has no client Expand UI (the dialog is personal-vault only), without the word quarantine.
3. A hit that is not a comment: return `blocked`.

Lanes: `cargo fmt -p <c> --check` and `cargo clippy -p <c> --all-targets -- -D warnings` for `cimmeria-base-methods`, `cimmeria-base-session`, `cimmeria-cell-console`.

---

### LN-20c Docs: debug hub and the crash finding

Host: windows · Agent: packet-coder · Branch `lab-nav-telemetry/ln-20c-hub-docs` · Worktree `ln-20c`, cut from `main` · Commit `LN-20c: document the lifted dialog quarantine (debug hub, crash finding)`

Why: D-LN3. The debug-hub docs and the crash finding describe the four dialogs as held back.

Rules: R2, R4 (every file below), R8. Docs only, no lane run. Never hand-edit a `<!-- gen:NAME -->` block; if a line you must change is inside one, return `blocked`.

1. `docs/content/debug-hub.md`:
   - L161-175 (the Dialog NPC callout, template 302, Airman Lance) and L181-188: the dialogs 60100/60101 are served again since 2026-10-22 (owner decision D-LN3, campaign lab-nav-telemetry); they were quarantined 2026-09-27 to 2026-10-22 (#943); the Castle_CellBlock entry crash may return, and the owner validates after release. Keep the walkthrough steps; drop "shows no dialog".
   - L417-418 (the mail clerk callout): the same for 60104.
   - L509 and L512: if they say the dialogs are not served, say served.
   - L528 table row: "`QUARANTINED_DIALOG_OVERRIDES` (not served)" becomes "`overrides_debug_hub.rs` in `DIALOG_OVERRIDES` (served since D-LN3)".
   - L567 test-table row: `quarantined_dialogs_are_not_served` becomes `debug_hub_dialogs_are_served`.
2. `docs/reverse-engineering/findings/cooked-dialog-override-crash.md`:
   - A new section `## 2026-10-22: quarantine lifted (D-LN3)` before `## Cross-references`: the owner chose to serve 60100, 60101, 60104 and 60110 again with the cause still unknown (no field was named); what to watch for: a session that gets `onClientMapLoad` for Castle_CellBlock and never sends `mapLoaded` (SigNoz, the same signature as 2026-09-27); the recovery is to put the entries back (revert the LN-20a/b commits) **and** have the tester delete `Cache.en-US\CookedDataDialogs.pak`, since the server evicts nothing; the owner validates after release, no lab run.
   - L71 ("Next step is empirical: after the #943 quarantine release ships, ..."): append "Superseded 2026-10-22: the quarantine is lifted (D-LN3); the empirical check is now the owner's post-release validation."
   - In `## Recommendation` and `## Summary`, one sentence each pointing at the new section; no other rewrite.
3. `docs/reverse-engineering/README.md` L83 (the index line for the crash finding): add "quarantine lifted 2026-10-22 (D-LN3)".
4. `Grep` `quarantin` in `docs/content/` and `docs/reverse-engineering/`: any hit left that describes these dialogs as held back goes in the report (do not edit files this packet does not name).

---

### LN-20e Docs: UAT guide, vault, catalog, commands

Host: windows · Agent: packet-coder · Branch `lab-nav-telemetry/ln-20e-uat-docs` · Worktree `ln-20e`, cut from `main` · Commit `LN-20e: UAT guide and vault docs for the served Expand and hub dialogs`

Why: D-LN3. The UAT guide, the vault docs, the target catalog and the command reference still describe the quarantine and its suppressed-offer row.

Rules: R2, R4 (every file below), R8. Docs only, no lane run. Never hand-edit a `<!-- gen:NAME -->` block; if a line you must change is inside one, return `blocked`. Do not touch `docs/gap-analysis*` or `docs/project-status.md` (close-out LN-14 does).

1. `docs/guides/unified-uat.md`:
   - L86 (K2, debug-hub NPCs show no dialog): the dialogs 60100/60101/60104 are served since 2026-10-22 (D-LN3). Post-release check for the owner: talk to Airman Lance and the Gate Mail Clerk, page through, press the button; and enter Castle_CellBlock twice. If the client dies after the map loads, report the time (SigNoz: `onClientMapLoad` with no `mapLoaded`) and delete `Cache.en-US\CookedDataDialogs.pak` before the next try.
   - L87 (K3, Banker Expand offer quarantined, 60110): the offer is served; a Banker visit with a full vault shows the Expand vault dialog and the price line on the first visit; `.bankexpand` stays the GM path.
   - L431 (Storage Officer, "No Expand vault dialog appears"): the expected result becomes "The Expand vault dialog appears, with the price line in chat".
   - L461 (the SigNoz query on `expand_offer_suppressed` with `reason='dialog_quarantined'`): query `expand_offered` instead (target `bank`, DEBUG), expecting one row per offer.
   - L490 (the Gate Mail Clerk opens no dialog, K2): the clerk opens dialog 60104 and the button sends the mail.
   - L964 (M6-1, `.net_dialog` with a quarantined hub dialog shows nothing): the hub dialog now shows.
   - Leave L85 (K1) and L1370 alone (LN-14 owns L1370).
2. `docs/gameplay/inventory-system.md`: L41 (vault expansion status) and the L232 callout naming `QUARANTINED_DIALOG_OVERRIDES`: the Expand offer is served since D-LN3 (`overrides_bank.rs`), no suppression. L328 (org vault): keep "no client UI" for the org vault, without the quarantine reason. Do not touch L279 (LN-05's hunk).
3. `docs/architecture/observability-target-catalog.md` L172 (the `bank` row): remove `expand_offer_suppressed` from its DEBUG list. Do not touch the `org_vault_open_rejected` row (LN-05).
4. `docs/commands.md` L622 (the Bank row, `.bankexpand`): drop any "while the dialog is quarantined" reason; `.bankexpand` is the GM path beside the served offer.
5. `docs/content/debug-area.md` L1483 ("crashed the client (#943)"): append "; served again since 2026-10-22 (D-LN3)".
6. `Grep` `quarantin|expand_offer_suppressed|dialog_quarantined` in `docs/` outside `docs/analysis/`, `docs/gap-analysis*`, `docs/project-status.md` and the LN-20c files: list any hit left in the report (Black Market auction `QUARANTINED` status hits are unrelated).
