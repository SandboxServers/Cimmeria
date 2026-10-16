---
name: dev-workbench-overlay
description: D3D9 EndScene render gate and device lifecycle evidence for DW-16
type: reference
---

The existing telemetry DLL D3D9 seam patches `Direct3DCreate9` via an import-checked IAT slot and follows COM `CreateDevice`, `Reset`, and `TestCooperativeLevel`. The bootstrap globally refuses all hooks when the supported SGW.exe code/slot fingerprint mismatches. Windows SDK `d3d9.h` assigns `IDirect3DDevice9::EndScene` slot 42, `Clear` 43, `GetViewport` 48 and `CreateStateBlock` 59; `IDirect3DStateBlock9::Apply` is slot 5. DW-16 adds a lab-bridge-only EndScene patch after bridge startup, then coarse atomic status rendering with `Clear`, `D3DSBT_ALL` restoration, loss/reset/recursion/fault gates, and a three-Clear bound. No original SGW.exe or live rendering was available for visual/driver proof. See `docs/architecture/client-telemetry.md` and `docs/analysis/dev-workbench/worknotes/DW-16.md`.