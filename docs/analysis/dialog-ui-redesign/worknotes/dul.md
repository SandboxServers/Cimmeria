# DU-L Worknotes

> Type: reference. Audience: dialog-UI-redesign coordinator.
> Companions: [work-packets.md](../work-packets.md), [TESTING.md](../../../../TESTING.md),
> [content-chains.instructions.md](../../../../.github/instructions/content-chains.instructions.md).

## Contract

- **Packet:** DU-L — dialog button linter.
- **Scope:** one no-DB, seed-parsing integration test in `crates/content-engine/tests/`,
  enforcing the two hard rules of the Client Contract plus the Blurb button-type rule, for
  every dialog id that keys a `dialog_choice` chain in the four Castle / Castle_CellBlock
  chain seeds.
- **Decisions in force:** Client Contract F1, F6, F7, F8, F9 and the two "Consequences that
  overrule the brief" hard rules.
- **Depends on:** nothing. Wave 0.
- **Source revision/base:** branch `dialog-ui/dul-button-linter` in the `dul` worktree, based
  on `origin/main` @ `192d4216`.
- **Owned paths:**
  - `crates/content-engine/tests/dialog_button_linter.rs`
  - `crates/content-engine/tests/dialog_button_linter/sql_scan.rs`
  - `crates/content-engine/tests/dialog_button_linter/seed_model.rs`
  - `crates/content-engine/tests/dialog_button_linter/rules.rs`
  - `crates/content-engine/tests/dialog_button_linter/rule_guards.rs`
  - `TESTING.md` (type 6 section + picker row)
  - `.github/instructions/content-chains.instructions.md` (new review section)
  - `docs/analysis/dialog-ui-redesign/worknotes/dul.md` (this file)
- **Read set:** `docs/analysis/dialog-ui-redesign/work-packets.md` (Client Contract, Native
  Findings, Target Matrix, DU-L / DU-02a / DU-02b sections);
  `crates/content-engine/tests/interact_tag_linter.rs` (precedent);
  `crates/content-engine/src/loader/trigger.rs` and `src/triggers/matching.rs` (how a
  `dialog_choice` key is parsed and matched); `db/resources/Dialogs/Tables/{dialogs,
  dialog_screens, dialog_screen_buttons}.sql` (column semantics);
  `db/resources/Dialogs/Seed/` and `db/resources/Content/Seed/castle_*_chains.sql`
  (read-only); `data/cache/CookedDataDialogs.pak` (read-only, not in git, evidence only);
  `TESTING.md`, `CLAUDE.md`, `.github/instructions/content-chains.instructions.md`.

## Evidence

### Screen ordering: `dialog_screens.index`, not `screen_id`

`db/resources/Dialogs/Tables/dialog_screens.sql` declares
`(dialog_id, screen_id, text, speaker_id, index)`. `index` is the ordering column:

- Across all 5,412 shipped dialogs, `index` is exactly `0..n-1` — no duplicates, no gaps.
  Verified by scanning every row of `dialog_screens.sql`.
- The `index` order matches the cooked entry's `<Screens ScreenID>` order for 2576, 3999 and
  5861, read from `data/cache/CookedDataDialogs.pak` (a zip; entries are named `_<dialogId>`).
- Sorting by `screen_id` gives the same order for **every** dialog in today's seed (checked
  all 5,412). That makes the choice invisible on current data, which is why it is pinned by a
  test: `rule_guards::final_screen_follows_the_index_column_not_the_screen_id` builds a
  dialog whose last screen has the lowest id and asserts the soft-lock is still reported.

### The seed mirrors the cooked data

The linter reads the seed; the client reads the cooked pak (F1). Cross-checked on
2026-09-21 for all three violators — the cooked `<Buttons ButtonType ButtonID Text>` rows
match the seed's `dialog_screen_buttons` rows exactly, and the screen order matches:

| Dialog | Cooked screens | Cooked buttons |
|---:|---|---|
| 2576 | 96821-96825 | 3 x `ButtonType="4" ButtonID="71" Text="Take Missions"` |
| 3999 | 96252-96260 | 7 x `ButtonType="4" ButtonID="70" Text="Receive Item"` |
| 5861 | 96782-96789 | 5 x `ButtonType="2" ButtonID="8" Text="Accept"` |

The pak is not in git, so no test depends on it.

### R1 violators: exactly the three the ledger names

