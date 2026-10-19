# UB-DP: Beta dialog patches (1997 and 3228 Accept on the final screen; 1916 and 3905 stripped)

> **Type:** How-to (work packet) · **Index:** [beta-work-packets.md](../beta-work-packets.md) · **Wave:** B1a

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Size | about 38k |
| Branch / worktree | `uzp-ubdp` / `uzp-ubdp` (based on `uzp-e1`) |
| Commit subject | `fix(beta-e1): UB-DP Beta dialog patches: offer Accept on the final screen, undrawable More Info stripped` |
| Depends on | Nothing. Runs alongside UB-01a and UB-02a |
| Ship order | Before UB-31 and UB-33 (C6: they display 1916 and 3905) and UB-35 and UB-36 (C5: their `dialog_choice` chains key 1997 and 3228) |

## Why

UB-00a1 (`docs/analysis/union-beta-e1-continuation/mission-seal.md`, Q2 and C5) found that two Beta offer dialogs break the dialog-button rule R1: a dialog that keys a `dialog_choice` chain must have a button on its FINAL screen, or no buttons at all. Otherwise a player who reads to the end presses Done, nothing is sent, and the mission can never be accepted.

| Dialog | Mission (offer) | Screens | Accept today | Final screen |
|---|---|---|---|---|
| 1997 | 477 Covert Observations (dsm 2456, set 500) | 64045-64049 | button row 1737 on 64047 (screen 3 of 5) | 64049 |
| 3228 | 479 Process Support (dsm 2461, set 502) | 64066-64068 | button row 2597 on 64066 (screen 1 of 3) | 64068 |

UB-00a1 review pass 2 (`mission-seal.md` Q2 R3 note and C6) found a second problem. The DialogWin draws button types 2, 4, 5 and 6 only (R3, `rules.rs` `drawable_button_types`), and the linter checks every dialog a chain displays. Two `show`-only DialogWin dialogs carry a type-1 More Info the window cannot draw: 1916 (screens 95484 and 95485, rows 1677 and 1678; displayed by UB-31 chain 23317) and 3905 (final screen 49940, row 3007; displayed by UB-33 chain 23395). Once their chain files are registered, `r3_violations` goes red. Neither is `dialog_choice`-keyed, so stripping all buttons is safe: a zero-button close sends `(id, -1)` to no chain.

The fix is a server-side cooked-dialog patch (the Castle precedent, DU-02b): the server rewrites the cooked entry at PAK load and the client renders the patched entry. No client patch and no new opcode. The architect chose this over C5a (re-keying the chains to another trigger); C5a is rejected.

## Contract excerpt

- `crates/resources/src/base/dialog_overrides/patch.rs`:
  - `pub struct DialogPatch { pub dialog_id: u32, pub ui_screen_type: Option<u32>, pub buttons: ButtonPlan }`
  - `ButtonPlan::OnlyOn { screen_id, button_type, button_id, text: &'static str }` keeps exactly one button, on `screen_id`. `ButtonPlan::StripAll` removes every button of the dialog.
  - `pub const DIALOG_PATCH_TABLES: &[&[DialogPatch]] = &[super::patches_cellblock::CELLBLOCK_DIALOG_PATCHES, super::patches_castle::CASTLE_DIALOG_PATCHES];` (about lines 111-114). The shared guards in `patch_tests/` iterate this list, so a table that is not listed is neither served nor checked.
