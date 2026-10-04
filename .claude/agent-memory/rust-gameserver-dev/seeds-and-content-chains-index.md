---
name: seeds-and-content-chains-index
description: Sub-index of seed authoring, content-chain, dialog-bind, inventory-lock and pet/trainer seed memories (moved out of MEMORY.md to keep it small)
metadata:
  type: reference
---

# Seeds and content chains

- [entity-template-seed-authoring](entity-template-seed-authoring.md) — one ability per set; faction 10 is immutable.
- [cover-seed-ids-and-orient-convention](cover-seed-ids-and-orient-convention.md) — cover set ids `world*100000+n`; cover `orient` is not entity yaw.
- [seed-name-id-and-asset-naming](seed-name-id-and-asset-naming.md) — new `texts.sql` moniker ids never render; monikers name UE3 asset families.
- [content-engine-condition-gotchas](content-engine-condition-gotchas.md) — a rejected condition row UNGATES its chain.
- [cell-startup-caches-vs-base-roundtrip](cell-startup-caches-vs-base-roundtrip.md) — the cell has a DB pool and ~20 caches; no base round-trips mid-chain.
- [content-chain-authoring-traps](content-chain-authoring-traps.md) — `display_dialog` needs an interact.
- [content-chain-dispatch-traps](content-chain-dispatch-traps.md) — `dialog_choice` has no archetype.
- [content-chain-condition-context-gaps](content-chain-condition-context-gaps.md) — `archetype neq` fails open on dialog chains; `delay_ms > 0` queues.
- [player-loaded-edge-trigger-race](player-loaded-edge-trigger-race.md) — a gated `player_loaded` chain never fires for a player already inside.
- [edge-trigger-replay-and-abandon](edge-trigger-replay-and-abandon.md) — H52 `enter_region` replay and H54 `mission_abandoned` wiring.
- [chain-replay-trigger-param-vacuity](chain-replay-trigger-param-vacuity.md) — a `TriggerEvent` missing its key param matches nothing.
- [dialog-set-bind-routing-and-edges](dialog-set-bind-routing-and-edges.md) — `target_id` is a dialog_set_MAP id; a bind fans to every entity of the template.
- [dialog-button-strip-and-seed-agreement](dialog-button-strip-and-seed-agreement.md) — linter floors block the packet that changes them; roster pins for patch tests.
- [container-capacity-and-grant-targets](container-capacity-and-grant-targets.md) — raising `bag_max_slots` opens loot/content grants into that bag.
- [inventory-lock-keys-and-failure-injection](inventory-lock-keys-and-failure-injection.md) — writers use different lock keys, so a read-then-send must row-lock.
- [org-cash-balance-and-overflow-traps](org-cash-balance-and-overflow-traps.md) — KEY SHARE does not freeze `naquadah` (use RETURNING).
- [move-path-lock-layers-and-vault-verdict](move-path-lock-layers-and-vault-verdict.md) — moveItem has three lock layers (strip all in a concurrency revert proof).
- [debug-hub-npc-authoring-traps](debug-hub-npc-authoring-traps.md) — Vendor interaction was never set (now derived at spawn).
- [pet-template-seed-traps](pet-template-seed-traps.md) — NoPetLeveling freezes a pet at template level.
- [trainer-seed-and-gm-grant-traps](trainer-seed-and-gm-grant-traps.md) — trainer_abilities.sql is generated.
- [effect-nvp-generator-seed-traps](effect-nvp-generator-seed-traps.md) — effect_nvps.sql loads before effects.sql; heal per-pulse math; heals that must stay unbound until routing.
- [seed-name-columns-and-placeholders](seed-name-columns-and-placeholders.md) — mission_defn is the name (not mission_label); near-empty name columns; "Unused Explosive" is real.
