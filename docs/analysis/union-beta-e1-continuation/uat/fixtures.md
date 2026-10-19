# Beta E1 lab fixtures (UB-90)

> **Type:** Reference · **Index:** [uat/README.md](README.md) · **Tool:** `server_stage_fixture` ([live-research-lab.md § Staging a row fixture](../../../guides/live-research-lab.md#staging-a-row-fixture-server_stage_fixture))

Every run starts from one fixture below. The **stager** calls `server_stage_fixture` on the `lab-server` MCP. The stager is the orchestrator's helper: a `hands-on` or `general-purpose` agent that has that MCP. It is never the `lab-driver`, which has no such tool. The tool writes nothing at once: the server applies the fixture at the character's next Play, once, within 10 minutes. So:

1. Make sure the fixture character is not in the world. A stage on an online character is refused with `online`. If that happens, dispatch a one-call driver (`lab_logout`) and stage again.
2. Stage. Expect `{"staged": true, ...}` and the INFO line `lab fixture staged`.
3. Dispatch the driver at once. Its first call is `lab_ensure_in_world` with the character. Grade nothing until the server logs INFO `lab fixture applied` for that character. On ERROR `lab fixture not applied`, the run is BLOCKED: record the `reason`.

A fixture writes silently, so no content event, reward or tutorial fires (D-FX4). The first `player_loaded` after it is the row's first real event.

## Characters

A character's name follows the runner's profile rule: `<first> Fx<key><instance>`. The default instance is `defa` and p2 is `pc` ([automated-uat.md § profiles](../../../guides/automated-uat.md)). Each one must be on a GM account and created after the last DB reload, because a reload deletes lab characters.

| Key | Name (default / p2) | Race | Who creates it |
|---|---|---|---|
| `betahum` | `Uat Fxbetahumdefa` / `Uat Fxbetahumpc` | Human SGU Soldier, male | The driver, on first use: `lab_ensure_in_world {"lease_id":L,"create":{"alignment":"sgu","archetype":"Soldier","gender":"male","first":"Uat"}}`. Check that the name it reports is the one in this table |
| `betafj` | `Fxbetafjdefa` (default only) | Free Jaffa (archetype 7) | **By hand, during the approved lab session**: the user makes it at character select. `lab_create_character` cannot make a single-name race |
| `betaasg` | `Fxbetaasgdefa` | Asgard (archetype 5) | Not needed until UO-16 (the Asgard rows are NOT RUNNABLE). Made by hand, like `betafj` |

ASSUMPTION: the lab's `create` names the character after its profile key, as `lab_uat_run` does. If the name it reports differs, stage that name instead, because the stager matches the last word ignoring case. The last word must start with `Fx`.

## State vectors

Every fixture lists every mission of its arc and every mission item, so a leftover state from an earlier run cannot leak in. All items are in the mission bag (`container_sets {2}`). `world` is the `resources.worlds` name. If the stager refuses the name, use id 23 for Beta and 18 for Omega (ASSUMPTION: the tool takes the name).

- **Human arc:** 1423, 381, 473, 478, 1425, 1426, 1427. Items: 2636 Carter's Inventory Checklist, 3635 Xenobiology Cage (stack 5) and 4832 Digital Camera.
- **Free Jaffa arc:** 1500, 1502 and 1503 (1503 is not built, D-UB5 A). Item: 5175 Sacred Tribal Medallion. If the stager refuses 1503 as missing from the seed, drop it.
- **Asgard arc:** 481, 420, 421, 1147, 443, 477, 479. Items: 3399 the Communiqué, 2647 Asgard Scanner (stack 5) and 2646 Asgard Recording Device.

`NA` below means `{"id":<m>,"state":"not_active"}`, `C` means `{"id":<m>,"state":"completed"}` and `S<n>` means `{"id":<m>,"step":<n>}`. Every item not named below is staged at count 0.

| Fixture | Character | World and position | Missions | Items | Used by |
|---|---|---|---|---|---|
| F-H1 | betahum | `Omega_Site` [34.70, -51.42, -138.00] (respawner 180, 11 m from the gate) | 1423 S4223, the rest NA | none | UB-U1, UB-U1b |
| F-H2 | betahum | `Beta_Site_Evo_1` [1009.4, 13.57, 507.85] (the Carter stand) | 1423 S4223, the rest NA | none | UB-U3, U3c, U3d, U6b (247), UR1, U9 (p2) |
| F-H3 | betahum | `Beta_Site_Evo_1` [712.4, -65.99, 408.2] (cage 1 stand) | 1423 C, 381 S1409, the rest NA | none | UB-U3b, UB-U7b |
| F-H4 | betahum | `Beta_Site_Evo_1` [1009.4, 13.57, 507.85] | 1423, 381, 473, 478 C; 1425, 1426, 1427 NA | none | UB-U4, U4b, U2b, U6b (456, 457, 458) |
| F-H5 | betahum | `Beta_Site_Evo_1` [633.0, -59.83, 405.0] (6 m off respawner 6) | 1423 C, the rest NA | none | UB-U7, UB-U2, UB-U2c |
| F-FJ1 | betafj | `Beta_Site_Evo_1` [1009.4, 13.57, 515.85] (Carter, north side) | 1500 S4369; 1502, 1503 NA | none | UB-U6, U6b (575, 576), U2b, U9 (default) |
| F-FJ2 | betafj | `Beta_Site_Evo_1` [146.7, 9.77, 409.3] (6 m off the hub ring pad) | 1500 C, 1502 S4376, 1503 NA | none | UB-U8 |

### Fallback fixtures for UB-U6b

Use one only when the button was never pressed in its arc run: the arc FAILed before that step, or its log line is missing. Each one is a one-button run (UB-U6b § Fallback).

| Fixture | Character | Position | Missions | Items | Button |
|---|---|---|---|---|---|
| F-B247 | betahum | [1014.9, 13.57, 508.35] (hub computer stand) | 1423, 381, 473 C; 478 S1747; the rest NA | none | 247 on 3224 (AdminWorkstation) |
| F-B456 | betahum | [1014.9, 13.57, 508.35] | 1423-478 C; 1425 S4242; 1426, 1427 NA | none | 456 on 4655 (HubComputer) |
| F-B457 | betahum | [653.9, 19.77, 810.6] (Science stand) | 1423-478 C; 1425 S4243; 1426, 1427 NA | none | 457 on 5497 (ScienceComputer) |
| F-B458 | betahum | [1014.9, 13.57, 508.35] | 1423-478 C; 1425 S4244; 1426 S4246; 1427 NA | 4832 x1 | 458 on 4659 (HubComputer) |
| F-B575 | betafj | [-548.6, 6.77, 143.5] (camp stand) | 1500 C; 1502 S4377; 1503 NA | none | 575 on 5042 (JaffaBox) |
| F-B576 | betafj | [-548.6, 6.77, 143.5] | 1500 C; 1502 S4378; 1503 NA | none | 576 on 5043 (JaffaBox) |

### Asgard fixtures (NOT RUNNABLE until UO-16)

| Fixture | Position | Missions | Items | Row |
|---|---|---|---|---|
| F-A1 | [1009.4, 13.57, 507.85] | 481 S1760; the rest NA | 3399 x1 | UB-U5, UB-U9 (Asgard client) |
| F-A2 | [699.0, 158.77, 70.4] (deck, 4 m off Mimir) | 481 C; 420 S1455; the rest NA | none | UB-U5b |
| F-A3 | [699.0, 158.77, 70.4] | 481, 420, 421, 1147, 443 C; 477, 479 NA | none | UB-U5c, UB-U10 (477 offer) |
| F-A4 | [699.0, 158.77, 70.4] | 481-477 C; 479 NA | none | UB-U10 (479 offer) |

## Example call (F-H2)

```json
{ "character": "Fxbetahumdefa", "world": "Beta_Site_Evo_1", "position": [1009.4, 13.57, 507.85],
  "missions": [{ "id": 1423, "step": 4223 }, { "id": 381, "state": "not_active" },
               { "id": 473, "state": "not_active" }, { "id": 478, "state": "not_active" },
               { "id": 1425, "state": "not_active" }, { "id": 1426, "state": "not_active" },
               { "id": 1427, "state": "not_active" }],
  "items": [{ "id": 2636, "count": 0, "container": "mission" },
            { "id": 3635, "count": 0, "container": "mission" },
            { "id": 4832, "count": 0, "container": "mission" }] }
```

The other fixtures follow the same shape. Spell each mission out; never stage a range.
