# Mission Systems Advisor — Memory Index

- [content-engine-once-semantics.md](content-engine-once-semantics.md) — CRITICAL: the `once` trigger column is DEAD CODE; not enforced anywhere. Use step_status/mission_status gates for one-shot guards.
- [multi-chain-dispatch-semantics.md](multi-chain-dispatch-semantics.md) — CRITICAL: one event fires EVERY matching chain; conditions frozen at resolve time; `priority` never excludes. Sibling chains on one trigger key must be pairwise disjoint.
- [interact-dialog-routing-traps.md](interact-dialog-routing-traps.md) — CRITICAL: an `interact_tag` chain short-circuits `handle_interact`, so `last_interaction_target` is never pinned and a follow-up `dialog_choice → display_dialog` bails. Also: no distance check on tag chains; nested mission-event dispatch runs mid action list.
- [condition-column-layout.md](condition-column-layout.md) — Which `content_conditions` column each condition type reads; a malformed row leaves the chain UNGATED, not disabled.
- [advance-step-vs-complete-objective.md](advance-step-vs-complete-objective.md) — advance_step force-completes objectives but skips the COMPLETED wire tick; MissionUpdate persists the step id into active_objective_ids.
- [castle-cellblock-chains.md](castle-cellblock-chains.md) — Mission 622/638/639/640/641 chain shapes, interactability gating, re-loot guard patterns, dialog-set→template binding.
- [engine-gaps.md](engine-gaps.md) — Triggers that never dispatch (`dialog_set_open`!), actions with no executor arm, no timers, no spawn/XP/cash, no per-player state; `MissionObjective` enum is dead code.
- [harset-zone-evidence.md](harset-zone-evidence.md) — Harset has exactly 1 mission script (742) + 2 space scripts, zero chains, and a 22-row spawn table; mission 742's full chain shape and template ids.
- [atrea-node-mapping.md](atrea-node-mapping.md) — Atrea `Event_*`/`Act_*` node → Cimmeria trigger/action mapping, with the nodes that have no port.
- [dialog-chain-authoring-rules.md](dialog-chain-authoring-rules.md) — CRITICAL: non-interact chains can only display MONOLOGUE dialogs; `dialog_choice` gated on `open_dialog_id`; NULL-dialog_set binds are no-ops that kill clickability; buttons keyed by screen_id.
