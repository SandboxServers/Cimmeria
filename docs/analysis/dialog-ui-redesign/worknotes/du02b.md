# DU-02b Worknotes

> Type: reference. Audience: dialog UI redesign coordinator, the Castle rebuild campaign, and whoever runs the M1b UAT.
> Companions: [work-packets.md](../work-packets.md), [du01.md](du01.md) (the patch engine), [dul.md](dul.md) (the linter), [dialog-ui-client-contract.md](../../../content/dialog-ui-client-contract.md), [castle-rebuild/README.md](../../castle-rebuild/README.md).

## Contract

- **Packet:** DU-02b — Castle button patches (dialogs 2573, 5861, 2576).
- **Scope shipped:** three `ButtonPlan::OnlyOn` rows in the Castle patch table; the matching seed button rows moved; a patch-versus-seed agreement test plus a cooked-entry guard; four chain-replay cases keyed on the real wire `ButtonID`; the DU-L allowlist emptied of 5861 and 2576; GAP 3 and D-CA13's Accept gap marked resolved; an M1b UAT row.
- **Decisions in force:** Client Contract facts F1-F14 and both hard rules; D-CA13 (Jaffa branch of 701); D-CA05 (2576 hands out 703 with 702); D-DU2 (defer the 2572 offer flow — this packet ships without it).
- **Depends on:** DU-01 (the patch engine) and DU-L (the linter), both already on the base branch.
- **Blocks:** nothing. DU-06 will want the `button_id` condition these tests pin the absence of.
- **Branch / base:** `dialog-ui/du02b-castle-buttons`, based on the integration branch `dialog-ui/wave1-base` @ `a9861f1f` (= `main` @ `192d4216` + DU-01 + DU-L + ledger updates). Worktree `.claude/worktrees/du02b`. Not rebased onto `main`.
- **Owned paths:**
  - `crates/services/src/base/dialog_overrides/patches_castle.rs`
  - `crates/services/src/base/dialog_overrides/patch_seed_agreement_castle.rs` (new)
  - `crates/services/src/cell/content/chain_replay_tests/mission_701/dialog_buttons.rs` (new)
  - `db/resources/Dialogs/Seed/dialog_screen_buttons.sql` (three dialogs only)
  - `db/resources/Content/Seed/castle_701_chains.sql` (comments only)
  - `docs/analysis/castle-rebuild/README.md`, `audit.md`, `worknotes/m701.md` (status rows only)
  - `docs/analysis/dialog-ui-redesign/worknotes/du02b.md` (this file)
- **Shared files touched, minimally:** `crates/services/src/base/dialog_overrides/mod.rs` (+2 lines), `crates/services/src/cell/content/chain_replay_tests/mission_701/mod.rs` (+1 line), `crates/content-engine/tests/dialog_button_linter.rs` and `dialog_button_linter/rules.rs` (allowlist removal).
- **Explicitly not touched:** `patches_cellblock.rs`, `castle_cellblock_chains.sql`, `patch.rs`, `emit.rs`, `parse.rs`, `patch_tests.rs`, `cell/interactions/dialog.rs`, `cell_methods/player/interaction/dialog.rs`, `crates/content-engine/src/**`, `work-packets.md`, `docs/readme.md`, dialogs 2572, 30, 2042, 2043, 2044. No `ui_screen_type` changed.
- **Read set:** the whole campaign ledger; `dialog-ui-client-contract.md`; the DU-01 and DU-L worknotes; `dialog_overrides/{mod,patch,parse,emit,patch_tests}.rs`; `dialog_button_linter.rs` and its four submodules; `castle_701_chains.sql` in full; `castle_702_704_chains.sql` references; `chain_replay_tests/mission_701/{mod,arrival,body}.rs`; `content/event_dispatch/dialog.rs`; `db/resources/Dialogs/Seed/{dialogs,dialog_screens,dialog_screen_buttons}.sql`; `docs/analysis/castle-rebuild/{README,audit}.md` and `worknotes/m701.md`. Read-only, and one test now reads it: `data/cache/CookedDataDialogs.pak` entries `_2573`, `_5861`, `_2576`.

## Evidence

Both records were read before either was written, on 2026-09-21.

| Dialog | Screens (by `index`) | Final screen | Shipped buttons | Cooked entry agrees? |
|---:|---|---:|---|---|
| 2573 | 113552-113558 (7) | **113558** | Accept `ButtonType 2 ButtonID 8 "Accept"` on all 7 | yes, byte for byte |
| 5861 | 96782-96789 (8) | **96789** | Accept on 96782-96786; 96787-96789 bare | yes |
| 2576 | 96821-96825 (5) | **96825** | `ButtonType 4 ButtonID 71 "Take Missions"` on 96821-96823; 96824/96825 bare | yes |

