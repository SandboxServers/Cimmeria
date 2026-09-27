---
name: cimmeria-side-flag-bits-collide-with-client-enums
description: Server-invented flag constants (AF_*, etc.) must be checked against entities/defs/enumerations.xml; AF_CHANNEL_ALLOWS_MOVEMENT sat on SpeedPet (16384) and exempted every summon warmup from the move interrupt
metadata:
  type: project
---

`AF_CHANNEL_ALLOWS_MOVEMENT` was defined as 16384 ("reserved bit 14, not in any python
reference"), but `EAbilityFlags::SpeedPet = 16384` in `entities/defs/enumerations.xml:51`.
The seed sets bit 14 on the 14 summon abilities only, so every summon warmup ignored
movement. PT-03 (pets, 2026-09-26) moved it to `1 << 20` and added `AF_SPEED_PET`.

**Why:** "not in python" is not "not in the client". The client enum is the flag namespace;
python only used a subset.

**How to apply:** before adding a Cimmeria-side bit to any client-seeded bitfield (ability
flags, effect flags, entity flags), read the client enum in `enumerations.xml`, pick a bit
above its highest token, and add a live-DB guard that no seed row sets it
(`use_ability/tests/summon_live_db.rs` is the pattern). Also: when parsing
`abilities.sql` with a regex, descriptions span lines — parse the whole file with `re.S`,
not line by line (line-by-line matched only 1032 of 1886 rows).
