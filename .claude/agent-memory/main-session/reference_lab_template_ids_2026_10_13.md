---
name: Lab client mob IDs versus server template IDs
description: A live identity comparison and the AP-10 selector source
type: reference
---

2026-10-13 live local-server comparison by shared entity id in SGC_W1:

| Entity | `client_entity_find.mob_id` | `server_entity_get.template_id` |
|---|---:|---:|
| General Hammond | 2410 | 29 |
| Airman | 0 | 31 |
| Ba'al's Jaffa | 0 | 35 |

`unitMobId` is not `entity_templates.template_id`. The client's `SGWSpawnableEntity.def` calls `mobId` a mob type definition ID; the server's `NpcAoIData`/AoI create cascade does not carry the server template id. For template and dead/alive targeting, join `server_entity_query` in the player's space to the client entity map by BigWorld entity id. Keep the final click or walk in real client input. The behavior and selector interface are documented in `docs/analysis/cellblock-autoplay/scenario-map.md`.