The seed and `CookedDataDialogs.pak` did not disagree anywhere, and every final screen id matched the one the packet named, so there was nothing to stop and report. All three dialogs are `ui_screen_type` 2 (`DUIST_DefaultDialog`), whose window draws button types 2 and 4 — both plans are drawable.

`screen_button_id` turned out to be a bare primary key: a `serial` column with `dialog_screen_buttons_2_pkey` and a sequence default, referenced by no foreign key in `db/resources/` and by no Rust code. Which id a surviving row carries is therefore bookkeeping.

## Design decisions

**One button, on the final screen, keeping the shipped `ButtonID`.** The client resolves a click to a position in the screen's button array and puts the cooked `ButtonID` at that position on the wire (F14). Changing the id would silently re-point every future `button_id` condition, and the Castle chains were authored against 8 and 71. The ids, types and texts are copied from the shipped data, not invented.

**Which seed row survives.** One row per dialog, reusing an id the dialog already had rather than minting one from the sequence. The rule, applied in order: keep the row that is *already* on the final screen if there is one; otherwise keep the dialog's lowest `screen_button_id` and repoint its `screen_id`.

| Dialog | Kept | Why | Deleted |
|---:|---:|---|---|
| 2573 | `2239` | already on 113558, so the surviving row is byte-identical | 2233-2238 |
| 5861 | `4226` | lowest id of the block, repointed 96782 → 96789 | 4227-4230 |
| 2576 | `2240` | lowest id of the block, repointed 96821 → 96825 | 2241-2242 |

**2573 is a design change, not a bug fix, and it is called out as such.** It satisfies the hard rule today. Reducing it to one Accept on screen 7 makes the player read the briefing before committing, matches what 5861 and 2576 now do, and stops the client drawing its automatic Decline beside Accept on every screen (F6). The `mission-systems-advisor` confirmed nothing in the 701, 702 or 703 chain sets or the Castle rebuild docs depends on accepting from screen one.

**Where the agreement test lives: `cimmeria-services`, not the linter crate.** This was the packet's open choice and the dependency graph settles it. `CASTLE_DIALOG_PATCHES` is in `cimmeria-services`, and `cimmeria-content-engine` does not depend on `cimmeria-services` — the arrow points the other way (`crates/services/Cargo.toml:13`). The linter crate physically cannot see the patch table, so a test that compares the two has to sit on the services side and read the seed SQL from the repo path, which is what `base/resources/tests/committed_paks.rs` and `mercury/protocol/tests.rs` already do with `CARGO_MANIFEST_DIR`. It is a new file whose name carries `castle` so DU-02a's `cellblock` equivalent never collides with it.

**A third record needed guarding, and it is the one that matters.** The `mission-systems-advisor` review found the real hole. `apply_dialog_patch` refuses an `OnlyOn` whose `screen_id` is absent from the cooked entry and the caller keeps the canonical bytes (`patch.rs` `PatchError::ScreenMissing`) — correct fail-closed behaviour, and completely silent. With 5861 and 2576 removed from the DU-L allowlist, a typo in a screen id would have left no red test anywhere: the seed would say fixed, the linter would agree, and the player would still page to a bare final screen. So `castle_patches_apply_to_the_committed_cooked_entries` runs every plan against the committed archive and asserts the button lands on the cooked entry's last screen in document order.

`data/cache/CookedDataDialogs.pak` **is in git** (`git ls-files data/cache/` lists it with nineteen sibling archives). The DU-01 acceptance note and the DU-L module doc both say the pak is not in git; that is wrong, and `base/resources/tests/committed_paks.rs` has been reading the directory all along. The entry is read with `zip::ZipArchive` directly rather than through `ResourceCache::load_pak`, only because that helper is private to the `resources` module.

**The negative case has no server-side form.** Once a dialog carries any button, Done, the title-bar X and the automatic Decline all send **nothing** (F8). There is no `dialogButtonChoice` frame for an early close, so there is no `TriggerEvent` to replay and no behaviour to assert. "Closing 2576 early grants nothing" is true because the server never hears about it. A replay test fed a synthetic event would be testing a packet the client does not send, so none was written; the finding is recorded in the module doc of `dialog_buttons.rs` instead. What protects the player is re-entry, and that *is* server-side — see the next section.

**No chain row changed.** `dialog_choice` matches on dialog id alone (F9), so where a button sits cannot change what a chain resolves. The existing `arrival.rs` and `body.rs` cases were left untouched and stayed green, which is the evidence for that claim rather than an assumption about it.

