# Union Omega: ID amendment request

> **Type:** Reference · **Audience:** Union coordinator · **Last updated:** 2026-10-18 · **Packet:** [UO-00](UO-00.md)
> **Companions:** [Omega ledger § ID map](../omega-work-packets.md#id-map-d-uo1), [handoff note](handoff-sgc-beta.md), [four-zone id-allocation.md](../../four-zone-planning/id-allocation.md)

Request; the coordinator merges into id-allocation.md. Omega does not edit that file.

## The row

It sorts after the `11500–12999` Cross-zone row, because `id-allocation.md` has no `63500–63799` Dakara row yet (Dakara's UD-00 sends its own request). If that row lands first, this one goes right after it. Beta's `64000–64499` row (D-UB9, request UB-00c) goes after this one.

```markdown
| 63800–63999 | Union Omega | Omega Site (worlds 18 and 80): content chains, entity templates, spawns and item lists in 63800–63999, item-list rows 638000–639999, dialog set and maps 63800–63803, respawner 180. Ledger: [omega-work-packets.md](../union-zone-planning/omega-work-packets.md). |
```

## Collision checks

Run against `origin/main` (1303888d3, equal to this worktree's base) and every sibling ledger on 2026-10-18.

| Check | Result |
|---|---|
| `id-allocation.md` on `origin/main`: any `63xxx` or `64xxx` row | None. The block is free there. |
| Dakara: `dakara-work-packets.md`, `dakara-packets/` | Owns 63500-63799. Ends at 63799, so no overlap or gap. |
| Beta: `beta-work-packets.md:92-100` | Authored rows start at 64000 (D-UB9, 64000-64499). No overlap. |
| SGC: `sgc-work-packets.md:229` | Lists Omega as 63800-63999. The older Omega ranges (12000-12399, 63000-63099, 64000-64199) appear in no SGC, Beta or Dakara file. |
| Seeds, 5-digit ids `\b63[89][0-9]{2}\b` under `db/resources/` | Only Omega's own scaffold files (`union_omega_*_chains.sql`, `dialog_sets_union_omega.sql`) plus the **screen** finding below. |
| Seeds, 6-digit ids `\b63[89][0-9]{3}\b` | Only `item_lists_union_omega.sql`. The `cover_nodes.sql` hits are float coordinates and node ids 800xxx. |
| Dialog sets and maps 63800-63803 | `dialog_sets.sql` and `dialog_set_maps.sql` hold no such id. |

**Finding: stock dialog screens sit inside the block.** `db/resources/Dialogs/Seed/dialog_screens.sql` uses `screen_id` 63847-63854, 63874 and 63881-63887 (for example `:9376`, `:13826`, `:15923`), and `dialog_screen_buttons.sql:3548` uses screen 63887. Omega's ledger allocates no screen ids (it reuses dialogs 4641 and 5025, and its own dialog set holds only the set and map rows). The block therefore stays valid, with one rule: **no Omega packet may allocate a `dialog_screens` or `dialog_screen_buttons` id from 63800-63999.** UO-15 (HELD, authored ambient dialogs) must take screen and button ids from a range the coordinator assigns when it unholds.

Do not open `id-allocation.md` for writing from Omega.