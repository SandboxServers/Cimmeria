# Dialog button strips: seed/patch agreement, and the linter floors they break

Learned landing DU-02a (twelve Castle_CellBlock dialogs stripped to zero buttons).
Related: [[content-chain-dispatch-traps]], [[chain-replay-trigger-param-vacuity]],
[[vacuous-guard-and-sentinel-collision-review]], [[dialog-set-bind-routing-and-edges]].

## 1. A seed-linter vacuity floor calibrated on today's data blocks the packet whose job is to change that data

`crates/content-engine/tests/dialog_button_linter.rs`'s `assert_r3_has_subjects`
asserted "referenced Blurbs carry >= 5 button rows", ">= 30 DialogWin rows",
">= 10 Generic1". Every one of those numbers was a snapshot of the pre-Wave-1
population. Deleting most of those buttons is exactly what the fixing packets do,
so the floors went unsatisfiable: DU-02a alone takes them to 2 / 15 / 3, and after
DU-02b it is 2 / 3 / 1.

**Rule:** a vacuity guard's job is "the inner loop executed", so its floor is 1.
Keep the *population* floors (how many dialogs / rows the scan FOUND) at their real
values — those are what catch a broken scanner. Setting the floor to 1 rather than
to the current value also makes the file merge-stable across two parallel packets
that both reduce the same population.

## 2. A patch-vs-seed agreement test needs BOTH directions or it is half a guard

`db/resources/Dialogs/Seed/dialog_screen_buttons.sql` (what the linter reads) and
`dialog_overrides/patches_*.rs` (what the client receives) are synced by hand; F1
means the seed changes nothing in game. The obvious test iterates the patch table
and compares each row to the seed — but **deleting a patch row makes that test stop
checking the dialog rather than fail**. Pair it with a roster pin (an explicit id
list + expected `ButtonPlan`). Proved both directions by mutation; each names its
own dialog.

Also: re-adding a button on a keyed dialog's FINAL screen keeps the DU-L linter
green (R1 is satisfied), so the agreement test is the only thing that catches it.

## 2b. The cooked PAKs ARE in git, and a seed-only guard cannot catch a typo

`data/cache/*.pak` is tracked (21 archives, commit `cad5b754`) and tests may read it; `base/resources/tests/committed_paks.rs` already did. Ledgers and worker briefs in this campaign said otherwise — check `git ls-files data/cache/` rather than believing a doc.

This is load-bearing, not trivia. A `StripAll` plan asserts "this dialog holds no seed button rows", which **a dialog id that does not exist satisfies perfectly**. At runtime `apply_dialog_patches` warns once and keeps the canonical bytes, so a mistyped id is silent on every layer: seed says fixed, linter agrees, player still sees the buttons. Run every plan against the committed archive (`zip::ZipArchive`, entry `_<dialog_id>`), assert the entry HAD buttons before the strip, and assert speaker/escaped text/root attributes survive unchanged. Proof: point a row at a nonexistent id — both seed-side tests stay green, only the cooked guard fires.

## 3. Where such a test can live

`cimmeria-services` depends on `cimmeria-content-engine`, not the reverse, so a
content-engine test target **cannot** see `CELLBLOCK_DIALOG_PATCHES`. And a new
module under `dialog_overrides/` needs a `mod` line in a `mod.rs` shared with the
other zone packet. Answer: `#[cfg(test)] mod tests` inside the zone table file
itself. Costs a duplicated seed scanner per zone; flag it for later extraction.

A live-DB test is the wrong shape here even though `mission_1326.rs:711` precedents
counting buttons via SQL: it self-skips without `DATABASE_URL`, so plain
`cargo test` goes green while seed and patch disagree.

## 4. Scanning `dialog_screens.sql` / `dialog_screen_buttons.sql` without quote tracking

`dialog_screens.text` holds raw newlines and apostrophes (1,013 of 13,467 rows), so
a line scan silently drops them. But `text` is the LAST or near-last column in both
tables, and every id field sits in front of it — match the full
`INSERT INTO <t> (cols…) VALUES (` header and read leading integers. Pin the match
count against `s.matches("INSERT INTO <t>").count()` or the scan can go vacuous.
All 4,350 button rows are single-tuple single-line inserts.

## 5. The close path `-1` is the discard, and the pin is cleared before the fire

`cell_methods/player/interaction/dialog.rs` rejects a choice unless
`open_dialog_id == dialog_id` and clears the pin BEFORE calling `fire_dialog_choice`.
Consequences worth not re-deriving:

- A replayed `(id, -1)` is dropped: one choice per display, always.
- A chain whose first action displays another dialog does NOT produce a second
  eviction `-1` for the first — client-side the `-1` *is* what frees the slot, so
  by the time the second display arrives there is nothing to evict.
- Two dialogs displayed within one read-gap (e.g. `delay_ms` 10100 then 10600) DO
  collide: the evicted one's `-1` arrives after the re-pin and is rejected with a
  `warn!` on every run. Stripping buttons off the first one is what makes that fire.

## 6. `fire_dialog_choice` populates no `archetype`

`cell/content/event_dispatch/dialog.rs` sets `dialog_id`, `button_id`, world and
mission context only. An `archetype` condition on a `dialog_choice` chain reads a
missing key (evaluates as -1) and fails open. Replay fixtures must reproduce the
omission, not helpfully supply a value.
