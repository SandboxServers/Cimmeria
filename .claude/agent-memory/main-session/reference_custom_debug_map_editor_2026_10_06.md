---
name: reference-custom-debug-map-editor-2026-10-06
description: "2026-10-06 Ghidra QA SGW.exe editor-path findings and correction; see the detailed custom-map feasibility study."
metadata:
  type: reference
---

Verified static evidence is in
`docs/analysis/debug-area/custom-map-editor-feasibility.md`. The local QA
executable contains a real New Level dialog (`0x00ef6d30`), a BigWorld chunk
save path (`0x00fede90` / `0x00efb290`), a map-thumbnail builder
(`0x01035830`), and editor rebuild/cover commands (`0x00ff40b0`). The
previously labeled BigWorld wizard at `0x00fe9cc0` is actually the
RandomActorSettings wizard. The editor-to-game saved-package boundary was
not tested; do not treat a fully custom map as proven until an isolated
editor-save and QA-game-load experiment passes.
