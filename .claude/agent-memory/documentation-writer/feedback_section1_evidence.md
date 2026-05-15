---
name: feedback-section1-evidence
description: Section 1 of a bible chapter cites RE findings only — not Cimmeria implementation, not agent memory, not inferred-from-stock-BW claims at high confidence.
metadata:
  type: feedback
---

Section 1 of any bible chapter answers "what does the original binary do, and how do we know?" — the evidence must be RE findings or `external/BigWorld-2.0.1/` stock-BW source. Implementation choices in `crates/` are Section 5, not Section 1. Agent memory is not evidence.

**Why:** Per `docs/spec/conventions.md` and the project owner's direction during the Mercury chapter review pass — Section 1 is canon, and citing Cimmeria as a Section-1 cross-check would create a self-referential loop where the spec validates the implementation that validates the spec.

**How to apply:**
- The §"Source-of-truth crosswalk" subsection in every chapter must cite at least one V5 finding per claim. If the only cross-check is "Cimmeria matches," replace with another V5 doc, an `external/BigWorld-2.0.1/...` path, or `—`.
- Claims like "stock BW default, matches Cimmeria implementation" for retry counts / timeouts should be pinned to `medium` confidence with the inherited-from-stock-BW basis stated explicitly. Upgrade to `high` only when a Ghidra anchor or pcap confirms the value.
- Rationale paragraphs are dangerous. A claim like "the 1453-byte cap is Ethernet MTU minus IP/UDP/HMAC overhead" sounds plausible but is invention if no V5 doc says it. The V5 doc here only says "1453 bytes is the per-packet space check in `Bundle::newMessage`" — keep the constant + anchor and drop the math.
- Agent memory entries (e.g., `mercury-cipher-chain.md`) are author-side notes, not bible evidence — never appear in `evidence_refs` or crosswalks.

Related: [[reference-v5-mercury-evidence]], [[feedback-bible-voice]].