## Advisor review — re-entrancy

Consulted `mission-systems-advisor`, read-only, no model override. Verdict: **the re-entrancy design is sound.** Its evidence, which I re-checked:

- **2576.** Chain 1236 re-displays it on any later `interact_tag Castle_Coppleman` while `step_status 701 2421 eq active` (`castle_701_chains.sql:464-470`), `once = false`. The bind that makes Copplemann clickable (3063 slot 48) is added by 1235 and re-armed on login by 1242/1243, and removed in exactly one place: chain 1237's first action, which runs only on the choice (`:497-500`). So an early close leaves her clickable and the step active. 702 and 703 cannot be accepted twice — 1238 and 1239 each carry `mission_status 70x eq not_active` on top of the step gate (`:510-516`, `:537-543`). 701 cannot complete twice: `MissionInstance::complete()` moves 2421 out of `current_step_id`, so 1237's `eq active` gate is permanently false afterwards.
- **2573 / 5861.** Chains 1202 and 1203 re-display on `interact_tag Castle_SgtGerschon` while `step_status 701 2399 eq not_active`, split by archetype, both `once = false` (`:172-184`, `:193-205`). The 3062 slot-149 bind is dropped only by 1204/1205 on the choice. 701 is protected from a double accept by `mission_status 701 eq not_active` on both accept chains, by the offer chains ceasing to match once 2399 goes active, and by `handle_dialog_button_choice` rejecting a choice whose dialog is not the player's currently-open one.
- **Relog mid-flow** is covered by chains 1201, 1240, 1241, 1242 and 1243. Duplicate binds accumulate, OR-fold, and are cleared wholesale by `remove_dialog_set` — existing behaviour, unchanged here.
- **`once` flags:** every trigger row in the file is `once = false`.

Three findings from the review landed in this packet: the cooked-entry guard above, chain 1203's wrong "the missing buttons are cosmetic here" comment, and chain 1236 citing screen 96825 as carrying speaker 1110 when its `speaker_id` is 0 (the conclusion it supports still holds on 96821 and 96823).

One finding is **out of scope and reported, not fixed**: the advisor says GAP 1 in the same header is stale — CA02 landed, `DialogSetMapEntry.dialog_id` is `Option<i32>` and NULL-dialog rows are kept, so Gerschon and Copplemann really are clickable. That block is eight paragraphs of CA02 contingency and rewriting it is the Castle campaign's call, not this packet's.

## Files

| File | What |
|---|---|
| `crates/services/src/base/dialog_overrides/patches_castle.rs` | three `OnlyOn` rows; `#[allow(unused_imports)]` dropped now that the imports are used |
| `crates/services/src/base/dialog_overrides/patch_seed_agreement_castle.rs` | new — four tests, a quote-aware seed scanner, and the cooked-entry guard |
| `crates/services/src/base/dialog_overrides/mod.rs` | +2 lines: `#[cfg(test)] mod patch_seed_agreement_castle;` |
| `crates/services/src/cell/content/chain_replay_tests/mission_701/dialog_buttons.rs` | new — four replay cases on the real wire ids |
| `crates/services/src/cell/content/chain_replay_tests/mission_701/mod.rs` | +1 line: `mod dialog_buttons;` |
| `db/resources/Dialogs/Seed/dialog_screen_buttons.sql` | 12 rows deleted, 2 repointed; nothing else in the file touched |
| `crates/content-engine/tests/dialog_button_linter/rules.rs` | 5861 and 2576 out of `R1_ALLOWLIST` (3 → 1) |
| `crates/content-engine/tests/dialog_button_linter.rs` | their rows out of `allowlisted_dialogs_still_ship_the_soft_locking_layout` |
| `db/resources/Content/Seed/castle_701_chains.sql` | comments only: GAP 3 resolved, chain 1203's wrong claim replaced, chain 1236's screen citation corrected |
| `docs/analysis/castle-rebuild/README.md` | D-CA13's Accept gap resolved; new M1b UAT row |
| `docs/analysis/castle-rebuild/audit.md` | the 5861 row marked resolved |
| `docs/analysis/castle-rebuild/worknotes/m701.md` | gap 3 flipped from "Filed, not fixed" |

## Commands run

All through the lane wrapper from the `du02b` worktree, each prefixed `CARGO_BUILD_JOBS=4`.

