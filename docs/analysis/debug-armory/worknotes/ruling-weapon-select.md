# Ruling: active weapon selection, row isolation, canary notes (DR-07h)

> **Type:** worknote (architect ruling). **Audience:** the orchestrator, the DR-07h packet-coder and its reviewer. **Date:** 2026-11-02. **Inputs:** [results/DR-L2-canary.md](../results/DR-L2-canary.md), [canary.md](canary.md), [lab-run-plan.md](../lab-run-plan.md). **Base:** `lab/dr-lab` at `0f40c496f` (DR-07g).

## Ruling 1: a spec gap, not a server defect. File no issue.

An equip into a free, non-active bandolier slot leaves the active slot where it was. That is the shipped behaviour, not a defect:

- **Server:** `use_instance.rs` `resolve_auto_equip_target` (lines 216-296, 399-461) sends a right-click equip Main→Bandolier to the first empty slot. Only when all 4 slots are full does it replace the active slot (`sgw_player.bandolier_slot`). It never moves the active slot.
- **Legacy:** this matches `deprecated/python/cell/Inventory.py` `moveItem` / `updateActiveSlot` (lines 345, 435-510).
- **Client:** the active slot changes only through `requestActiveSlotChange(3, N)`, sent by `BandolierMod.ActivateBandolierSlotN` (keys F1-F4). Its Lua no-ops when slot N is already active ([client-wire-emit-suppression.md](../../../reverse-engineering/findings/client-wire-emit-suppression.md)).
- **Feedback:** a player who equips a new weapon into slot 2 keeps firing slot 1 until they press F2. That is normal weapon-set play. The refusal already gives first-press feedback: `weapon_requirement.rs` sends `onErrorCode(…63)` and the CHAN_FEEDBACK line "You need a different weapon to use that ability."

"Activate a weapon when it is equipped" would be a gameplay design change with no client evidence behind it. It is not recommended and not filed. No owner decision is needed unless the owner asks for one.

**Mechanism: every weapon row, in both generators, does reset slot 1, equip, select slot 1, then asserts.** It uses only existing lab tools:

