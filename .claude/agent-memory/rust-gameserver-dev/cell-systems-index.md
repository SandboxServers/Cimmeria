---
name: cell-systems-index
description: Sub-index of the rust-gameserver-dev cell-system notes (grants, loot, per-session state, abilities, NPC AI, missions, pets, crafting, black market, duels, respawn)
metadata:
  type: reference
---

# Cell systems notes

Moved out of MEMORY.md to keep the index under its read limit. One line per topic file.

- [grant-placement-and-loot-handback-traps.md](grant-placement-and-loot-handback-traps.md) — grants re-placed by container_sets on the base.
- [grant-paths-pick-different-containers.md](grant-paths-pick-different-containers.md) — gmGiveItem grants to bag 1; loot and content grant_item use the first `container_sets` entry.
- [mail-placement-rule-and-fixture-types.md](mail-placement-rule-and-fixture-types.md) — send, take and pay-COD share `take::carried_bag`.
- [per-session-player-state-lifecycle.md](per-session-player-state-lifecycle.md) — a CellEntity field dies on every space change/logout by construction.
- [ability-event-sets-are-server-only.md](ability-event-sets-are-server-only.md) — ability event sets never reach the client (seed-only wiring); most mob kits deal 0 damage.
- [client-action-bar-is-client-side.md](client-action-bar-is-client-side.md) — hotbar bindings are a client Lua saved var; server "hotbar" = `onKnownAbilitiesUpdate`.
- [npc-range-gate-and-weapon-range-columns.md](npc-range-gate-and-weapon-range-columns.md) — four item range columns; range gated in two places; bogus melee `max_range`.
- [npc-ai-fight-test-fixtures.md](npc-ai-fight-test-fixtures.md) — `make_ai_fixture` has no navmesh; assert the INFO log, not `nav_path`.
- [npc-detector-telemetry-traps.md](npc-detector-telemetry-traps.md) — AI-path statics race across tests (use task_local); release detector state on destroy.
- [npc-class-filter-and-dead-target-traps.md](npc-class-filter-and-dead-target-traps.md) — `all_npc_entity_ids` is mob-only; HEALTH alone is not dead.
- [ai-state-private-and-revert-proof-mtime.md](ai-state-private-and-revert-proof-mtime.md) — write `ai_state` via `npc_ai::set_ai_state`; revert proofs must `touch` restored files.
- [no-movement-type-wire-and-nav-path-writers.md](no-movement-type-wire-and-nav-path-writers.md) — no movement-type wire exists; nav_path writes go through `movement_stop`.
- [mission-persist-hydrate-roundtrip.md](mission-persist-hydrate-roundtrip.md) — one serializer, one hydrator; roster rebuilt from `mission_objectives`.
- [stargate-address-book-three-legs.md](stargate-address-book-three-legs.md) — three copies (DB, cell, client); a grant needs client method 66.
- [cell-mirrors-of-base-owned-counters.md](cell-mirrors-of-base-owned-counters.md) — base-owned counters must be messaged to the cell.
- [stat-with-no-consumer-trap.md](stat-with-no-consumer-trap.md) — a stat in `StatList` may have no reader; the dirty-publish pattern.
- [ring-transport-fsm.md](ring-transport-fsm.md) — `disconnect_entity` vs `destroy_entity`; `BSF_*` bits are ref-counted.
- [cross-world-transfer-flow.md](cross-world-transfer-flow.md) — `handle_gate_travel` is the back half; fake default-instance mechanisms.
- [damage-seams-and-gm-index-pins.md](damage-seams-and-gm-index-pins.md) — two damage seams (apply_hit, fire_pulse); scripts write stats directly; two tests pin an unimplemented GM index.
- [session-scoped-cell-state-hooks.md](session-scoped-cell-state-hooks.md) — per-session cell state: key by player_id, tear down on DisconnectEntity only.
- [revert-proof-commit-first.md](revert-proof-commit-first.md) — commit before a revert-proof run; git checkout -- <dir> also wipes uncommitted work.
- [destroy-entity-vs-despawn-npc.md](destroy-entity-vs-despawn-npc.md) — `destroy_entity` sends no LeftAoI; use `despawn_npc` for visible removals.
- [effect-scripts-run-after-the-death-check.md](effect-scripts-run-after-the-death-check.md) — `abilities::death::resolve_death` is the only kill path.
- [kill-credit-seams-and-loot-ownership.md](kill-credit-seams-and-loot-ownership.md) — XP decided in `grant_kill_xp`, mission credit via `credited_player` (4 callers).
- [throttle-key-hides-transitions.md](throttle-key-hides-transitions.md) — key throttles by `(entity_id, kind)`; `destroy_space` is a second teardown path.
- [npc-caster-player-ordered-gates.md](npc-caster-player-ordered-gates.md) — an NPC casting on a player's order skips #444, fire_los and the warmup re-check; add them.
- [native-consumable-round-trip.md](native-consumable-round-trip.md) — item heals/stims: consume-first cell->base->cell round trip (not outbox); stat-buff ledger across 4 crates; flush only expired stats.
- [ability-launch-fire-split.md](ability-launch-fire-split.md) — AT-10: handle_use_ability is launch-only; damage may fire a tick later via fire.rs; ground-AoE tests need class_id 0x04.
- [npc-ai-tick-snapshot-and-hash-order.md](npc-ai-tick-snapshot-and-hash-order.md) — the AI tick's state snapshot goes stale inside a tick; NPCs visit in HashMap order, so multi-NPC tests flake under nextest.
- [crafting-verb-traps.md](crafting-verb-traps.md) — crafting verbs: component sets are subsets (match designs exactly); don't hold a craft to its named instances; `&Completion` across await is not Send.
- [pet-owner-lifecycle-hooks.md](pet-owner-lifecycle-hooks.md) — PT-02: every GateTravel/TeleportPlayer site calls a pets hook (scan-guarded); owner gets no LeftAoI on travel.
- [crafting-induction-engine-seams.md](crafting-induction-engine-seams.md) — crafting engine: global registry + drop hooks.
- [cimmeria-side-flag-bits-collide-with-client-enums.md](cimmeria-side-flag-bits-collide-with-client-enums.md) — check `enumerations.xml` before inventing a flag bit; AF_CHANNEL_ALLOWS_MOVEMENT was SpeedPet.
- [crafting-verb-packet-traps.md](crafting-verb-packet-traps.md) — a new crafting verb breaks stub-pinning dispatch tests in base-world-entry.
- [org-vault-storage-and-lock-order.md](org-vault-storage-and-lock-order.md) — org vault items are their own table; lock order advisory, KEY SHARE player, lock_org, rows.
- [owner-pet-effects-and-passives.md](owner-pet-effects-and-passives.md) — self casts apply no effects; pulse_count=1 buffs never register; passives need 3 seams.
- [duel-end-paths-and-travel-scan.md](duel-end-paths-and-travel-scan.md) — SS-D3: travel sites need `duel::on_travel` (scan test).
- [black-market-escrow-and-authority.md](black-market-escrow-and-authority.md) — listed items live in container 18 (exclude it from client reads); BM lock order.
- [bm-settlement-mail-traps.md](bm-settlement-mail-traps.md) — BM-02b: status gate before any mail (writer mints every call); quarantine = status 4.
- [deployable-pulse-and-seed-traps.md](deployable-pulse-and-seed-traps.md) — `apply_damage_to_target` registers every pulsing effect of its def; DeploymentBar flag is not a spawn marker; templates 200-409 taken.
- [entity-recreate-and-tautological-expectations](entity-recreate-and-tautological-expectations.md) — death pose needs a re-create (NPC: LeftAoI + introduction_events); don't build expectations via the builder under test.
- [stored-target-lifetime-and-gm-view-check](stored-target-lifetime-and-gm-view-check.md) — #844 clears current_target_id; GM targets must be in view.
- [live-loot-containers-and-tag-state.md](live-loot-containers-and-tag-state.md) — open_loot rolls per player_id on a live chest; once flags in sgw_player.looted_containers; entity_tag_state reads live_tags.
- [grantitem-overcap-and-process-wide-gm-switches.md](grantitem-overcap-and-process-wide-gm-switches.md) — GrantItem over-cap row (#1045), use return_rounds; player_id-keyed GM switches in cimmeria-entity.

## Ammo campaign (#1026)

Moved out of MEMORY.md on 2026-09-28 (AM-12 compaction).

- [per-shot-damage-seam-is-damage-apply.md](per-shot-damage-seam-is-damage-apply.md) — per-shot modifiers hook damage_apply, not effect scripts; MITIGATION cap 0 makes armour inert.
- [ammo-reserve-round-trip.md](ammo-reserve-round-trip.md) — AM-02: base loop is sequential, so flush then trust the weapon row; load rounds at draw commit, not in the tick.
- [support-shot-inverse-gate.md](support-shot-inverse-gate.md) — client useAbility has no friend/foe check; beneficial ammo's inverse #444 gate lives at launch, warmup and fire.
- [beneficial-cast-resolution-and-abilitydef-fields.md](beneficial-cast-resolution-and-abilitydef-fields.md) — AB-01 resolver + #444 gate in use_ability/beneficial.rs; 2228 is a Heal-typed attack; new AbilityDef field = ~70 literals.
- [ammo-on-hit-effect-needs-a-script.md](ammo-on-hit-effect-needs-a-script.md) — ammo on-hit effects need a script_name or the hit pulse never fires; no Radioactive dart toggle exists.
- [effect-category-and-friendly-target-gaps.md](effect-category-and-friendly-target-gaps.md) — cleanses key on an `EffectCategory` NVP; no ally targeting (#444); new effect ids must not reach the client.
- [pulsing-script-reapply-and-npc-cc.md](pulsing-script-reapply-and-npc-cc.md) — on_apply runs per pulse/refresh, on_remove once: put stateful scripts on the ledger (state_flags since AB-09); script interrupts queue for combat.
- [mechanical-target-signal-is-body-set](mechanical-target-signal-is-body-set.md) — no mechanical flag exists; use `ammo_emp::is_mechanical` (body_set prefixes); EMP split from grenade 2864.
- [on-hit-fanout-and-recursive-async-send](on-hit-fanout-and-recursive-async-send.md) — scripts can't damage secondaries (no wire/death); fan out in damage_apply; box recursion as a named dyn Send.
- [effect-flag-64-marks-sequenced-damage](effect-flag-64-marks-sequenced-damage.md) — AB-03: NVP damage per TCM_Single effect; single-shot flag-64 rows are vs-low-Focus/chain/barrage follow-ups, never give them NVPs.
- [timed-effect-ledger-and-stat-routing](timed-effect-ledger-and-stat-routing.md) — AB-04 ledger keyed (effect, invoker); script writes it; routing traps for binding stat effects; ability monikers are broad.
- [effect-routing-and-scoped-defs](effect-routing-and-scoped-defs.md) — AB-07 per-effect routing: user/area halves land after the target part; ground/splash scoped defs; routing.py mirror.
- [held-toggles-and-stance-moniker](held-toggles-and-stance-moniker.md) — AB-08 held entries: toggle switch is the last held effect, EFFECT_Stance = CRC-32 via EffectMoniker NVP, held icon horizon.
- [absorb-shield-ledger-and-cleanse-categories](absorb-shield-ledger-and-cleanse-categories.md) — AB-10: shields mirror into absorb* stats, every drain seam must settle; categories from co-sequenced resist rolls; shield rows lack the beneficial bit.
- [combat-debug-notes-and-flush-points](combat-debug-notes-and-flush-points.md) — AB-N1: note beside the AB-T3 row via SpaceManager.combat_debug; lines leave only at scope-close flushes; damage_apply/mod.rs near cap.
