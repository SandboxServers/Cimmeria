---
name: template-look-copy-and-client-evidence
description: Copying a character look into a new entity template must use a source with a LOWER id (the Debug Area lineup tags actors by lowest wearer); how to confirm names, speakers, components and meshes in the client's cooked files in minutes without building anything
metadata:
  type: reference
---

Learned on Dakara_E1 packet DK-03 (templates 440-457, 2026-10-06).

## A copied look needs a source with a lower template id

The Debug Area lineup (`live_db_debug_area_lineup.rs`) keys on looks, not
templates: body set, components as a set, the two colours, skin tint, static
mesh. A new template that copies a look whole adds no look, so
`..._every_look_has_an_actor` stays green. But
`..._nameplates_and_tags_name_the_source` names each actor after the **lowest
template id that wears its look**. Copy from a template with a higher id than
yours (443 copying 1401) and that test asks for the lineup actor's tag,
nameplate and doc row to be renamed. Copy from a lower id instead (167), or
accept the rename. Props (`GLB_Components.*`) and `WP-Human.*` are outside the
lineup.

## Nameplate rules that decide a prop's class

- Names go only to SGWBeing classes (`class_binds_being_methods`, wire class
  1..=5). A `class = 'spawnable'` prop is sent no name at all: a named prop
  must be `being`.
- A non-empty `display_name` suppresses the name id and sends
  `onBeingNameUpdate` instead. Use it when the client's string for the actor
  is empty. On screen it was proven on mobs only (lineup, 2026-10-05); a
  `being` prop with a literal name was first seeded in DK-03 and not yet seen.
- `static_interaction_sets` holds `dialog_set_maps` ids. Nothing reads it but
  the org registrar; it never makes a prop clickable. Only an
  `interaction_type` bit does.

## Reading the client's cooked files as evidence (read-only, no build)

- `Working/SGWGame/SourceCache.en-us/*.pak` are plain zips; entry `_<id>` is one
  XML record. `TextStrings.pak` (`Text=`, `MonikerName=`), `CookedDataDialogs.pak`
  (`Screens SpeakerID=`), `CookedInteractionSet.pak` (= `dialog_set_maps`).
  `python3 -I` with `zipfile` is enough.
- `CookedPC/Packages/**/*.upk` (not the maps) have `compression_flags == 0`, so
  `tools/upk_parser.py`'s `PackageReader.parse()` gives the full export table
  with no `lzallright`. `get_export_class_name` separates `BodyComponent` and
  `StaticMesh` exports from textures and materials. Check the flag before
  trusting `strings` or `grep` on a package: on a compressed one they see
  fragments.

Related: [[entity-template-seed-authoring]], [[seed-name-id-and-asset-naming]],
[[live-db-seed-iteration-without-reload]].