- `patches_castle.rs` (about 100 lines) is the model for the new table file: header doc comment, `use super::patch::{ButtonPlan, DialogPatch};`, then `pub(super) const CASTLE_DIALOG_PATCHES: &[DialogPatch] = &[ ... ];`.
- `patch_seed_agreement_castle.rs` (about 460 lines) is the model for the new test file. Its four tests: `castle_patches_agree_with_the_committed_button_seed`, `castle_only_on_patches_target_the_final_screen`, `castle_patches_apply_to_the_committed_cooked_entries` (reads `data/cache/CookedDataDialogs.pak`, which is in git), `the_castle_agreement_scan_is_reading_real_rows` (pins the table length).
- `mod.rs` (about lines 55-65) declares `mod patches_castle; mod patches_cellblock;` and the `#[cfg(test)] mod patch_seed_agreement_castle;` / `_cellblock;` lines.
- Hard rule from the client contract: never add a button to 2300, 5021, 5020, 2574, 2575, 2577, 2581, 5003, 5004 or 5009. 1997 and 3228 are not on that list.

## Rules (binding)

1. PowerShell only; start every command with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\uzp-ubdp;`. Absolute paths. No bash.
2. Never run `cargo` directly; use `pwsh tools/build-lane/lane.ps1 cargo <cmd> -p <crate>`. Run in the foreground; check every test count is above 0 (a filter matching nothing passes).
3. Edit only the allowlist.
4. `dialog_screen_buttons.sql` is CRLF throughout (8646 CRLF, no bare LF). Edit the two rows in place and keep it CRLF; after any rewrite normalise with `[IO.File]::WriteAllText('<abs>', ([IO.File]::ReadAllText('<abs>') -replace "\r?\n", "\r\n"))`. Docs under `docs/` are CRLF too.
5. Seeds in `db/resources/` only; no migrations; do not touch `dialog_screens.sql`, `dialogs.sql` or `dialog_set_maps.sql`.
6. Files under 500 lines (hard cap 700). The copied test file lands at about 460; do not add to it beyond the renames.
7. Never touch the `sgw` database, the lab or the client.
8. No `git stash`, `git worktree prune` or push. One local commit.

## Allowlist

| File | Change |
|---|---|
| `crates/resources/src/base/dialog_overrides/patches_beta_e1.rs` | new: `BETA_E1_DIALOG_PATCHES` |
| `crates/resources/src/base/dialog_overrides/patch_seed_agreement_beta_e1.rs` | new: copy of the Castle agreement tests |
| `crates/resources/src/base/dialog_overrides/mod.rs` | `mod patches_beta_e1;` and `#[cfg(test)] mod patch_seed_agreement_beta_e1;`, next to the Castle lines |
| `crates/resources/src/base/dialog_overrides/patch.rs` | add `super::patches_beta_e1::BETA_E1_DIALOG_PATCHES` to `DIALOG_PATCH_TABLES` |
| `db/resources/Dialogs/Seed/dialog_screen_buttons.sql` | rows 1737 and 2597: `screen_id` only; delete rows 1677, 1678 and 3007 |
| `docs/architecture/mission-pak-overrides.md` | line 153 table row and line 298 step: name `patches_beta_e1.rs` / `BETA_E1_DIALOG_PATCHES` |

## Steps

1. **Table.** Create `patches_beta_e1.rs` from `patches_castle.rs`'s shape. Header: "Beta Site E1 dialog patches (UB-DP)", the same three-step "adding a row" note, and a line citing `mission-seal.md` Q2/C5. Rows:

   ```rust
   pub(super) const BETA_E1_DIALOG_PATCHES: &[DialogPatch] = &[
       // 477 offer (dsm 2456): Accept moves from screen 64047 to the final screen 64049.
       DialogPatch { dialog_id: 1997, ui_screen_type: None,
           buttons: ButtonPlan::OnlyOn { screen_id: 64049, button_type: 2, button_id: 8, text: "Accept" } },
       // 479 offer (dsm 2461): Accept moves from screen 64066 to the final screen 64068.
       DialogPatch { dialog_id: 3228, ui_screen_type: None,
           buttons: ButtonPlan::OnlyOn { screen_id: 64068, button_type: 2, button_id: 8, text: "Accept" } },
       // 420 (UB-31 chain 23317 shows it): DialogWin cannot draw its type-1 More Info (R3).
       DialogPatch { dialog_id: 1916, ui_screen_type: None, buttons: ButtonPlan::StripAll },
       // 1147 (UB-33 chain 23395 shows it): same R3 fix.
       DialogPatch { dialog_id: 3905, ui_screen_type: None, buttons: ButtonPlan::StripAll },
   ];
   ```

