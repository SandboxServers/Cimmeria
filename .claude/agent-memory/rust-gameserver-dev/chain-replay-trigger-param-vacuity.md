# Chain-replay tests: the trigger needs its OWN param, or every negative is vacuous

`ChainEngine::resolve_event` matches the trigger BEFORE it evaluates any
condition (`content-engine/src/triggers/matching.rs`). A hand-built
`TriggerEvent` whose `params` lack the key the trigger discriminates on
matches nothing — so the chain resolves an empty action list and every
`assert!(...is_empty())` negative in the file passes for the wrong reason.

Per-trigger required param (all read off `event.params`, NOT `ctx` fields):

| Trigger | Required param |
|---|---|
| `OnDialogOpen`, `OnDialogChoice` | `dialog_id` (i64) |
| `OnItemUse`, `OnItemEquipped` | `item_id` |
| `OnInteractTag` | `entity_tag` (str) |
| `OnInteractTemplate` | `template_name` |
| `OnMissionAccepted`, `OnMissionCompleted` | `mission_id` |
| `OnPlayerLoaded` | `world_name` (str) — `Option`, `None` matches all |
| `OnStargateDialed`/`Crossed` | `destination_world` — `Option`, `None` matches all |

`world_id` is an `ExecutionContext` FIELD, not a param, and it feeds
`Condition::World`, not any trigger.

The trap is that `player_loaded` and `interact_tag` tests usually set
their key incidentally (a `ctx_world_entry` / `ctx_planting` helper), so
the file looks consistent while the dialog and item_use tests silently
guard nothing. Found in the Harset H40/H41 packet: four `mission_1200`
guards failed the moment they were first run, and both files' dialog
negatives had been green on a miss.

**Pattern:** give the module a `ctx_dialog(dialog_id, ...)` /
`ctx_item_use(item_id, ...)` helper so the key cannot be forgotten, and
sanity-check any all-negative test by making one case positive.

See also [[vacuous-guard-and-sentinel-collision-review]],
[[content-chain-dispatch-traps]].
