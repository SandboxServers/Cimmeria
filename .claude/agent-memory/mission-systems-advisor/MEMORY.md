# Mission Systems Advisor — Memory Index

- [content-engine-once-semantics.md](content-engine-once-semantics.md) — CRITICAL: the `once` trigger column is DEAD CODE; not enforced anywhere. Use step_status/mission_status gates for one-shot guards.
- [castle-cellblock-chains.md](castle-cellblock-chains.md) — Mission 622/638/639/640/641 chain shapes, interactability gating, re-loot guard patterns, dialog-set→template binding.
- [engine-gaps.md](engine-gaps.md) — Triggers that never dispatch (`dialog_set_open`!), actions with no executor arm, no timers, no spawn/XP/cash, no per-player state; `MissionObjective` enum is dead code.
- [harset-zone-evidence.md](harset-zone-evidence.md) — Harset has exactly 1 mission script (742) + 2 space scripts, zero chains, and a 22-row spawn table; mission 742's full chain shape and template ids.
- [atrea-node-mapping.md](atrea-node-mapping.md) — Atrea `Event_*`/`Act_*` node → Cimmeria trigger/action mapping, with the nodes that have no port.
- [dialog-chain-authoring-rules.md](dialog-chain-authoring-rules.md) — CRITICAL: non-interact chains can only display MONOLOGUE dialogs; `dialog_choice` gated on `open_dialog_id`; NULL-dialog_set binds are no-ops that kill clickability; buttons keyed by screen_id.