| Command | Result |
|---|---|
| `lane.sh cargo check -p cimmeria-services --all-targets` | exit 0 |
| `lane.sh cargo test -p cimmeria-services --lib patch_seed_agreement -- --test-threads=1` | **4 passed, 0 failed** |
| `lane.sh cargo test -p cimmeria-content-engine --test dialog_button_linter` | **23 passed, 0 failed** |
| `live-db-test.sh mission_701` | **52 run, 52 passed**, 2883 skipped. `DATABASE_URL` was set by the wrapper to `postgres://w-testing:w-testing@localhost:5433/sgw_du02b`, so the live-DB replay tests **ran** rather than self-skipping. 48 of the 52 are the pre-existing mission-701 cases, unmodified. |
| `live-db-test.sh dialog` | **111 run, 111 passed**, 2824 skipped. Same database, same reload. Run because the seed change touches a resource table; no loader broke. Nothing self-skipped in either live-DB run. |
| `lane.sh cargo fmt --all` then `cargo fmt --all -- --check` | exit 0, clean |
| `lane.sh cargo +1.98.1 clippy -p cimmeria-services -p cimmeria-content-engine --all-targets -- -D warnings` | exit 0, clean |

The live-DB run is the one that matters for the seed change: `dialog_screen_buttons.sql` is loaded by `db/database.sql`, and `live-db-test.sh` wipes and reloads `sgw_du02b` from this worktree's copy before running, so the 52 passes are against the moved rows.

Per TESTING.md's picker this is type 6 (chain-replay) plus type 3 (live-DB) for the seed, and a unit guard for the patch-versus-seed and patch-versus-cooked agreement. No wire format changed, so no type 2 test is owed.

## Regression proof

Three reverts, each run against the restored tree afterwards. The good state was committed first, so `git checkout --` restored it.

**Revert 1 — the seed drifts away from the patch.** Put 2576's surviving seed row back on screen 96821 and left the patch on 96825.

- `castle_patches_agree_with_the_committed_button_seed` **FAILED**, printing `left: [(96821, 4, 71, "Take Missions")]` against `right: [(96825, …)]`.
- The other three services-side guards stayed green, so the agreement test is not being carried by its neighbours.
- The DU-L linter **also FAILED** on R1 with `dialog 2576: … Buttons on screen(s) [96821], but final screen Some(96825) has none`, confirming that removing 2576 from the allowlist really did arm the linter.

**Revert 2 — the patch names a screen the cook never shipped.** Changed 5861's target from 96789 to 96780, leaving the seed correct. This is the silent case the advisor found.

- `castle_patches_apply_to_the_committed_cooked_entries` **FAILED** with `ScreenMissing { screen_id: 96780 }`.
- The two seed-side guards also failed, as expected, because the seed then disagrees with the patch. The DU-L linter would **not** have caught this: the seed is untouched and still satisfies R1.

**Revert 3 — the button goes back to a mid-dialog screen in BOTH records.** 2576's patch and seed row both moved to 96821, which is the shape the game shipped.

- `castle_only_on_patches_target_the_final_screen` **FAILED**: `the patch lands its only button on screen 96821, but the dialog's screens in dialog_screens.index order are [96821, 96822, 96823, 96824, 96825]`.
- `castle_patches_apply_to_the_committed_cooked_entries` **FAILED** on the same shape read from the cooked document.
- `castle_patches_agree_with_the_committed_button_seed` **PASSED** — the two records agree with each other and are both wrong. That is what proves the final-screen rule is an independent guard and not a restatement of the agreement test.

All three reverts were undone and the suite returned to 4 passed, 0 failed.

## UAT rows

Added as milestone **M1b** in `docs/analysis/castle-rebuild/README.md`. Repeated here because there is no separate Castle UAT guide — the gates table in that README is where those rows live.

**Every row needs a server restart first.** Cooked-entry patches apply once at PAK load; there is no hot reload.

1. **Human offer (2573).** Interact with Sgt. Gerschon as any non-Jaffa archetype. Screens 1-6 show Next with no Accept. Screen 7 of 7 shows Accept and Decline. Accept starts mission 701 and the quest topic moves to Capt. Copplemann.
2. **Jaffa offer (5861).** The same as a Jaffa (archetype 8). Accept and Decline appear only on screen 8 of 8. Accepting starts 701 and the Moh'katan radio call (5862) follows.
3. **Turn-in (2576).** At step 2421, click Copplemann and page to screen 5 of 5. Press **Take Missions**: 701 completes, and "Rescue Dr. Zuritska" (702) and "Payback" (703) both arrive.
4. **Early close.** Open 2576 and close it with the title-bar X before the last screen. Nothing is granted, no error appears, and Copplemann can be clicked again to re-open the same dialog. Repeat for 2573 or 5861 with Gerschon.

