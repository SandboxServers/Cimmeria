---
name: controller-mouse-input-model
description: ASGWController_Player mouse/movement handlers, button/mode bits, drag counters, click classification; DI pump and camera AddYaw/AddPitch (static RE 2026-10-22)
type: reference
---
Full write-up: docs/reverse-engineering/findings/controller-mouse-input-model.md.

- PC vtable 0x019e2b2c, PlayerTick 0x00e84d70. LMB 0x00e85710, RMB 0x00e85860, shared helper 0x00e85540, MouseMove 0x00e83f40 (both buttons orbit the camera).
- PC+0x49c bits: LMB, RMB. +0x498 bits: 0 mouselook, 1 jump, 2 autorun, 3 mouse-walk. +0x4a0/+0x4a4 drag counts; a drag under 5 counts is a click: LMB select 0x00e84ac0, RMB interact 0x00e84b20. +0x4c0 hover id. +0x3f4 PlayerInput (+0xa4 turn, +0xa8 fwd, +0xc0 strafe, +0xc4 up).
- The pawn faces the camera while bit 0 is set: 0x00e7b240. Camera AddYaw 0x00e7e600, AddPitch 0x00e7e590.
- DI pump 0x01234770, GetDeviceData vt index 10. The telemetry DLL's DInput hook is lab-bridge-only.
- Static only. Handler→action rows for strafe, turn, zoom and crouch are inferred from subscription order (0x00e85c00).
- LN-15 (static, section 6 of the finding): 0x0056b890 is a CEGUI hit test (UIMgr global 0x01ee2a58, +0xc System, sheet System+0x2c, getChildAtPosition 0x011a55b0, not-"BackgroundWin"), no flag. Nearest poll: System+0x28 d_wndWithMouse (set in injectMouseMove 0x011ae5b0); differs on capture/modal and goes stale with no mouse move.
- Release never touches +0x4a0/+0x4a4 (D-LN5 holds). Click handlers re-pick via 0x00e84860, never +0x4c0. +0x498 writers: 0x00e83a73, 0x00e83e10, 0x00e83e56, 0x00e853a9, 0x00e856d5, 0x00e8583e, 0x00e85946, 0x00e859a4.
- Ghidra global g_pFlashExternalWindowModule is really GEngine (+0x2d0 GamePlayers). Headless RE:0x498\] scan is the fast field-writer search.
