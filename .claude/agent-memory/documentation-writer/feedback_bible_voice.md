---
name: feedback-bible-voice
description: Bible chapter voice — conversational, second person, present tense, active voice. No emoji, no exclamation marks. Backtick identifiers and paths.
metadata:
  type: feedback
---

Bible chapters use the same voice as `TESTING.md` and `docs/architecture/`. Match it.

**Why:** Project owner enforces this — the bible is read by engineers across crates and agents, and a consistent voice keeps cognitive load down across hundreds of cross-references.

**How to apply:**
- Second person ("you read"), present tense ("the receiver pops"), active voice ("the sender writes"). Not "we" or "the reader".
- No emoji. No exclamation marks. Bullet points for lists, not for asides.
- Backtick every identifier (`Mercury::Bundle::finalise`), file path (`crates/services/src/...`), constant (`0x5AD`), and Ghidra anchor (`ghidra://SGW.exe@0x0157ac90`).
- Inline `**Divergence:**` callouts next to subsections that diverge from stock BigWorld, then roll up into the §1.13 (or equivalent) divergence table.
- `> [!NOTE]` admonitions for implementation-side asides that aren't Section 1 evidence — flag them as such in the admonition body.
- No line numbers in finding-doc citations (use `§"Section Title"` per `docs/spec/conventions.md`). Line numbers ARE allowed for `deprecated/cpp/...` paths.
- "Worked example" subsections are valuable — they pin the abstract structure to a concrete byte sequence. Keep them.

Related: [[feedback-section1-evidence]].
