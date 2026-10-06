---
name: dakara-e1-map-facts
description: Dakara_E1 (worlds 61/62) client-map facts found by DK-02 (2026-10-06) that the code does not record - city wall with three arches, nav components 279/324 unconnected, four furnished camps, covernodes paks are not per-map
metadata:
  type: project
---

Facts from the DK-02 placement pass (ledger: `docs/analysis/dakara-e1-rebuild/placements/`, reproduction: `worknotes/DK-02.md`):

- `dakara_e1.nav`: component 279 = walled city (gate plaza, camps, rings, courtyard ground), 324 = everything outside. They are NOT connected: they come within 1.5 to 3.5 m only at three `JF-HighWallArch00` wall openings (x -212 z 136; x 411 z 203; x 72 z 441). Outside hostiles cannot path in on the shipped mesh.
- Four furnished `JF-MilitaryTent00` camps (A to D, 42 to 125 m from the gate) are direct static meshes with no TriggerVolume, so trigger-based tent surveys miss them. Camp A (x 135, z 270) is nearest and has the `EM-MedicalBox00` props.
- `extract_actors` (Rust) names every actor by class only. Mesh and prefab names need `tools/upk_parser.py` (needs `pip install lzallright`) reading `TemplatePrefab` / `StaticMesh` imports.
- `Cache/covernodes_*.pak` are ZIPs of per-mesh/prefab templates (1,337 names), no map key; Dakara_E1's real cover is 9,847 `SGWSpecCoverNode` actors in 1,643 sets (36 of 400 chunks). Rust reader has 0 parse errors; the audit's "8,022 with 99 errors" was the Python reader.
- `nav_inspect` `dy` = probe height minus polygon height (negative = polygon above the probe).
- The "Eastern"/"Western" of the mission texts is NOT tied to any map axis by the data; no ledger row assumes a direction.

**Why:** these cost a full extraction pass to find and none is in the code. **How to apply:** read the ledger before placing any Dakara actor; do not seed hostiles outside the wall without a nav link.
