# DU-02a Worknotes

> Type: reference. Audience: dialog UI redesign coordinator, DU-02b, DU-08.
> Companions: [work-packets.md](../work-packets.md), [dialog-ui-client-contract.md](../../../content/dialog-ui-client-contract.md), [du01.md](du01.md), [dul.md](dul.md), [TESTING.md](../../../../TESTING.md).

## Contract

- **Packet:** DU-02a — Castle_CellBlock button patches.
- **Scope shipped:** twelve `ButtonPlan::StripAll` rows, the matching deletion of 45 seed
  button rows, a two-directional patch-versus-seed agreement test, a chain-replay module
  firing all five keyed chains from the `-1` close path, the DU-L allowlist removal for
  3999, a UAT scenario and two doc notes.
- **Decisions in force:** Client Contract F1-F14 and both hard rules; D-DU1 (resolved:
  strip all four post-accept blurbs).
- **Depends on:** DU-01 (the patch engine), DU-L (the linter).
- **Blocks:** nothing. DU-08 and DU-06 interact with it; neither is gated on it.
- **Branch / base:** `dialog-ui/du02a-cellblock-buttons`, branched from
  `dialog-ui/wave1-base` @ `a9861f1f` (= `main` @ `192d4216` + DU-01 + DU-L + the
  coordinator's table-agnostic guard commit). Worktree `.claude/worktrees/du02a`.
- **Owned paths:**
  - `crates/services/src/base/dialog_overrides/patches_cellblock.rs`
  - `crates/services/src/cell/content/chain_replay_tests/cellblock_dialog_closes.rs` (new)
  - `crates/services/src/cell/content/chain_replay_tests/mod.rs` (one `mod` line)
  - `crates/content-engine/tests/dialog_button_linter.rs` (three edits, below)
  - `crates/content-engine/tests/dialog_button_linter/rules.rs` (allowlist)
  - `db/resources/Dialogs/Seed/dialog_screen_buttons.sql`
  - `db/resources/Content/Seed/castle_cellblock_chains.sql` (comments only, no rows)
  - `docs/analysis/castle-cellblock-rebuild/uat-guide.md`
  - `docs/content/mission-chains.md`
  - `docs/analysis/dialog-ui-redesign/worknotes/du02a.md` (this file)
- **Explicitly not touched:** `dialog_overrides/{mod,patch,emit,parse,patch_tests}.rs`,
  `patches_castle.rs`, every Castle chain seed, `cell/interactions/dialog.rs`,
  `cell_methods/player/interaction/dialog.rs`, `crates/content-engine/src/**`,
  `chain_replay_tests/mission_{638,640,641}.rs`, `work-packets.md`, `docs/readme.md`.
- **Read set:** the whole ledger; `dialog-ui-client-contract.md`; `du01.md` and `dul.md`;
  `patch.rs`, `patch_tests.rs`, `patches_cellblock.rs`; `triggers/matching.rs`;
  `cell/content/event_dispatch/dialog.rs`; `cell/cell_methods/player/interaction/dialog.rs`;
  `castle_cellblock_chains.sql`; `chain_replay_tests/{mod,mission_638,mission_640,
  mission_641,mission_708,mission_1324,mission_1326}.rs`; `dialog_button_linter.rs` and
  its four submodules; `TESTING.md` type 6; `CLAUDE.md`. Read-only, not in git and not
  read by any test: `data/cache/CookedDataDialogs.pak`.

## Audit of the twelve dialogs

Shipped layout read twice, independently: from the cooked PAK (python `zipfile`, entries
`_<id>`) and from `dialog_screens.sql` + `dialog_screen_buttons.sql`. **The two agreed
exactly for all twelve** — same screen count, same screen ids, same per-screen button
type/id/text, same order. No discrepancy to report.

| Dialog | Type | Screens | Shipped buttons | Final screen | Keyed chain | Plan |
|---:|---|---:|---|---:|---|---|
| 2299 | Dialog | 5 (96175-96179) | Accept (2/8) on all 5 | 96179, has one | 1019 | StripAll |
| 4001 | Dialog | 5 (96247-96251) | Accept on all 5 | 96251, has one | 1053 | StripAll |
| 5022 | Dialog | 10 (96261-96270) | Accept on 8; 96262, 96263 bare | 96270, has one | 1054 | StripAll |
| 3999 | Dialog | 9 (96252-96260) | Receive Item (4/70) on 7; **96259 and 96260 bare** | 96260, **bare** | 1058 | StripAll |
| 5023 | Dialog | 11 (96282-96292) | Receive Item on 9; 96283, 96284 bare | 96292, has one | 1059 | StripAll |
| 2309 | Dialog | 3 (96336-96338) | Accept on all 3 | 96338, has one | not keyed (displayed by 1171) | StripAll |
| 2516 | Dialog | 1 (96392) | Accept | 96392 | not keyed (displayed by 1161) | StripAll |
| 5859 | Blurb | 1 (96367) | Accept | 96367 | not keyed (displayed by 1172) | StripAll |
| 2305 | Blurb | 1 (18795) | Accept | 18795 | not keyed (displayed by 1151) | StripAll |
| 4000 | Blurb | 1 (96219) | Accept + More Info (1/9) | 96219 | not keyed (displayed by 1152) | StripAll |
| 2308 | Blurb | 1 (96323) | Accept + More Info | 96323 | **no chain at all** (1153 reserved) | StripAll |
| 2518 | Blurb | 1 (96406) | Accept | 96406 | not keyed (displayed by 1154) | StripAll |

45 seed button rows deleted in total.

**Correction to the ledger's Target Matrix.** It records 3999 as "Receive Item on 7 of 9".
True, but it understates the breakage: screens 96259 **and** 96260 are bare, so a player
reading to the end is two screens into a dead end, not one. The coordinator may want to
amend that row; I do not own the file.

## Evidence

**Where the weapon is actually granted (packet deliverable 6).** The "Receive Item" button
on 3999 and 5023 granted nothing, in either the old shape or the new one. Item 21 (the
SGHC 6 SMG) comes from `add_item` on the locker interact:

- trigger: `castle_cellblock_chains.sql:1194` — `(1055, 'interact_tag', 'Preparation_SMG1A', 'player', false, 0)`
- gate: `castle_cellblock_chains.sql:1197` — `(1055, 'step_status', 641, '2121', 'eq', 'active', 0)`
- grant: `castle_cellblock_chains.sql:1202` — `(1055, 'add_item', 21, NULL, '{"container": 1, "qty": 1}', 0, 0)`

Chains 1058 (`:1348-1362`) and 1059 (`:1372-1383`) carry only `advance_step 641 3564` plus
two `set_interaction_type` rows. No grant of any kind.

The ordering proves it rather than merely permitting it: 1053/1054 accept 641 (step 2121)
→ 1055 grants item 21 at the locker and advances to 80641 → 1066 (`item_equipped '21'`)
advances to 3563 → only then can 1056/1057 display 3999/5023. The SMG is in the backpack
*and equipped* before either dialog can open. `neither_second_briefing_chain_grants_an_item`
pins this so a future edit that moves the grant onto the dialog trips a test.

**Nothing else keys off the deleted `screen_button_id` values.** Repo-wide grep for
`screen_button_id` / `dialog_screen_buttons` outside the seed itself finds: the DU-L
linter's scanner, `db/database.sql`'s `\ir` lines and two sequence files, three agent-memory
notes, prose comments in four chain seeds and three replay-test doc comments, and one live
query in `mission_1326.rs:711` which counts buttons for dialogs **4375 and 4376** (Lantoc,
not in this packet). The column is a surrogate key with no foreign key pointing at it.

## Design decisions

**The agreement test lives in `patches_cellblock.rs`, as a `#[cfg(test)] mod tests`.**
The packet offered two homes and I took neither as stated, for a reason worth recording.

- *Not the content-engine linter crate.* It cannot see `CELLBLOCK_DIALOG_PATCHES`:
  `cimmeria-services` depends on `cimmeria-content-engine`, not the other way round. A
  linter-side test could only re-assert what the seed says, which is the half that is
  already covered.
- *Not a new module under `dialog_overrides/`.* That needs a `mod` line in `mod.rs`, which
  this packet may not edit (shared with DU-02b).
- *Not a live-DB test*, though `mission_1326.rs:711` is a precedent for counting seed
  buttons through the database. A live-DB test self-skips without `DATABASE_URL`, so a
  contributor running plain `cargo test` would see it pass while the seed and the patch
  disagreed. The file-parsing form always runs, in both CI gating jobs, and compares
  against the Rust constant the DB cannot see.

So the tests sit beside the table they guard, which also means DU-02b's equivalent lands
in `patches_castle.rs` and the two branches never touch the same file. The cost is a
~60-line seed scanner that will exist twice once DU-02b lands — see "Integration edits".

**Two tests, because one only fails in one direction.** `cellblock_patches_agree_with_the_
dialog_seed` iterates the patch table, so deleting a row makes it stop checking that dialog
rather than fail. `cellblock_patch_table_covers_exactly_the_du02a_roster` pins the twelve
ids and the plan on each, which is what fails when a row goes missing while the seed rows
stay deleted. Both proofs are recorded below.

**The scanner reads fields in front of `text`, so it needs no quote tracking.**
`dialog_screens.text` contains raw newlines and apostrophes, and a naive line scan drops
1,013 of its 13,467 rows (DU-L found this). Every field this scanner reads —
`dialog_id`, `screen_id`, `screen_button_id`, `button_id`, `button_type` — sits *before*
`text` in both column lists, so matching the full `INSERT INTO … VALUES (` header and
reading leading integers is exact. `the_seed_scan_reads_every_insert_row` pins the match
count against the raw `INSERT INTO <table>` count per file, so a format change fails loudly
instead of silently parsing a subset, and additionally asserts dialog 2298 still has
buttons so an empty map cannot pass.

**The replay tests assert an equivalence, not the strip.** No chain-replay context can see
a button; the decision to send `-1` is made inside the 2009 client. So
`cellblock_dialog_closes.rs` fires each chain twice — once with `button_id = -1`, once with
the cooked id the dialog used to send — and asserts both the exact seed action list and
that the two resolve identically. That equivalence is the load-bearing claim: it holds only
because `Trigger::OnDialogChoice` compares `dialog_id` and nothing else
(`triggers/matching.rs:141-143`). If DU-06 lands a `button_id` condition and anyone puts one
on these five chains, the close path silently stops resolving. The fixtures also reproduce
`fire_dialog_choice`'s omission of `archetype` (`event_dispatch/dialog.rs:88-95`), so an
archetype condition added to one of these chains cannot pass unnoticed, and each chain gets
a negative firing dialog 2298 so a trigger that matched everything would fail.

**`mission_638`, `mission_640` and `mission_641` are untouched**, which the packet requires
and which is part of the claim: they cover the interact and pickup halves of the same
missions and nothing about those changed.

**Declined: adding the five keyed dialogs to DU-L's `NEVER_ADD_A_BUTTON` list.** The
advisor recommended it, reasoning that a future packet could re-add an Accept to 4001 and
kill mission 641's accept path. That is not what would happen. Under F9 a chain fires on
the dialog id whatever button id arrives, so a button on the **final** screen still fires
the chain — R1 already draws exactly that line, and it is the shape DU-02b is deliberately
shipping for 2573/5861/2576. Putting these five on the never-add list would forbid a
legitimate future `OnlyOn` fix while guarding against nothing R1 misses. R1 covers them
correctly as zero-button keyed dialogs today.

**`#[allow(dead_code)]` on `ButtonPlan` must stay** (DU-01 asked the first Wave 1 packet to
drop it). My twelve rows construct only `StripAll`. `Keep` and `OnlyOn` are still
constructed nowhere outside `#[cfg(test)]`, so removing the attribute would fail clippy's
`-D warnings`. It can go when DU-02b's `OnlyOn` rows and DU-05's `Keep` rows have both
landed. I did not edit `patch.rs`, per the packet.

**DU-01's `shipped_patch_tables_are_empty_until_wave_1` landmine is already defused** by
the coordinator's `a9861f1f`, which replaced it with a guard that holds for filled tables.
Nothing was needed from me and `patch_tests.rs` is untouched.

## The DU-L edits, including one the packet did not anticipate

Three changes in the linter, all in files the packet assigns me:

1. `rules.rs` — 3999's `R1Exemption` deleted, array length 3 → 2, doc comment updated.
   5861 and 2576 are byte-identical to before (DU-02b owns them).
2. `dialog_button_linter.rs` — 3999's row deleted from
   `allowlisted_dialogs_still_ship_the_soft_locking_layout`, array length 3 → 2.
3. **`assert_r3_has_subjects`: three button-count floors lowered to 1.** This one was
   forced, and the coordinator should know why.

DU-L calibrated three vacuity floors on the pre-Wave-1 seed: referenced Blurbs carry ≥ 5
button rows, referenced `DialogWin` dialogs ≥ 30, and ≥ 10 of those are Generic1 (type 4).
Deleting most of those buttons is exactly what Wave 1 is for. Measured:

| Population | Before | After DU-02a | After DU-02a + DU-02b |
|---|---:|---:|---:|
| Blurb button rows (5 referenced Blurbs) | 7 | 2 | 2 |
| `DialogWin` button rows (32 referenced) | 53 | 15 | 3 |
| Generic1 (type 4) among those | 19 | 3 | 1 |

So the packet cannot land with those floors: DU-02a alone takes all three below them. I
lowered the *button*-count floors to 1 and left the *dialog*-count floors (≥ 3 Blurbs,
≥ 20 `DialogWin`, 2298 present, ≥ 15 keyed) at their original values, because those are the
real check that the reference scan still finds all thirty-seven dialogs. A floor of 1 still
catches the failure the guard was written for — a scan returning nothing, leaving R3's
inner loop unexecuted. The comment above the function records the end-state numbers.

**Chosen to be merge-stable:** setting them to 1 rather than to the post-DU-02a values
means DU-02b does not have to touch the same three lines again. Its branch will merge
clean here.

## Files

| File | What |
|---|---|
| `crates/services/src/base/dialog_overrides/patches_cellblock.rs` | twelve `StripAll` rows with per-row rationale; `tests` module with the seed scanner and three tests (407 lines) |
| `crates/services/src/cell/content/chain_replay_tests/cellblock_dialog_closes.rs` | new; 11 tests over chains 1019, 1053, 1054, 1058, 1059 (359 lines) |
| `crates/services/src/cell/content/chain_replay_tests/mod.rs` | one `mod` line, alphabetically placed so `cargo fmt` leaves it alone |
| `crates/content-engine/tests/dialog_button_linter/rules.rs` | 3999 out of `R1_ALLOWLIST` |
| `crates/content-engine/tests/dialog_button_linter.rs` | 3999 out of the layout pin; three R3 vacuity floors to 1 |
| `db/resources/Dialogs/Seed/dialog_screen_buttons.sql` | 45 rows deleted; header note recording the removal and pointing at the patch table |
| `db/resources/Content/Seed/castle_cellblock_chains.sql` | two comments, no rows: the 2516/5859 eviction note and the 2308 note |
| `docs/analysis/castle-cellblock-rebuild/uat-guide.md` | scenario T30 (six checks) + three Results rows |
| `docs/content/mission-chains.md` | zero-button notes on the 638 and 641 sections |

## Commands run

All through the lane wrapper from `/c/Users/Steve/source/projects/Cimmeria/.claude/worktrees/du02a`,
with `L=/c/Users/Steve/AppData/Local/Temp/cimmeria-castle` and `CARGO_BUILD_JOBS=4` on every
call, as the coordinator's memory-pressure notice requires. No out-of-memory, paging-file,
`STATUS_NO_MEMORY` or `LNK1102` failure occurred.

| Command | Result |
|---|---|
| `$L/lane.sh cargo test -p cimmeria-content-engine --test dialog_button_linter` | exit 0 — **23 passed, 0 failed**, no skips (no DB involved) |
| `$L/lane.sh cargo check -p cimmeria-services --all-targets` | exit 0, no warnings |
| `$L/live-db-test.sh cellblock_dialog_closes patches_cellblock` | exit 0 — **15 run, 15 passed**, 2927 skipped |
| `$L/live-db-test.sh mission_638 mission_640 mission_641 dialog_overrides` | exit 0 — **78 run, 78 passed**, 2864 skipped |
| `$L/lane.sh cargo fmt --all` | exit 0; touched only this packet's files |
| `$L/lane.sh cargo +1.98.1 clippy -p cimmeria-services -p cimmeria-content-engine --all-targets -- -D warnings` | exit 0, clean |

**Live-DB status: it ran, it did not self-skip.** `live-db-test.sh` reloaded
`sgw_du02a` from this worktree's `db/database.sql` and exported
`DATABASE_URL=postgres://w-testing:w-testing@localhost:5433/sgw_du02a`; the run reports
tests *run*, not skipped, and the chain-replay tests cannot load a chain without a pool.
The seed deletion does touch a loader path (`dialog_screen_buttons` is read into
`resources`), which is why the reload was worth doing rather than relying on the file-parse
tests alone.

Per TESTING.md's picker this packet is type 6 (chain-replay) plus a no-DB seed-consistency
test of the kind the "seed linters" sibling section describes. No wire-format test: DU-01
already pins the emitter byte-exactly and this packet adds no new emitted shape. No test
reads `data/cache/`.

## Regression proof

Three mutations, each reverted by an inverse edit, with `git status --porcelain -- db/`
confirming the seed came back to the intended diff. No `git stash` was used.

### (a) A seed button row re-added for a patched dialog → the agreement test fails

Appended 4001's final-screen Accept back into `dialog_screen_buttons.sql`:

```sql
INSERT INTO dialog_screen_buttons (screen_button_id, button_id, screen_id, button_type, text) VALUES (3082, 8, 96251, 2, 'Accept');
```

`$L/live-db-test.sh patches_cellblock` → exit 100:

```text
dialog 4001: the patch strips every button, but the seed still has
[SeedButton { screen_id: 96251, button_type: 2, button_id: 8 }]. The client would show no
button and the seed would claim one — delete those rows from
db/resources/Dialogs/Seed/dialog_screen_buttons.sql.
```

Worth noting **the DU-L linter stays green on this mutation**: a button on 4001's final
screen satisfies R1. That is precisely the gap the agreement test exists to close.

### (b) A patch row deleted while the seed rows stay deleted → the roster test fails

Removed 2299's `DialogPatch` from `CELLBLOCK_DIALOG_PATCHES`. Same run, same command:

```text
CELLBLOCK_DIALOG_PATCHES no longer covers exactly the DU-02a roster. A row removed here
without restoring its dialog_screen_buttons.sql rows leaves the client showing buttons the
seed says are gone. …
  left: [2305, 2308, 2309, 2516, 2518, 3999, 4000, 4001, 5022, 5023, 5859]
 right: [2299, 2305, 2308, 2309, 2516, 2518, 3999, 4000, 4001, 5022, 5023, 5859]
```

(a) and (b) ran together: `3 tests run: 1 passed, 2 failed`, each naming its own dialog.
Both reverted; the suite returned to 3 passed.

### (c) 3999's seven buttons restored → R1 fires, with no allowlist left to suppress it

Re-added rows 3069-3075 (screens 96252-96258, type 4 id 70 "Receive Item").
`$L/lane.sh cargo test -p cimmeria-content-engine --test dialog_button_linter` → exit 101,
**22 passed / 1 failed**:

```text
R1 dialog 3999: keyed by castle_cellblock_chains.sql:chain 1058. Buttons on screen(s)
[96252, 96253, 96254, 96255, 96256, 96257, 96258], but final screen Some(96260) has none.
A player who pages to the end sees only Done; Done is a close, and closing a dialog that
HAS buttons sends nothing (F8), so the chain never fires and the player is soft-locked. …
```

This is the proof that removing the allowlist entry has teeth: before this packet the same
seed shape passed, suppressed by the exemption. Reverted; 23 pass again.

## mission-systems-advisor review

Consulted read-only after the design was drafted, with the twelve-row table, the five
chains' resolved actions, and five specific double-advance / skip scenarios. **Overall
verdict: SAFE to land as specified.** Per question:

| # | Question | Verdict |
|---|---|---|
| 1 | Early close commits after one screen | **SAFE** — the early-out already ships. Accept sits on `index 0` for 2299 (96175), 4001 (96247), 5022 (96261) and 5023 (96282). The strip removes the duplicate button, not the commit-early path. No Cellblock chain can depend on reaching the end: `display_dialog` renders every screen in one action and there is no per-screen event. |
| 2 | Repeat close double-fires an unconditional chain | **SAFE.** `handle_dialog_button_choice` rejects unless `open_dialog_id == dialog_id` and clears the pin *before* firing (`cell_methods/player/interaction/dialog.rs:39-55`), so a replayed `-1` is dropped: one choice per display. A second *display* is gated by the display chain in every case (1051/1052 on `mission_status(641) = not_active`, 1056/1057 on `step_status(641,3563) = active`, 1021/1019 on `step_status(638,2116) = active`), and each choice chain's own action invalidates that gate. Non-blocking RISK: the four choice chains are safe only because a *different* chain is gated — see below. |
| 3 | The seven unkeyed dialogs become closeable with a `-1` | **SAFE.** Full sweep of all eleven files under `db/resources/Content/Seed/`: none of 2309, 2516, 5859, 2305, 4000, 2308, 2518 appears as a `dialog_choice` key anywhere. Their closes resolve zero actions and log at debug. One real side effect: `cimmeria_discord::emit_dialog` fires before the resolve (`event_dispatch/dialog.rs:103`), so seven extra off-by-default gameplay events per playthrough. |
| 4 | 2298's display evicting a now-button-less 2299 | **SAFE, premise wrong on both sides.** Server-side the pin is already `None` when chain 1019's first action runs. Client-side the `-1` *is* the discard — it is emitted from the path that frees the slot, so by the time 2298 arrives there is nothing to evict. DU-08 makes this strictly better, not worse. |
| 5 | 2516 at `delay_ms 10100` vs 5859 at 10600 | **RISK, log-level only.** A player who has not closed 2516 within ~500 ms has it evicted; the `(2516, -1)` arrives after the server re-pinned to 5859 and is rejected with a `warn!` — a forgery/replay signal now firing on every run of the scene. No progression hazard (nothing keys 2516). DU-08 removes it. |
| — | "Receive Item" never granted anything | **CONFIRMED**, with the file:line evidence reproduced above. |

**Acted on:** added the 2516/5859 note beside chain 1172 and the 2308 note beside the
reserved 1153 block, both comment-only; corrected 3999's bare-screen count to two
throughout; recorded the DU-L two-deletion requirement (which the packet already had).

**Not acted on, and why:**

- *Self-gating chains 1053/1054/1058/1059* with the mirror condition their display chain
  carries. It is good advice and costs nothing at resolve time, but it changes seed rows in
  four chains that are provably safe today, and the ledger assigns exactly this audit to
  DU-08. Widening DU-02a into it would put a behaviour change in a packet whose whole claim
  is that behaviour is unchanged. **Recommended to the coordinator as a DU-08 follow-up.**
- *Extending the never-add-a-button list* — see "Design decisions" for the reasoning.

## Known gaps

1. **The seed scanner will exist twice.** DU-02b needs the same ~60 lines in
   `patches_castle.rs`. Neither packet may add a module to `dialog_overrides/mod.rs`, so
   the duplication is structural until the coordinator extracts a shared `seed_scan.rs`.
   See "Integration edits".
2. **Nothing verifies the patch against the PAK in CI.** The cooked archive is not in git,
   so the agreement test compares the patch to the *seed*, and the seed-to-PAK agreement
   was established by hand (this packet) and is not re-checked. A PAK that drifts from the
   seed would make the patch's `screen_id`s stale — the engine handles that gracefully
   (`warn!` + keep the canonical entry, DU-01), it just would not be caught early.
3. **No test observes the `warn!` from the 2516 eviction.** It is emitted from a handler
   this packet may not touch; a `LogCapture` guard belongs with DU-08, which changes that
   code.
4. **T30 is unrun.** The whole packet is untested in the client. The scenario and its three
   Results rows are written for the next UAT pass.
5. **Test-inventory docs not updated.** 14 added tests is far under CLAUDE.md's
   ~147-test (5%) threshold for a per-PR inventory edit; it rolls up in the next sweep.

## Integration edits the coordinator must make

1. **Ledger.** Mark DU-02a done. Two corrections to the Target Matrix: 3999 is bare on
   **two** screens (96259 and 96260), and 5023's final screen 96292 **does** carry its
   button — 5023 was never R1-violating, it is stripped for consistency with 3999 and
   because the label is misleading.
2. **DU-02b must not re-raise the R3 floors.** `assert_r3_has_subjects`'s three
   button-count floors are now 1 and are correct for the post-Wave-1 end state (2 Blurb
   rows, 3 `DialogWin` rows, 1 Generic1). DU-02b should leave them; its own edits to
   `R1_ALLOWLIST` and the layout pin will merge cleanly alongside mine.
3. **Shared seed scanner.** Once both zone packets have landed, consider extracting the
   scanner in `patches_cellblock.rs::tests` and `patches_castle.rs::tests` into a
   `dialog_overrides/seed_scan.rs` with a `#[cfg(test)]` `mod` line. Neither packet could
   do it without editing the shared `mod.rs`.
4. **`#[allow(dead_code)]` on `ButtonPlan` stays for now.** DU-01's note says the first
   Wave 1 packet should drop it; it cannot go until `OnlyOn` (DU-02b) and `Keep` (DU-05)
   both have non-test constructors.
5. **DU-08 follow-up.** Add the mirror gate to chains 1053, 1054, 1058 and 1059, and a
   `LogCapture` guard for the 2516 eviction rejection. Both fall inside DU-08's stated
   audit scope.
6. **Doc index.** Nothing new to index — `uat-guide.md`, `mission-chains.md` and the
   worknotes directory are all already listed.

## Log

- 2026-09-21 — read the ledger, the client contract, `du01.md` and `dul.md`; dumped all
  twelve dialogs from the PAK and cross-checked every screen and button against the seed;
  traced the five keyed chains and located the item-21 grant.
- 2026-09-21 — commissioned the mission-systems-advisor review on the five double-advance
  scenarios while writing the patch table.
- 2026-09-21 — twelve `StripAll` rows, 45 seed rows deleted, agreement tests, DU-L
  allowlist and layout-pin edits; discovered the R3 vacuity floors were calibrated on the
  pre-Wave-1 button population and had to be lowered.
- 2026-09-21 — `cellblock_dialog_closes.rs`; ran the three regression proofs and restored;
  fmt, `+1.98.1` clippy, the two live-DB runs.
- 2026-09-21 — UAT scenario T30, the two `mission-chains.md` notes, the two chain-seed
  comments the advisor's Q5 and the 2308 finding called for, and this worknote.
