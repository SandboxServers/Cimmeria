<!--
  Cimmeria Bible chapter template.

  Before you fill this in:
    1. Read docs/spec/how-to-write.md end-to-end.
    2. Read docs/spec/conventions.md for the frontmatter schema, citation
       format, and the no-line-numbers rule for sections 4 and 5.
    3. Pick a chapter_id from the namespaces in conventions.md. The ID is
       permanent — renaming means publishing a new chapter that lists the
       old ID in `supersedes:` and flipping the old chapter to `deprecated`.

  Drop this file at docs/drafts/spec/<your-chapter-id>.md, replace every
  bracketed placeholder, and remove HTML comments before promoting to
  `verified`. Comments are not allowed in final chapters.

  Naming on disk: the filename does NOT have to match the chapter_id
  segment-for-segment. The chapter_id is the citation key; the filename
  is just where it lives in the tree. Pick a filename humans can read.
-->
---
title: [Chapter title, plain English, title case]
chapter_id: spec.[system].[topic-kebab-case]
status: draft
last_verified: [YYYY-MM-DD when authored or last walked end-to-end]
verified_by: [your handle, or "automated-agent" — only valid for draft]
confidence:
  re: [high | medium | low | n/a]
  client: [high | medium | low | n/a]
  deprecated: [high | medium | low | n/a]
  rust_expected: [high | medium | low | n/a]
  rust_actual: [high | medium | low | n/a]
evidence_refs:
  re:
    # Finding doc(s) under docs/reverse-engineering/findings/ and/or Ghidra
    # anchors. No line numbers on finding docs (they refactor); addresses
    # are fine.
    - docs/reverse-engineering/findings/[matching-finding].md
    # - ghidra://SGW.exe@0x[address]
  client:
    # Repo-relative paths under game/sgw/, line numbers welcome (immutable tree).
    # - game/sgw/Working/SGWGame/Config/[file].ini:[line]
  deprecated:
    # Repo-relative paths under deprecated/, line numbers welcome (immutable tree).
    # - deprecated/python/[file].py:[line]
  rust:
    # NO LINE NUMBERS. Symbol-only: crate::module::Type::method.
    # - cimmeria-services::cell::[module]::[Type]::[method]
related_chapters:
  # Other chapter_ids this chapter cross-references. Empty list is valid
  # for the first chapter in a new area.
  # - spec.[system].[topic]
disputed_by: []
supersedes: []
---

# [Chapter title — matches the title in frontmatter]

<!--
  Opening paragraph: ONE sentence stating the question this chapter answers,
  plus 2–3 sentences of context for a reader who landed here cold. No
  "this document covers..." framing — just answer the question.

  Bad opening:
    "This chapter documents the damage pipeline in Cimmeria."

  Good opening:
    "When a player uses an ability, the result resolves through a fixed
    chain: client emits useAbility → server validates → effect runs →
    OnEffectResults dispatches per-target outcomes. This chapter walks the
    chain and pins each step to its evidence."
-->

[One-paragraph chapter intro.]

---

## Section 1 — RE findings

<!--
  What the SGW binary does.

  Source material: V5 finding docs under docs/reverse-engineering/findings/,
  Ghidra anchors (ghidra://SGW.exe@<address>), the address map.

  Line numbers are addresses here — they're stable. Use them freely.

  Goal: a reader closes this section knowing what bytes get moved, which
  functions touch them, and what the structural invariants are, WITHOUT
  having to open the finding doc.

  Distill the finding doc into facts. Do not copy-paste — if you find
  yourself reproducing a long block from the finding doc verbatim, you
  have written a longer copy of the finding, not a chapter.
-->

[Body of section 1.]

---

## Section 2 — Client findings

<!--
  What the client expects.

  Source material: game/sgw/. UE3 .ini configs, compiled UnrealScript
  references, BigWorld entity defs (Common/res/entities/defs/*.def),
  animation maps, slash command tables.

  Line numbers are valid here (immutable tree). Cite as path:line or
  path:start-end.

  If your chapter is server-only:
    `N/A — server-only feature, no client-side presence. <one-sentence reason>.`
-->

[Body of section 2.]

---

## Section 3 — Deprecated server

<!--
  What the original server did. By definition correct behavior, modulo
  bugs flagged in commit history.

  Source material: deprecated/. python/base/, python/cell/, cpp/src/baseapp/.

  Line numbers are valid here (immutable tree).

  Section 3 carries special weight. Sections 4 and 5 derive from this —
  if section 4 ever diverges from section 3, that divergence is on you
  to justify in section 4's prose.

  If your chapter covers Cimmeria-specific behavior:
    `N/A — Cimmeria-specific, no deprecated-server precedent. <reason>.`
-->

[Body of section 3.]

---

## Section 4 — Expected implementation in Rust

<!--
  What Cimmeria MUST do, given sections 1–3.

  NO LINE NUMBERS. Cite symbols: crates/services/src/cell/combat/threat.rs::ThreatList::add.
  See docs/spec/conventions.md § "the no-line-numbers rule for sections 4 and 5".

  This is the load-bearing reasoning section. A reader should be able to
  read section 4 in isolation and predict what modules and types will
  appear in section 5.

  Call out divergences from section 3 explicitly:

    The deprecated server stored cooldowns in a per-ability dict on the
    player entity (deprecated/python/AbilityManager.py:847). Cimmeria
    stores them in a flat BTreeMap<AbilityId, CooldownState> on the
    cell-side combat state because Rust ownership rules push us toward
    one cooldowns owner per cell-tick. Wire behavior is unchanged.

  A divergence without a stated reason is a bug. A divergence with a
  stated reason is an architectural decision — link any matching ADR
  under docs/architecture/.
-->

[Body of section 4.]

---

## Section 5 — Actual implementation in Rust

<!--
  What Cimmeria CURRENTLY does.

  NO LINE NUMBERS. Same rule as section 4. Cite symbols.

  This is the gap-anchor. When section 5 disagrees with section 4, that
  is one of:
    1. A bug. File an issue, link it from section 5.
    2. A deliberate divergence section 4 hasn't caught up to yet. Update
       section 4 with the new rationale.
    3. A divergence justified by an ADR under docs/architecture/. Link the
       ADR; section 4 picks it up at next edit.

  Confidence in section 5 is usually `medium` for any chapter older than
  three months — Rust changes fast. Re-verify on promotion-to-verified
  and on stale-flip recovery.

  If not yet implemented:
    `N/A — not yet implemented; gap tracked in #<issue>.`
-->

[Body of section 5.]
