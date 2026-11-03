# Soldier-1475 Flushing Fire: hotbar press ignored by the client, nothing reaches the server (lab, cause unclassified)

Status: resolved. The full-file rerun after DR-07l/m passes Soldier-1475 (two presses reach the server, two `ability_launched`); see [DR-L2-soldier-0.md](../DR-L2-soldier-0.md#merged-final-grades). Kept as the record of the first run.

Rows: `armory-abilities-soldier-0`: Soldier-1475.
Runs: `%LOCALAPPDATA%\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqjljvxcsrgh` (row JSON `rows/armory-abilities-soldier-0/Soldier-1475.json`).

Expected: `@use_ability place` puts ability 1475 on the bar, the press on key `0` leaves the client (`client.ability.sent`) and the server launches it. The ability is sound on the server side: DD, effect 1767 with `FocusDamage 100`/`HealthDamage 10`, moniker 728213066 (shared with 713, 716, 722 which launched with the same SMG).

Observed: the setup `place` call (action 9) placed the ability on button 10 (key `0`), then the press returned `denied: ignored: The game read the input but nothing changed` (`ability_sent 0`, `verdict nothing_observed`). The preceding `select-slot1` F1 key and both window clicks were `posted_only`. SigNoz scope `abilities`, `player_name CONTAINS 'Cgbqjljvxcsrgh'`, 19:35:54Z to 19:36:08Z: no `use_ability_recv` for 1475, no refusal. The previous row (1446) ended 19:35:34 and the next (1445) pressed fine at 19:36:11.

Repro: `lab_uat_run {sections: ["armory-abilities-soldier-0"], rows: ["Soldier-00", "Soldier-1475"], specs_dir: "C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab\docs\guides\uat-specs"}`. Re-run alone first: it may be a one-off (UI focus), in which case close this.

Area: unknown (lab tool or the row's select-slot / place steps).
Labels: lab (see docs/agents/).
