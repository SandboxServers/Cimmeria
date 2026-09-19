---
name: interface-element-length-escape
description: Mercury InterfaceElement length fields DO escalate on value — all-0xFF sentinel in the inline field plus a 4-byte real length in the body. Contradicts the "static width only" reading of expandLength.
metadata:
  type: project
---

# InterfaceElement length encoding has a value-dependent escape hatch

**Fact.** SGW's Mercury `InterfaceElement` length framing is *not* purely a static
per-message width. The inline field width is static (`lengthParam` = 1/2/3/4), but
when the payload length reaches the field's max, the sender writes **all-0xFF bytes**
into the inline field and streams the real 32-bit LE length into the message body.

Evidence (all in `docs/reverse-engineering/decompiled/`):

- `Mercury_InterfaceElement_compressLength_2` — `14_standalone_named.c:502519`.
  Switch on `lengthParam` at `this+4`. cases 1/2/3 write the field then test
  `length >= 0xff / 0xffff / 0xffffff` (SBORROW4 idiom); on overflow it tail-calls
  the special path. **case 4 returns immediately — DWORD_LENGTH never escalates.**
- `Mercury_InterfaceElement_compressLength` (special path) — `02_bigworld_network.c:15808`,
  debug string `"compressLength( %s ): length %d exceeds maximum of length format %d"`.
  Loop at the top fills `lengthParam` bytes with `0xff`.
- `Mercury_InterfaceElement_expandLength` — `02_bigworld_network.c:15998` (ghidra
  `0x0158b770`). After reading the inline field it calls `FUN_01578e20(this, len)`;
  false → falls through to the special reader.
- `InterfaceElement_expandLength_1` — `14_standalone_named.c:502594`, debug string
  `"expandLength( %s ): Received a message longer than normal length"`. Reads 4 bytes
  one at a time LE (`|= *p << (i & 0x1f)`, `i += 8` while `i < 0x20`) after seeking
  past `lengthParam + 1`.

**Why it matters.** `WORD_LENGTH` payloads >= 0xFFFF change shape to
`[msgID][FF FF][u32 realLen][payload]`. Any reader that treats 0xFFFF as a literal
length misframes and cascade-corrupts the rest of the bundle.

**How to apply.** Treat 0xFF-saturated length fields as a sentinel, not a value, on
both encode and decode for lengthParam 1/2/3. DWORD_LENGTH (4) is exempt.
`docs/protocol/mercury-wire-format.md:183-210` documents the static switch but omits
this escape — it is incomplete, not wrong.

Related: [[protocol-comparison]]
