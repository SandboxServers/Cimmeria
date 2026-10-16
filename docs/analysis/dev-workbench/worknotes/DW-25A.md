# DW-25A worknote — 2026-10-16

Added 27 typed, evidence-backed disabled descriptors for the first patch-012 slash slice. Each carries source TOML arguments, per-field help, server handler status, target scope, effect/impact, readback or diagnostic limitation, compensation and a specific blocked reason. The live GM gate remains server-sourced. The command path still lacks proven native submission, client event-to-method binding and cross-source receipts, so no form is runnable and no submission is treated as acceptance.

`slash_descriptors_1a` is a standalone slice; DW-27B owns the six-slice catalog merge. This keeps the existing 162-row conservative catalog unchanged until all slices exist. The optional `/gmshowflag` omission, target-argument mismatches and absent server handlers stay disabled. Boolean/optional display formatting is only a UI preview, never a native line serializer.

Checks: `cargo fmt -p cimmeria-lab` status=ok; `cargo clippy -p cimmeria-lab --all-targets -- -D warnings` status=ok; `cargo test -p cimmeria-lab` status=ok (1559 passed). The exact set, TOML type/order and mutation guard, Boolean/optional preview and submitted-versus-accepted receipt states passed.

Live-client visual UAT: not run; this packet adds data contracts only, with no rendered frontend. Real client submission remains under DW-24B and DW-35 approval.

Review correction: the builder now requires all 27 exact patch event names as well as word and typed argument shape. Unimplemented mission clear-active, clear-history, reset, complete and availability forms have no impact badge because their original effects are unknown; their unsupported reasons say so. Implemented/readable forms append command-specific source-backed preconditions to the common GM and target-scope checks. Regression guards mutate an event, compare every generated argument kind with TOML, inspect the serialized impact labels, and check representative mission, item and training-point prerequisites.

Authority correction: the three unimplemented interaction/address/inventory inspection forms also have unknown/null impact. Mob count readback is restricted to the resolved space and SGWMob class `0x04`, with an uncapped, complete entity query. Mission readbacks bind both authenticated `sgw_mission.player_id` and requested `mission_id`; item readback binds `sgw_inventory.character_id` and requested design `type_id`, then compares instance `item_id`, `stack_size` and `container_id`. Tests assert these exact keys and reject the former broad descriptors.
