---
name: world-entry-resolved
description: World-entry open questions Q1-Q4 resolved: ENABLE_ENTITIES=8 bytes (corrected), PostLoad handler=0x00de8430, World_Loaded emitter=0x005541a0, ClientReady wire-send=0x00d43dc0.
metadata:
  type: project
---

World-entry facts recovered from SGW.exe (full detail in `docs/reverse-engineering/findings/world-entry-pipeline.md`):
- ENABLE_ENTITIES is 8 bytes (a SGW-custom uint64 dummy payload), not the stock BigWorld 1-byte keepBase. An earlier version of this note said 1 byte; it misread a reliability flag as the size.
- The PostLoad handler is `0x00de8430`.
- The `World_Loaded` emitter is `0x005541a0`.
- The ClientReady wire send is `0x00d43dc0` (the shared NetOut member callback).
Re-verify the last three against the finding before relying on them.

INVALIDATED WHEN: the world-entry-pipeline finding revises the PostLoad, World_Loaded or ClientReady addresses
checked: 2026-11-01
