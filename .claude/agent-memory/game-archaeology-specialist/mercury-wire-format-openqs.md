---
name: mercury-wire-format-openqs
description: Answers to five open questions from the Mercury wire-format bible chapter, resolved by Ghidra session 2026-05-14.
metadata:
  type: project
---

Five open questions from `docs/drafts/spec/mercury-wire-format.md` resolved in Ghidra session 2026-05-14.

**Q1 — InterfaceElement compressed-length thresholds**

The `compressLength_write` function (`0x0158b120`) does NOT use per-threshold escape sentinels like the MachineGuard single-byte scheme. Instead, the InterfaceElement stores a fixed `length_field_width` value (1/2/3/4 bytes) in its descriptor at `+0x4`, and the encoder always uses that width. The thresholds are the natural capacities of the chosen width:
- width=1: max 0xFF (255)
- width=2: max 0xFFFF (65535)
- width=3: max 0xFFFFFF (16777215)
- width=4: always succeeds

If the value exceeds the capacity for the configured width, it falls through to `Mercury_InterfaceElement_compressLength` (`0x0158acc0`) for multi-packet/overflow handling. The width is per-interface, not dynamically selected by value. Key decompile: `0x0158b120`.

**Why:** The existing Ghidra annotation on `0x0158b120` was accurate — the scheme is width-per-interface, not a single variable-width encoding with sentinel thresholds.

**Q2 — ChannelInternal +0x170/+0x174 timer field roles**

From `ChannelInternal__checkAndSendNubException` at `0x0158bed0`:

- `+0x170`: low 32 bits of the **last-received-packet rdtsc baseline timestamp**
- `+0x174`: high 32 bits of the same rdtsc baseline timestamp

These form a 64-bit rdtsc pair. The function computes `elapsed = current_rdtsc - {+0x174, +0x170}` and compares against the recv-timeout threshold at `+0x160/+0x164`. If exceeded, throws `NubException`.

Both fields are initialized to zero in the constructor at `0x0158c9d5`/`0x0158c9db`. The write site (where they are updated on each received packet) was not found in this Ghidra session — it is likely in the Nub-level receive loop before `dispatchPacketWithFilter` is called. Medium confidence on role; high confidence that they are NOT counters or flags.

**Q3 — forcedPosition velocity Vec3 + standalone-path rotation semantics**

Q3a — Velocity semantics at the client:

The "velocity Vec3" in the chapter description is a misnomer. Decompile of `ProcessForcedEntityPosition` at `0x00dd9ee0` shows the handler passes message fields at `pMsg+0x24`, `pMsg+0x28`, `pMsg+0x2c` to `PackageAndSendEntityMove` as the `flYaw`, `flPitch`, `flRoll` float parameters (loaded via FLD at `0x00dd9f5a`, `0x00dd9f73`, `0x00dd9f7a`). `PackageAndSendEntityMove` does NOT have a velocity parameter — it takes orientation and position. The 12-byte field the chapter calls "velocity Vec3" is actually the **yaw/pitch/roll rotation** encoded as three floats. There is no velocity application at all — no delta-replacement, no additive impulse. The field is rotation, not velocity.

Q3b — Server-side emit triggers:

Cannot be answered from the client binary. `ProcessForcedEntityPosition` is the CLIENT-side receive handler registered via a dispatch table DATA write at `0x017bcb17`. The server-side emit code (what triggers the send) does not exist in the client binary. The RTTI type is `ClientMessageHandler<forcedPositionArgs@ClientInterface>`. Only a server binary analysis can answer Q3b.

**Q4 — MachineGuard port hex/decimal mismatch**

The port is **20022** (hex `0x4E36`). The decimal "19510" in the chapter is wrong.

Evidence: `Mercury_MachineGuard_sendAndRecv` at `0x015898c0` contains `htons(0x4e36)` (PUSH immediate `0x4E36` at `0x0158994b`, then `htons` call at `0x015899af`). The byte pattern `36 4E 00 00` appears at exactly one location in the MachineGuard range: `0x0158994b`. The pattern `36 4C 00 00` (which would give 19510) does not appear anywhere in the binary.

The existing Ghidra plate comment on `Mercury_MachineGuard_sendAndRecv` incorrectly says "port 0x4e36 (19510)" — the annotation was written with bad arithmetic and propagated into the V5 chain.

`0x4E36 = 4×4096 + 14×256 + 3×16 + 6 = 16384 + 3584 + 48 + 6 = 20022`.

**Q5 — ChannelInternal send-window slot count**

There is no fixed "send-window slot count" in the traditional circular-buffer sense. The ACK mechanism uses a 32-bit sliding bitmap window (confirmed by `UnAckedHandler__buildAndSendAckBundle` at `0x0158b2d0` which iterates `iVar2` in steps of 8 up to 0x20 = 32 bits).

The 512-entry value: `Channel__ctor` at `0x01576bf0` stores `0x200` (512) at `+0x2c`. This value is passed to `FUN_0158c170` (the hash table allocator for `ChannelInternal+0x40`) which allocates `512 * 4 + 4 = 2052` bytes and stores the mask `511` (0x1FF) at `ChannelInternal+0x44`. This is the **received-sequence-number hash table** (indexed by `seq_num & 0x1FF`), not a send-window count.

The "45-slot" claim from prior docs was unsourced and incorrect. The actual sequence tracking table has 512 entries; the ACK bitmap covers 32 outstanding sequence numbers at a time.

**How to apply:** Feed these directly into the five open-question blocks in `docs/drafts/spec/mercury-wire-format.md`. Q4 is highest priority — it's an embarrassing arithmetic error. Q3a corrects a semantic misidentification (velocity vs. rotation). Q1 corrects a threshold model misunderstanding. Q5 replaces an unsourced claim.