2. **Register.** Add the two `mod` lines in `mod.rs`, and the table to `DIALOG_PATCH_TABLES` in `patch.rs` (third entry).
3. **Seed.** In `dialog_screen_buttons.sql`, change row 1737's `screen_id` from 64047 to 64049 (about line 3500) and row 2597's from 64066 to 64068 (about line 5178). Change nothing else on either line. Then delete the three whole lines for rows 1677 (screen 95484), 1678 (95485) and 3007 (49940), all `9, <screen>, 1, 'More Info'` (about lines 3380, 3382, 5998). Put no comment inside the file (the file has none).
4. **Tests.** `Copy-Item` `patch_seed_agreement_castle.rs` to `patch_seed_agreement_beta_e1.rs`, then rename: `castle` → `beta_e1` in the four test names, `CASTLE_DIALOG_PATCHES` → `BETA_E1_DIALOG_PATCHES`, `patches_castle` → `patches_beta_e1`, the module doc's "Castle" → "Beta Site E1 (UB-DP)", and the length pin from 3 to 4 with the message "UB-DP ships four Beta patches (1997, 3228, 1916, 3905)". Keep the helpers private and unchanged.
5. **Docs.** In `docs/architecture/mission-pak-overrides.md`, add `patches_beta_e1.rs` and `BETA_E1_DIALOG_PATCHES` to the line-153 row and "`patches_beta_e1.rs` for Beta_Site_Evo_1" to the line-298 step. Keep CRLF.
6. **Revert proof.** (a) Restore row 1737 to 64047 and run the `beta_e1` agreement filter: `beta_e1_patches_agree_with_the_committed_button_seed` must fail. (b) Set the 1997 patch's `screen_id` to 64047: `beta_e1_only_on_patches_target_the_final_screen` must fail. (c) Remove the table from `DIALOG_PATCH_TABLES`: name the `patch_tests/` guard that fails, or say none does and why. (d) Restore row 1677 and run the content-engine `dialog_button_linter`: name the result (no Beta chain file is registered yet, so it may stay green; say so). Restore everything and rerun green. Quote all of it.

If `beta_e1_patches_apply_to_the_committed_cooked_entries` fails with `PatchError::ScreenMissing` on the unchanged rows, the cook does not ship that screen: stop and return `blocked` with the dialog and screen id. Do not pick another screen.

## Lane checks (in order)

```powershell
Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\uzp-ubdp; pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-resources
Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\uzp-ubdp; pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-resources --all-targets -- -D warnings
Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\uzp-ubdp; pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-resources dialog_overrides
Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\uzp-ubdp; pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-content-engine --test it dialog_button_linter
```

The first nextest runs the new file plus the shared `patch_tests/` guards and the Castle and Cellblock agreement tests. The linter run proves the seed still passes the DU-L rules with the moved buttons.

## Acceptance

- Four new tests green (the StripAll rows are covered by the copied agreement test, which asserts no seed rows for a StripAll dialog); every other `dialog_overrides` test and `dialog_button_linter` green; counts above 0.
- SigNoz: none until UB-35/UB-36 bind these dialogs. Lab row UB-U10 (UB-90) checks live that the 477 and 479 offers show Accept only on their last screen and that pressing it accepts the mission. 1916 and 3905 show no buttons and close with Done.

## Docs rows

`docs/architecture/mission-pak-overrides.md` (per-zone dialog patch tables), step 5. List it in the report.

## Return

`UB-DP <done|blocked|failed> <sha|-> C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\uzp-ubdp\.claude\reports\union-beta-e1\UB-DP-coder.md`
