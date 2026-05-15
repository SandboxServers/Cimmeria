---
name: feedback-source-doc-override
description: When a bible chapter contradicts a V5 finding doc, use an inline source-doc-override callout and cite the Ghidra evidence that proves the override.
metadata:
  type: feedback
---

When chapter Section-1 evidence contradicts an upstream V5 finding doc, do not silently "win" the contradiction. Mark it inline with a `> [!NOTE] **Source-doc override:** ...` callout that names every V5 doc that carries the wrong claim, summarizes the Ghidra evidence that establishes the correct claim, and tells future readers which evidence supersedes which. Also add a crosswalk row for the override itself.

**Why:** Per the bible's enforcement contract (per the documentation-writer system prompt and the Mercury chapter consolidation pass of 2026-05-14), the bible wins when it disagrees with another doc — but a future agent or human re-reading the V5 finding would otherwise have no idea that the contradiction was deliberate. The override callout converts a silent contradiction into an explicit one with an audit trail. The Mercury chapter closed five Q's this way:
- Q3 (forcedPosition offsets 24-35 are previous-position-reference, not velocity) — overrode three V5 docs.
- Q4 (MachineGuard port = 20022 decimal, not 19510) — overrode two V5 docs.

**How to apply:**

- Inline override callout sits next to the corrected claim with `> [!NOTE] **Source-doc override:**` in bold.
- The callout body lists every V5 doc and section that carries the wrong claim, names the Ghidra anchor(s) that establish the correct claim, and ends with "chapter overrides".
- Add a row to the §"Source-of-truth crosswalk" for the override itself — primary source is the Ghidra anchor; secondary is the V5 doc(s) being overridden. Use language like "overrides `X.md` §Y and `Z.md` §W".
- Cascade: any companion docs (glossary, related chapters, indexes) that carry the same wrong claim get rewritten to match the chapter. The Mercury Q4 closure cascaded into `docs/spec/glossary.md`'s MachineGuard entry.
- Confidence: high on the corrected claim if Ghidra-anchored; mention the inferential reasoning explicitly when the new claim is "inferred from usage" (Q3 — the previous-position-reference reading vs alternate orientation-vector reading was an inference from `PackageAndSendEntityMove` pointer-pass + `pPrevPos` aliasing, not a direct semantic label).

Related: [[reference-v5-mercury-evidence]], [[feedback-section1-evidence]].
