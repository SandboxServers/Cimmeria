---
name: client-gm-slash-xml-and-dhd-address-resolution
description: DA-F3/F8 (2026-10-05) client facts - /gm* commands live in InternalSlashCommands.xml (absent from launcher installs); AccessLevel property 7 is the command mask (2 passes 0x2FF); updateStargateAddress resolves async so an open DHD shows "Unknown"
metadata:
  type: project
---

Found fixing the Debug Area live-run failures (DA-06 report, PR from worker daf3).

- **Native `/gm*` "Invalid command." = a missing client file, not the server.**
  `LaunchMisc__InitContentSystems` (0x0041f9c0) loads
  `Common/xml/slash_commands/InternalSlashCommands.xml` then
  `FinalSlashCommands.xml` into `SGWTextCommandMgr` (singleton 0x01ef227c,
  map size at I+0x14). The launcher/retail install lacks Internal (167
  `/gm*`), so the map holds 104 entries; the QA tree has it (266). Shipping it
  is a client-content (owner) decision.
- **`onEntityProperty(7 AccessLevel)` is the client's command role mask**
  (case 7 of `FUN_00e6e9f0` -> `FUN_00c73350` -> 0x01df2d44, bound player
  only). Check: `cmd+0xa8 != 0 && (mask & cmd+0xa8) == 0` refuses. `/gmdhd`
  needs 0x2FF; access level 2 passes. I first guessed a mask mismatch and
  shipped 0xFFFFFFFF; a lab read disproved it. Ask the lab for a memory read
  before changing a wire value on a client-gate theory.
- **`updateStargateAddress` (66) does not resolve on receipt.** The
  `GateTravel` handler `FUN_00e2eff0` builds a record, requests the cooked
  stargate element and parks it in a pending vector (+0x34); an open DHD
  never redraws, so a grant sent with `onDisplayDHD` lists "Unknown". Grant
  earlier (the hub grant now runs in `InitPlayerState`, which follows
  `setupStargateInfo` on the client, so it is not overwritten).
- The client sends nothing when Escape clears the target frame, so the
  server's `current_target_id` goes stale; GM commands that act on
  "target or player" can hit an NPC the tester deselected.
- Headless Ghidra Probe: commas split tokens (use `+`); `FINDPTR` on a
  string VA finds the code that loads a filename.

Related: [[debug-area-plaza-and-training-dummies]], [[client-handler-abi-and-static-disassembly]].
