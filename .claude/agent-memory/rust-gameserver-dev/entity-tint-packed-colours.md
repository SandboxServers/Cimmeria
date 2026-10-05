---
name: entity-tint-packed-colours
description: onEntityTint's three "ColorId" args are packed 0xRRGGBB__ colours (client 0x00e6f8b0); NPC seed bigints are signed 32-bit, send low 32 bits; opt-in send_tint
metadata:
  type: reference
---

Verified 2026-10-05 by disassembling `GameEntity_ApplySkinTintColors`
(`0x00e6f8b0`, QA SGW.exe) with capstone: for each of `primaryColorId`,
`secondaryColorId`, `skinColorId` it stores bytes `v>>8, v>>16, v>>24, 0xFF`
and `0x004f6f20` reads them back as R=`v>>24`, G=`v>>16`, B=`v>>8` (through a
pow call, likely gamma). So the low byte is ignored and alpha is forced.

- `entity_templates.primary_color_id / secondary_color_id / skin_tint` are
  `bigint`; a colour with R >= 0x80 is stored as its signed i32
  (-52773120 = `0xFCDABF00`). Wire value = two's-complement low 32 bits
  (`EntityTint::from_template_columns`). The legacy Python loader negated
  them (`EntityTemplate.py:35-43`), which is wrong.
- Since PR for DA-10 tint: NPCs send them only when `send_tint` is true
  (lineup 1410-1570); everyone else still sends 0,0,0. Follow-up (t) in
  `docs/analysis/debug-area/README.md` turns it on game-wide after a lab look.
- Seed primaries look like placeholder debug colours (0xFFFF0000 yellow /
  0xFF000000 red on "DO NOT USE" templates); expect some garish actors.
- Sed/heredoc traps hit again here: build multi-line CRLF edits in a Python
  file written with the Write tool, not in a bash heredoc
  ([[bash-heredoc-backslash-and-metric-tests]]).
- Where it shows: setTint (0x00e6df40) stores the colours on the entity
  (+0xa0 flag, +0xa4 float4 x3); costume compositing (0x00ebcfe3) sets them
  as material vector params TintBase / TintHighlight / TintSkin. Only some
  packages name them (AR_H_Ablative/Ballistic00/Hazmat00/Lotar, AR_J_*,
  BS_Asgard; TintSkin in BS_HumanMale/Female). Lab A/B 2026-10-05: the
  colour-only pair 1426/1438 (AR_H_Clothing00) looks identical with tint on,
  as the assets predict. Check package names before promising a visual diff.