1. `@item_action { action = "unequip", container = "Bandolier", slot = 1 }`, optional. It returns whatever is in slot 1 (the previous row's weapon or a starter) to Main.
   - On an empty slot it fails harmlessly ("Bandolier slot 1 is empty") before any reveal.
   - When it succeeds, it opens `CharacterWin` (Bandolier slots reveal there, `slots.rs:106-110`).
2. Close the sheet: `@window_click { target = "CharacterWin__auto_closebutton__" }`, optional.
   - `CharacterWin` is a `ComplexFrame_2`, the same type as `InventoryWin` (`Character.layout:4`, `Inventory.layout:4`), so its close button follows the same verified naming.
3. The existing give and equip. Slot 1 is now the first empty slot, so the weapon lands there. Then close the bag (optional in ability rows).
4. `@key { key = "F1" }`. This is `client_input_key`, tier N1, `tools.rs:60`. It makes slot 1 active, and is a no-op if slot 1 already is.
5. A required clause `armed`: `@player_state` `/ammo/weapon_item_id` `eq <item id>`. This is the DA-U39 pattern (`debug-area.toml:1499-1506`), and `player_state.rs:86-98` reads `getItemIDForSlot` on the active slot. A wrong active weapon then grades FAIL with a named reason, instead of an opaque `wrong_weapon_type` or `weapon_unbound`. **Superseded by DR-07i:** the shipped clause reads `/ammo/weapon` and compares the weapon name; the `/ammo/weapon_item_id` pointers and item-id values below are the original ruling.

**Alternatives rejected:**

| Alternative | Why not |
|---|---|
| `/activatebandolierslot` | Its arguments are ambiguous: commands.md says `<bagId> <slotId>`, RE says SlotNum only. |
| A per-row F-key computed from row order | Breaks on resume, row subsets and failed equips. |
| Unequip by name | Main can hold copies with the same name. |
| Changing `client_item_action` to send `requestActiveSlotChange` | A lab-crate change. The spec fix needs none. |

**Main-bag headroom:** a reset returns one weapon to Main per row, which only refills the space the grant left. Grants stay ≤ 30 per gear file and ≤ 18 weapon grants per abilities file, against 40 slots.

## Ruling 2: yes, isolate every weapon row, in gear files and in ability files

The same setup reset (step 1) is the isolation. It sits in **setup**, not teardown, because a setup reset also covers:

- the first row (a starter in slot 1);
- rows whose teardown never ran (BLOCKED or failed setup);
- resumed runs.

There is no need to reorder unbound rows. Armor rows touch no weapon and stay unchanged.

## Ruling 3: the other canary notes

- **GW-3128 BLOCKED** (the `lab_login` focus bridge blip): rerun it unchanged in the next canary. If it blocks again at the same step, the verifier writes a lab issue body under `results/issues/` (DR-30).
- **Missing bookmark anchors** (Soldier-00, GW-17 `bookmark_id: null`, GW-3128): flag only. No fix in DR-07h.
  - Soldier-00's anchor goes out before the 17 s first-login hold, so its reply is lost. Moving the anchor after the first setup wait is a cosmetic runner change; it is noted for DR-99.
  - The others are intermittent. The full DR-L2 verifier records the missing-anchor count per file. If more than 20% of rows miss an anchor, it writes one lab issue body (DR-30).
- **Verifier lease (R5 amended).** Any guarded call needs the current lease, `lab_uat_attest` included (`lease/policy.rs:169`, `lease/mod.rs:396-412`). `lab_uat_run` takes and releases its own (R4), so a verifier with no lease is always refused.
  - The orchestrator does **not** pass its own lease. A lease it held would block the driver's next `lab_uat_run`.
  - Instead, the verifier takes a short lease of its own while the lab is idle, as worded in DR-07h step 6.
  - The orchestrator does not dispatch the next driver call while a verifier attest is in flight. It runs verifier attest steps in the gaps: after a driver returns and before the next `lab_uat_run`, or at the session end.

## DR-07h: select the granted weapon and isolate weapon rows

- **Agent:** packet-coder. **Host:** windows. **Size:** about 35k working tokens.
- **Worktree:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab`, branch `lab/dr-lab` (as DR-07d to DR-07g).
- **Commit:** `fix(uat): reset bandolier slot 1, select it with F1 and assert the active weapon in Armory weapon rows (DR-07h)`.
- **Precondition:** no lab session runs from this worktree. run.json pins spec SHA-256s, so specs are never edited mid-session. The orchestrator confirms this before it dispatches.

### Rules this packet carries

1. Edit only the files listed under Files. No `git stash`, no push, no `git worktree prune`. Commit locally with explicit paths, then stop.
2. Never call cargo directly. Use the lane, in the foreground, from this worktree: `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab; pwsh tools/build-lane/lane.ps1 cargo ...`.
3. The spec TOMLs under `docs/guides/uat-specs/` are generated, so never hand-edit them; rerun the generator. They stay **LF**.
4. `.md` files under `docs/` are **CRLF**. After editing one, run: `Get-ChildItem <absolute path> | ForEach-Object { $t=[IO.File]::ReadAllText($_.FullName); $t=$t -replace "`r?`n","`r`n"; [IO.File]::WriteAllText($_.FullName,$t) }`.
5. Python keeps LF, like its neighbours.
6. Files have a soft cap of 500 lines and a hard cap of 700. Both generators stay under 330.
7. Tests are Python `unittest` (the generator tests) plus the Rust spec guard below. A new test must fail when the change is reverted: prove it once (step 7).
8. Use PowerShell only, with absolute paths for `[IO.File]`. At most 5 fix rounds on a failing check, then report `blocked` with the last error.

### Files

| File | Change |
|---|---|
| `tools/ability_mechanics/debug_armory_specs.py` | Constants, the gated branch of `ability_row`, the `armed` clause |
| `tools/ability_mechanics/debug_armory_gear.py` | Import the constants, `_tool(optional=)`, `weapon_row` |
| `tools/ability_mechanics/test_debug_armory_specs.py` | Update the index test, add new tests |
| `tools/ability_mechanics/test_debug_armory_gear.py` | Add new tests |
| `docs/guides/uat-specs/armory-abilities-*.toml`, `docs/guides/uat-specs/armory-gear-*.toml` | Regenerated only |
| `docs/analysis/debug-armory/lab-run-plan.md` | R5, line 55 and the verifier rules line (CRLF) |

### Steps

**1. `debug_armory_specs.py`**

(a) Below `FIRST_LOGIN_HOLD`, add:

```python
# Equip lands in the first empty bandolier slot and never moves the active slot
# (use_instance.rs resolve_auto_equip_target), so a weapon row empties slot 1,
# equips into it and selects it with F1 (BandolierMod.ActivateBandolierSlot1) (DR-07h).
BAG_CLOSE = "InventoryWin__auto_closebutton__"
SHEET_CLOSE = "CharacterWin__auto_closebutton__"
SELECT_KEY = "F1"
```

(b) In `ability_row`, replace the two `setup.append` lines of the `else:` branch (`rep is not None`) with this order:

```python
setup.append(call("@item_action", {"action": "unequip", "container": "Bandolier", "slot": 1}, label="reset-slot1", optional=True))
setup.append(call("@window_click", {"target": SHEET_CLOSE}, label="close-sheet", optional=True))
setup.append(chat(f"/gmgiveitem {rep} 1"))
setup.append(call("@item_action", {"action": "equip", "name": items[rep]["name"]}, label="equip"))
setup.append(call("@window_click", {"target": BAG_CLOSE}, label="close-bag", optional=True))
setup.append(call("@key", {"key": SELECT_KEY}, label="select-slot1"))
setup.append("{ wait_ms = 1500 }")
```

Leave the `place`, wait and `cooldowns_reset` lines after it unchanged. Ungated and blocked rows get none of the seven lines.

(c) After the `sent`/`applied` clause lines (after the `required = false` append), add the following only when `ability["item_monikers"] and rep is not None`. It must be the **last** clause, so `expect[0]` and `expect[1]` keep their meaning:

```python
lines += [
    "[[row.expect]]",
    'id = "armed"',
    f"text = {tstr(f'the active bandolier weapon is item {rep} before the press')}",
    'source = "tool"',
    'tool = "@player_state"',
    'pointer = "/ammo/weapon_item_id"',
    'op = "eq"',
    f"value = {rep}",
]
```

**2. `debug_armory_gear.py`**

(a) Change the import to `from debug_armory_specs import BAG_CLOSE, SELECT_KEY, SHEET_CLOSE, chat, tstr`. Delete the local `BAG_CLOSE = ...` line, but keep its comment above the import's use. `gear.BAG_CLOSE` must still resolve, because the tests use it.

(b) `_tool` gains `optional: bool = False`. When it is true, append `("optional", "true")` after the label.

(c) In `weapon_row`, the setup becomes, in order:

1. `*throttle`
2. `_tool('@item_action', [('action', tstr('unequip')), ('container', tstr('Bandolier')), ('slot', '1')], 'reset-slot1', optional=True)`
3. `_tool('@window_click', [('target', tstr(SHEET_CLOSE))], 'close-sheet', optional=True)`
4. the give
5. the equip
6. the existing close-bag (still required)
7. `_tool('@key', [('key', tstr(SELECT_KEY))], 'select-slot1')`
8. the existing `@dummy`
9. `{ wait_ms = 2000 }`

There is no extra wait: the dummy spawn and its 2000 ms cover the slot change.

(d) Append to **both** branches' `clause` list (bound and unbound) a second clause: the same eight lines as step 1(c), with `rep` replaced by `item_id`. An unbound row also needs its own weapon active, or `weapon_unbound` names the wrong item.

(e) Update the module docstring's last sentences to say weapon rows reset slot 1, select it with F1 and assert `armed`.

**3. `test_debug_armory_specs.py`**

(a) Rewrite `test_the_row_gives_and_equips_its_weapon_before_it_places_the_ability` for the new indices:

| Index | Expected action |
|---|---|
| `setup[0]` | `@item_action`, args `{"action": "unequip", "container": "Bandolier", "slot": 1}`, `optional` true |
| `setup[1]` | `@window_click` `SHEET_CLOSE`, optional |
| `setup[2]` | the give |
| `setup[3]` | equip `Pistol` |
| `setup[4]` | close-bag, optional |
| `setup[5]` | `{"tool": "@key", "args": {"key": "F1"}, "label": "select-slot1"}` |
| `setup[6]` | `{"wait_ms": 1500}` |
| `setup[7]` | `@use_ability` place |

(b) Add `test_a_gated_row_asserts_the_active_weapon`: `expect[-1]` has id `armed`, tool `@player_state`, pointer `/ammo/weapon_item_id`, op `eq` and value `42`. It has no `required` key.

(c) Add `test_an_ungated_row_neither_resets_nor_selects`: no setup action has tool `@key`, none has action `unequip`, and no clause has id `armed`. Check this for an ungated row and for a blocked row.

**4. `test_debug_armory_gear.py`**

Add three tests:

- `test_weapon_row_resets_slot_one_then_equips_then_selects_it`: on the parsed TOML of `gear.render("s", "weapons", [gear.weapon_row(3753, "Cobra Staff", 584)])` (or parse the row text directly if `render` needs more), the setup labels/actions are in this order: `reset-slot1` < give (`chat`) < equip (`action = "equip"`) < `close-bag` < `select-slot1` < `@dummy`. `reset-slot1` and `close-sheet` are optional; `select-slot1` is not.
- `test_both_weapon_kinds_assert_the_active_weapon`: for `weapon_row(3753, "Cobra Staff", 584)` and `weapon_row(3754, "Plain Club", None)`, the `armed` clause has `pointer = "/ammo/weapon_item_id"` and value 3753 or 3754.
- `test_armor_rows_do_not_touch_the_bandolier`: an `armor_row(...)` text contains neither `"unequip"` nor `"@key"`.

The existing tests must still pass unchanged.

**5. Regen** (from the worktree, in the foreground):

- `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab; python tools/ability_mechanics/debug_armory.py specs abilities --out-dir docs/guides/uat-specs`
- `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab; python tools/ability_mechanics/debug_armory.py specs gear --out-dir docs/guides/uat-specs`
- Then `git status --short docs/guides/uat-specs`: only `armory-*` files may change. Spot-check `armory-abilities-soldier-0.toml` row `Soldier-717` and `armory-gear-weapons-1.toml` row `GW-5680` for the reset, F1 and `armed` lines.

**6. `lab-run-plan.md`** (CRLF afterwards, rule 4)

(a) Replace the R5 bullet with:

> - **R5. The verifier may call `lab_uat_report`, and for attests only `lab_lease_status`, `lab_lease_acquire`, `lab_uat_attest` and `lab_lease_release`**; never a driving tool. `lab_uat_attest` is lease-guarded and `lab_uat_run` releases its own lease, so the verifier works like this:
>   1. It grades from the bundle and writes its results while the next file drives.
>   2. It lists every pending attest (row, clause, value, query) in `results/<packet>.md` § Attests.
>   3. Only when `lab_lease_status` shows no holder, it calls `lab_lease_acquire {owner: "dr-verify-<packet>", purpose: "attest"}`, runs every attest with that `lease_id`, and calls `lab_lease_release` at once.
>   4. If the lease is held, it does not wait. It returns `attests-pending`, and the orchestrator re-dispatches only the attest step in the next gap: after a driver returns and before the next `lab_uat_run`, or at the session end.
>
>   The orchestrator never passes its own lease, and never dispatches a driver while a verifier attest is in flight. (Ruling: [worknotes/ruling-weapon-select.md](worknotes/ruling-weapon-select.md).)

(b) Line 55: replace "Verifiers run while the next file drives (they hold no lease)." with "Verifiers grade while the next file drives; their attests wait for an idle-lab gap (R5)."

(c) The verifier rules line (line 136): replace "call only `lab_uat_attest` and `lab_uat_report` of the lab tools (R5)" with "call only the R5 lab tools (`lab_uat_report`; for attests `lab_lease_status`, `lab_lease_acquire`, `lab_uat_attest`, `lab_lease_release`)".

**7. Checks** (foreground; record both revert runs in the commit body):

- `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab; python -m unittest discover -s tools/ability_mechanics -p "test_debug_armory*.py"`: all pass, including `test_committed_files_match_the_generator` in both files.
- **Revert proof:** comment out the `select-slot1` line in `weapon_row` and run the gear tests. The new order test fails. Restore it and rerun: it passes. Do the same for the `@key` line in `ability_row` and the specs tests.
- `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab; pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-lab committed_specs_parse_and_validate every_committed_spec_tag_is_seeded`: passes. This proves `@key`, the optional `@item_action` and the `armed` tool clause validate.

**8. Commit** with explicit paths (both generators, both tests, the regenerated `docs/guides/uat-specs/armory-*.toml`, `lab-run-plan.md`).

### Review and shipping

- One packet-reviewer pass against this section.
- DR-07h lands on `lab/dr-lab` like DR-07d to DR-07g, and ships with the lab results per lab-run-plan § Shipping. It is not a wave of its own.

**Re-run gate (the next canary, within the user's existing lab OK; otherwise ask first):**

- C1 `armory-abilities-soldier-0` on a fresh character: Soldier-717 shows `ability_launched` in SigNoz, and its `armed` clause passes.
- C2 `armory-gear-weapons-1` on a fresh character:
  - GW-3128 runs (rerun unchanged);
  - GW-17's `armed` clause passes (item 17), and `weapon_unbound` is attested under amended R5;
  - GW-5680 shows `use_ability_recv` 1216, and its `armed` clause passes.
- If `armed` fails because `reset-slot1` left slot 1 full (the unequip reveal did not work), stop. Bring it back to the architect: the fallback is a lab-crate fix in `client_item_action` unequip, not a spec change.
