---
name: reference-npc-costume-components
description: "2026-10-05 (DA-09): which costume component names the client really loads (BodyComponent exports via query-index scan), and the Ra kit combination that renders as the purple placeholder cube. Read before dressing an NPC."
metadata:
  type: reference
---

Found dressing the Debug Area System Lords (DA-09, `entity_templates_debug_area_lords.sql`), checked in the lab client 2026-10-05:

- **A template's `components` must name `BodyComponent` exports** (`Package.Object`). List them with `query-index scan <CookedPC>/Packages/Character/<pkg>.upk BodyComponent` (`cargo build -p cimmeria-upk-objects --release --bin query-index`). Raw string greps of a `.upk` also return mesh and material names (`*_D`, `*_MI`, `GF_PRX01_*`), which are not components.
- **Ra's NPC kit already includes his crowned head.** Template 41's list renders. Adding `NPC_Goauld.NPC_Ra_Head_00` or `NPC_Goauld.NPC_RaG_FingerNail_00` to it makes the client draw its placeholder cube (a purple box with a cartoon face) instead of the NPC. A cube where an NPC should be means one component does not fit the body: bisect by spawning variants side by side.
- **Female Goa'uld have almost no armour in this client.** `AR_G_*` packages hold only male (`AR_GM_*`) components; the only female outfit pieces are Anat's NPC kit (`NPC_GF_Anat_*_BC`), the `BS_GF_*` base body, hair, heads and face paint, and `ACC_GF_*` accessories/eyewear. Male Goa'uld have Praxis, Ashrak, AS01-03, Underlings and Yellow Trader sets.
- Shipped System Lord templates: Ra 41 (60-63 variants), Ba'al 42 (head only), Anat 43 (full kit), Athena 44 and Morrigan 45 (head and hair only), Nerus 53 (human body, full kit), Lethander 46. Name texts: Ra 20205, Ba'al 8186, Anat 2850, Athena 7563, Morrigan 7521, Nerus 7165.