Expected chrome, **not** defects:

- On 2573 and 5861 the final screen shows Next-and-Previous rather than Done, because the client swaps the pair whenever Accept is visible. Close with X or Decline; both send nothing, which is why re-entry exists.
- On 2576's final screen Done sits beside Take Missions and sends nothing. 2576's button is a Generic (type 4), not Accept, so the client keeps drawing Done.
- Decline never reaches the server on any of the three. There is no way to distinguish "declined" from "ignored".

## Known gaps

- **No in-client verification.** Nothing here has been seen in the game. The cooked-entry guard proves the patch applies and lands where intended; it cannot prove the client draws what we expect.
- **`ButtonPlan` still carries `#[allow(dead_code)]`** in `patch.rs`. DU-01's worknote asks the first Wave 1 packet to remove it, but `patch.rs` is off limits to this packet and the attribute is harmless on a now-used item. DU-02a or the coordinator should drop it.
- **The GAP 1 block in `castle_701_chains.sql` is stale** (see the advisor section). Out of scope here.
- **DU-01's and DU-L's "the pak is not in git" claims are wrong.** Worth correcting in `work-packets.md` (DU-01's acceptance line) and in the `dialog_button_linter.rs` module doc, neither of which this packet owns.
- **`dialog_screens.sql` was not touched.** 2576's screen 96824 has no button and never had one; no screen was added or removed anywhere.

## Integration notes for the coordinator

**Three merge points with DU-02a**, all small and all mechanical:

1. `crates/content-engine/tests/dialog_button_linter/rules.rs` — both branches edit `R1_ALLOWLIST`. This branch takes it from three entries to one (3999); DU-02a takes it from three to two by removing 3999. Resolve to an **empty** array, `[R1Exemption; 0] = []`. Note `rustfmt` collapsed the single-entry array to an inline literal on this branch, so the conflict hunk looks bigger than the change is.
2. `crates/content-engine/tests/dialog_button_linter.rs` — the same shape in `allowlisted_dialogs_still_ship_the_soft_locking_layout`. Resolve to an empty `expected` table. At that point the whole test becomes a no-op loop; consider deleting it and saying so in the ledger, since its stated job ("the allowlist is empty at the end of Wave 1") is then done by `r1_violations`'s stale-entry arm.
3. `crates/services/src/base/dialog_overrides/mod.rs` — this branch adds two lines (`#[cfg(test)] mod patch_seed_agreement_castle;`); DU-02a will add two for `..._cellblock`. `castle` sorts before `cellblock`, so the merged order is already what `rustfmt` wants. Keep both.

**Ledger edits this packet cannot make** (`work-packets.md` is the coordinator's):

- DU-02b → Done. Target Matrix rows 2573, 5861 and 2576 are shipped.
- The DU-L row's "the allowlist is empty at the end of Wave 1" is now half true: 3999 remains, pending DU-02a.
- DU-01's acceptance line says "The PAK is not in git, so no test may read `data/cache/`." That is factually wrong and this packet deliberately depends on it being wrong. Either correct the line or record the exception.
- The Client Contract's hard-rule paragraph names 3999, 5861 and 2576 as shipping broken; 5861 and 2576 are fixed.
- D-DU2 stays deferred; nothing here needs the `button_id` condition, and `the_turn_in_does_not_filter_on_button_id_today` will fail loudly if DU-06 lands one with the sense inverted.

**No `docs/readme.md` or index edit is owed.** No new doc was added outside this worknote and the campaign's own worknote directory.

## Log

- 2026-09-21 — read the ledger, the client contract, the DU-01 and DU-L worknotes, the 701 chain seed in full, and the Castle audit; dumped the seed rows and the cooked entries for 2573, 5861 and 2576 and confirmed the two agree.
- 2026-09-21 — wrote the three patch rows; moved the twelve seed rows; added the agreement test and the two non-vacuity guards; emptied the DU-L allowlist of 5861 and 2576.
- 2026-09-21 — consulted `mission-systems-advisor` on re-entrancy; it cleared the design and found the `ScreenMissing` blind spot, which became `castle_patches_apply_to_the_committed_cooked_entries`, plus two wrong comments in the chain seed.
- 2026-09-21 — added the four replay cases on the real wire ids; ran the live-DB mission-701 suite against `sgw_du02b`.
- 2026-09-21 — flipped GAP 3, D-CA13, the audit row and m701's gap table; added the M1b UAT row.
- 2026-09-21 — fmt, `+1.98.1` clippy, three regression reverts with restore, push.
