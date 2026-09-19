# Project-v1 Combat Status

The previously missing combat formula areas now have **PROJECT FINAL v1** replacements: Accuracy/Cover, Armor/Resistance/Penetration, Focus/Health exposure, status resistance, AoE/LoS, TechComp/Quality scaling and stacking/caps. The authentic retail formulas remain unknown and should still be treated as unrecovered historical facts.

# Known Unknowns / Do Not Autocomplete

## Combat
- Exact Accuracy vs Defense formula.
- Low/Medium/High cover numeric modifiers.
- Cover Penetration interaction.
- Armor/resistance/penetration formula.
- Exact Focus-depleted accuracy/Health formula.
- AoE/cover/LoS behavior.
- TechComp/Tier/Quality scaling.
- Global stacking/caps and level scaling.

## Skill progression
- Exact original per-node unlock levels for most trees.
- Exact original skill-point gain cadence/economy.
- Meaning of all `Training Cost = 0` cases.
- Full original prerequisite graph.
- Jaffa Tau'ri branch is particularly revision-heavy.
- Commando Stealth/Infiltration and Precision/Marksmanship branch labels are reconstruction/secondary evidence.
- Scientist Robotics and Archaeologist Archaeology branch labels are strong secondary-evidence reconstructions.

## Asgard
- Exact Energy/Gigajoule pool and regeneration formula.
- Many descriptions still contain `Cost: X Gigajoules`.
- Exact relationship between equipped program item, TechComp and granted/scaled abilities.

## Character starts
- Exact starter inventory/armor/weapon package is not yet finalized for every start profile.
- `char_creation_abilities` legacy rows are not authoritative enough to define canonical starts.

## Worlds
- Naitac World 16 has an empty Cooked ClientMap and remains unfinished/partial.
- Pen-Lai production UMAP is absent from the uploaded raw map set.
- Many dynamic spawns/regions/scripts depend on missing server-side data.

## Castle CellBlock
Strict production scope ends at Mission 688 `Secure the Armory`.
Do not mix Mission 701+, Copplemann/Zuritska Castle-main content, or later Castle progression into CellBlock runtime.

## QA-client caveat
The client is a 2009 development snapshot. Test/revision content can exist beside intended gameplay content.