A full scan of the four chain files found **nineteen** `dialog_choice`-keyed dialogs.
Zero-button (compliant, they advance on F8's `-1`): 2300, 2574, 2575, 2577, 2581, 5003,
5004, 5008, 5009, 5020, 5021. Button on the final screen (compliant): 2299 (96179), 2573
(113558), 4001 (96251), 5022 (96270), 5023 (96292). **Violating R1: 3999, 5861, 2576 and
nothing else.** No fourth violator was found, so the allowlist carries exactly the three the
packet specified and no `FOUND BY DU-L` entries were needed.

### R2 and R3 are green on the current seed

All eleven never-add-a-button dialogs have zero button rows. The four chain files reference
five Blurbs (2298, 2305, 2518, 4000, 5859) carrying six button rows between them, all of
type 1 or 2.

## Design decisions

1. **A quote- and comment-aware SQL scanner, not the precedent's line scan.**
   `interact_tag_linter.rs` reads its seeds a line at a time. That is wrong here twice over,
   and both failures are silent:
   - 1,013 of the 13,467 `dialog_screens` rows contain a raw newline inside `text`. A
     line-at-a-time scan drops every one, including screens that decide which screen is last.
   - The chain seeds put apostrophes inside `--` comments (`Frost's`, `work-packets.md's`,
     `Ba'al`). A scanner that tracks quotes but not comments reads the first as an opening
     literal and swallows every statement after it.

   `sql_scan.rs` therefore does one pass understanding single-quoted literals with the `''`
   escape and `--` line comments. There are no dollar-quoted strings and no `/* */` comments
   in these files; a literal `$$` does appear inside one `dialog_screens` value, so `$` is
   ordinary text. Each of these hazards has its own guard test.

2. **Rows addressed by column name, not position.** Every seed statement writes its column
   list out in full. A name-keyed read means a column inserted into the middle of a table
   produces a missing key rather than a silently wrong value.

3. **Dialog ids read the way the loader reads them.** `dialog_choice`'s id arrives as a
   quoted `event_key` and is `parse()`d, mirroring `loader/trigger.rs`. `display_dialog`'s
   arrives as the unquoted numeric `target_id`.

4. **The allowlist expires itself.** `r1_violations` fails on a *stale* entry — one whose
   dialog now satisfies R1, or that nothing keys any more — as well as on an unlisted
   violator. DU-02a and DU-02b cannot fix a dialog and leave the exemption behind.
   `allowlisted_dialogs_still_ship_the_soft_locking_layout` additionally pins each violator's
   screen count, screens-carrying-buttons count and final screen id, so a fix trips two tests
   and both point at the allowlist.

5. **R2 is a separate rule, not a special case of R1.** A button on the *final* screen of
   5003 satisfies R1 and still silences the dialog. `rule_guards::r2_flags_a_single_button_
   added_to_a_protected_dialog` asserts exactly that: R1 empty, R2 non-empty, same data.

6. **Predicates take their policy as a parameter.** `r1_violations(seed, refs, allowlist)`
   rather than reading the constant, so the synthetic guards drive the *production* function
   with an empty list. This is the lesson `interact_tag_linter.rs` records on
   `region_key_violations`: a guard that re-implements the rule keeps passing when the rule
   is loosened.

7. **Disabled chains are not filtered.** Every `content_chains` row in the four files is
   `enabled = true` today. A chain disabled later is still linted — the cost is a false
   positive on a dead chain, not a missed soft-lock on a live one.

8. **File layout.** Cargo fixes the entry point at `tests/dialog_button_linter.rs` and
   resolves a bare `mod` from a test root against `tests/` itself, where every `.rs` becomes
   its own test target. The submodules therefore live in `tests/dialog_button_linter/` and
   are pulled in with `#[path]`; the house `foo/mod.rs` style cannot apply. Largest file is
   318 lines, well under the 500-line soft cap.

## Tests

17 tests in one target, `cimmeria-content-engine --test dialog_button_linter`.

| Test | What it pins |
|---|---|
| `chain_keyed_dialogs_have_a_button_on_their_final_screen_or_none_at_all` | R1 on the live seed |
| `never_add_a_button_dialogs_still_have_zero_buttons` | R2 on the live seed |
| `blurbs_referenced_by_castle_chains_use_only_more_info_and_accept` | R3 on the live seed |
| `every_insert_row_in_the_dialog_seeds_is_parsed` | parsed row count == raw `INSERT INTO <table>` count, per file |
| `allowlisted_dialogs_still_ship_the_soft_locking_layout` | the exact broken layout of 3999, 5861, 2576 |
| `the_chain_scan_finds_the_dialogs_it_is_supposed_to_lint` | keyed set includes 2300 and 5003; 2300 keyed twice, 2576 three times; 5862 found from a multi-row VALUES list; R3 has >= 3 Blurbs carrying >= 5 buttons |
| `sql_scan::scanner_guards::*` (5) | newline + `''` inside a literal; apostrophe in a `--` comment; `--` inside a literal; every tuple of a multi-row VALUES list; other tables ignored |
| `rule_guards::*` (6) | R1 fires on mid-screen only; stale allowlist (fixed, and unkeyed); allowlist suppresses exactly its entry; final screen follows `index`; R2 catches what R1 cannot; R3 is Blurb-only and referenced-only |

### Commands run

All through the lane wrapper, from `/c/Users/Steve/source/projects/Cimmeria/.claude/worktrees/dul`,
with `L=/c/Users/Steve/AppData/Local/Temp/cimmeria-castle`.

```text
$L/lane.sh cargo test -p cimmeria-content-engine --test dialog_button_linter
  -> exit 0; 17 passed, 0 failed, 0 ignored, 0 filtered out. No skips (no DB involved).
$L/lane.sh cargo fmt --all
  -> exit 0 (reformatted the new files only; no other path touched)
$L/lane.sh cargo +1.98.1 clippy -p cimmeria-content-engine --all-targets -- -D warnings
  -> exit 0, no warnings
```

`git status` after the run shows no modification under `db/` — both mutation proofs were
reverted.

## Regression proof

Neither proof used `git stash`; each was an in-place edit reverted by an inverse edit, with
`git status --porcelain -- db/` confirming the seed came back clean.

### (a) A button added to dialog 5003 -> R2 fails

Appended to `db/resources/Dialogs/Seed/dialog_screen_buttons.sql` (screen 96986 is 5003's
index 2):

```sql
INSERT INTO dialog_screen_buttons (screen_button_id, button_id, screen_id, button_type, text) VALUES (999999, 8, 96986, 2, 'Accept');
```

`$L/lane.sh cargo test -p cimmeria-content-engine --test dialog_button_linter` -> exit 101,
15 passed / 2 failed:

```text
dialog button linter (R2) found 1 problem(s):
  R2 dialog 5003: on the never-add-a-button list but now carries buttons —
  [(96986, [Button { button_id: 8, button_type: 2, text: "Accept" }])]. It is keyed by
  castle_706_708_chains.sql:chain 1343 and advances ONLY through the zero-button close,
  which sends dialogButtonChoice(5003, -1) (F8). ...
```

R1 fired too, because the button landed mid-dialog — also correct, and it names the final
screen 96992. Reverted by truncating the appended row; `db/` clean.

### (b) The 2576 allowlist entry removed -> R1 fails naming 2576

Deleted 2576's `R1Exemption` from `rules.rs` (and dropped the array length to 2), simulating
DU-02b removing the entry without fixing the dialog. Same command -> exit 101, 16 passed /
1 failed:

```text
dialog button linter (R1) found 1 problem(s):
  R1 dialog 2576: keyed by castle_701_chains.sql:chain 1237, castle_701_chains.sql:chain 1238,
  castle_701_chains.sql:chain 1239. Buttons on screen(s) [96821, 96822, 96823], but final
  screen Some(96825) has none. ...
```

Entry restored; the full 17 pass again.

The converse — a stale entry when the dialog *is* fixed — is proved by
`rule_guards::r1_reports_an_allowlist_entry_that_no_longer_violates`, which drives the
production `r1_violations` with a fixed dialog and a live exemption and asserts one STALE
message naming the dialog and the owning packet.

## Known gaps

1. **The linter lints the seed; the client reads the override.** F1 means a seed-only fix
   changes nothing in-game. DU-02a/b keep the two in sync by hand, and the DU-L packet note
   already assigns them a per-zone `DialogPatch`-versus-seed agreement test. This linter does
   not and cannot check the patch side — `dialog_overrides.rs` is owned by DU-01.
2. **R3's subject set is five Blurbs, all currently clean.** The rule is real but has no live
   violator, so it is guarded by synthetic data
   (`rule_guards::r3_flags_non_blurb_button_types_and_only_on_referenced_blurbs`) rather than
   by the seed. It becomes load-bearing if DU-02a applies D-DU1 to 2305 / 4000 / 2308 / 2518.
3. **Scope is the four Castle / Cellblock chain files.** A `dialog_choice` chain authored in
   any other seed file is not linted. Widening `CHAIN_FILES` to a `read_dir` of the seed
   directory, the way `interact_tag_linter.rs` does, is a one-line change once the rest of
   the content tree has been audited — several `space_*_chains.sql` files key dialogs that
   were never reviewed against F8.
4. **No `button_id` dimension.** F9 means the linter cannot distinguish Accept from More Info
   either; if DU-06 lands a `button_id` condition, R1 gets a fourth shape to check (a keyed
   dialog whose chain conditions on a `button_id` that no screen actually carries).
5. **Test-inventory docs not updated.** 17 added tests is well under the 5%-of-workspace
   (~147) threshold CLAUDE.md sets for a per-PR inventory edit; it rolls up in the next sweep.

## Integration edits the coordinator must make

1. **Ledger row.** Mark DU-L done in `docs/analysis/dialog-ui-redesign/work-packets.md` and
   record that the allowlist shipped with exactly the three specified dialogs and no
   additional violators.
2. **DU-02a acceptance.** Its "DU-L allowlist entry for 3999 removed" step now has a second
   half: also update 3999's row in `allowlisted_dialogs_still_ship_the_soft_locking_layout`
   (delete the row when the entry goes). The linter fails until both are done.
3. **DU-02b acceptance.** Same for 5861 and 2576.
4. **`docs/readme.md` / index files.** Nothing added there by this packet; the two docs it
   touched are already indexed.

## Judgment call flagged for review

The packet said to mention the linter in
`.github/instructions/content-chains.instructions.md` *if that file lists the interact-tag
linter*. It does not list it by filename — it states the interaction-type rule the linter
enforces, with no pointer to the test. Since the file's `applyTo` glob is
`db/resources/Content/Seed/**/*.sql`, which is exactly where a `dialog_choice` trigger gets
authored, a new **Dialog buttons on a `dialog_choice`-keyed dialog** section was added there
with the three rules and a link to both linters. Revert it if the coordinator reads the
condition strictly.
