---
title: Bible Conventions
chapter_id: spec.meta.conventions
status: verified
last_verified: 2026-07-25
verified_by: documentation-writer
audience: bible authors and reviewers
type: reference
---

# Bible Conventions

This is the rulebook for writing bible chapters. Citation format, frontmatter schema, the no-line-numbers rule, confidence tags, N/A marking, evidence-ref grammar. If you are authoring a chapter, read [how-to-write.md](how-to-write.md) first; this doc is the leaf reference that page calls into.

Every rule here exists because we have seen the failure it prevents. The rationale for the bible structure is in issue [#264](https://github.com/SandboxServers/Cimmeria/issues/264); the rationale for each rule below is inline.

---

## Frontmatter schema

Every chapter starts with a YAML frontmatter block. Required keys are non-negotiable; optional keys may be omitted but never blanked.

```yaml
---
title: Damage Pipeline
chapter_id: spec.combat.damage-pipeline
status: verified
last_verified: 2026-05-13
verified_by: <handle or "automated-agent">
confidence:
  re: high
  client: medium
  deprecated: high
  rust_expected: high
  rust_actual: medium
evidence_refs:
  re:
    - docs/reverse-engineering/findings/combat-damage-analysis.md
    - ghidra://SGW.exe@0x00c79120
  client:
    - game/sgw/Working/SGWGame/Config/DefaultGame.ini:447
  deprecated:
    - deprecated/python/AbilityManager.py:847
related_chapters:
  - spec.combat.ability-resolution
  - spec.protocol.combat-wire-formats
disputed_by: []
supersedes: []
---
```

### Required keys

| Key | Type | Notes |
|---|---|---|
| `title` | string | Plain English. Title case. Matches the H1 immediately below. |
| `chapter_id` | string | The citation key. Stable forever — see [chapter ID grammar](#chapter-id-grammar) below. |
| `status` | enum | One of `draft`, `verified`, `stale`, `disputed`, `deprecated`. See [status lifecycle](how-to-write.md#status-lifecycle) in `how-to-write.md`. |
| `last_verified` | ISO date | When the chapter was last walked end-to-end against current Rust and current evidence. Updated on promotion to `verified`. |
| `verified_by` | string | Handle of the human who signed off, or `automated-agent` if an agent drafted and no human has reviewed. The latter is only valid for `draft`. |
| `confidence` | object | Five sub-keys, one per section (`re`, `client`, `deprecated`, `rust_expected`, `rust_actual`). Each value is one of `high`, `medium`, `low`, or `n/a`. The `n/a` value is required when the corresponding body section is marked `N/A — <reason>`; see [N/A marking](#na-marking). |
| `evidence_refs` | object | Up to four sub-keys (`re`, `client`, `deprecated`, `rust`). Each is a list. See [evidence-ref grammar](#evidence-ref-grammar) below. |

### Optional keys

| Key | When to set |
|---|---|
| `related_chapters` | Always set when the chapter has neighbors. List `chapter_id`s, not paths. |
| `disputed_by` | Set when an open issue contests the chapter's claims. List issue numbers (e.g. `- 266`). Empty list `[]` is valid; leaving the key out is fine when no chapter has ever been disputed. |
| `supersedes` | Set when a chapter replaces a prior one during migration. List `chapter_id`s of replaced chapters, not paths of deleted docs. |

### What "blanked" means and why it is forbidden

`status:` with nothing after it is invalid. `confidence: {}` is invalid. If a value does not yet exist, the chapter is `draft` and the key carries a placeholder (`tbd`, `[]`, `low`). The schema must always parse; the symbol-resolution lint ([`tools/spec-lint`](../../tools/spec-lint/), warn-only today) flags otherwise-empty frontmatter.

---

## Chapter ID grammar

A `chapter_id` is `spec.<system>.<topic>`, all lowercase, kebab-case for multi-word topics. The system namespace is one of:

| Namespace | Scope |
|---|---|
| `spec.meta` | Bible apparatus itself (this file, glossary, how-to-{read,write}). |
| `spec.engine` | BigWorld/CME engine internals. Cell/Base split, AoI, ghost entities, RPC dispatch, cooked data, CME event signal. |
| `spec.protocol` | Mercury wire format, entity property sync, message catalog, auth handshake. |
| `spec.world` | World entry, space management, zone load. |
| `spec.player` | Spawn, death-respawn, animations, state fields, faction/alignment, character creation. |
| `spec.combat` | Ability resolution, weapons, ammo, effects, damage pipeline, threat, loot generation. |
| `spec.inventory` | Containers, equip flows, bandolier. |
| `spec.missions` | Mission lifecycle, objectives, task primitives. |
| `spec.npcs` | Spawn system, movement and pathfinding, cover behavior. |
| `spec.crafting` | Craft, research, reverse-engineer, alloy. |
| `spec.gate-travel` | DHD, stargate, ring transport. |

Add a namespace only when at least two chapters need it. Floating one-off chapters cluster under the closest existing namespace; a `chapter_id` is not a filesystem path and may live in any subdirectory the index points to.

Once a `chapter_id` is published it never changes. Renaming a chapter means publishing a new chapter that lists the old `chapter_id` in `supersedes:` and a redirect note in the old chapter's body. The old chapter then flips to `status: deprecated`.

---

## Evidence-ref grammar

`evidence_refs:` is the load-bearing field for the CI symbol-resolution lint. Each sub-key holds a list; each list entry is a single citation in one of the allowed forms below.

### `re:` — reverse engineering

Two forms accepted.

**Finding doc:** repo-relative path to a doc under `docs/reverse-engineering/findings/`. No line number; findings docs are too dense for line citations to survive an edit.

```yaml
re:
  - docs/reverse-engineering/findings/combat-damage-analysis.md
  - docs/reverse-engineering/findings/cme-event-signal.md
```

**Ghidra anchor:** `ghidra://SGW.exe@<address>` where address is the canonical hex form (`0x` prefix, lowercase, no padding to 8 chars).

```yaml
re:
  - ghidra://SGW.exe@0x00c79120
  - ghidra://SGW.exe@0x015841d0
```

You may cite both — a finding doc points to the body of evidence, a Ghidra anchor points to the specific instruction that grounds a claim.

### `client:` — client evidence

Repo-relative path under `game/sgw/`. Line numbers are valid here because `game/sgw/` is the immutable 2009 client tree.

```yaml
client:
  - game/sgw/Working/SGWGame/Config/DefaultGame.ini:447
  - game/sgw/Common/res/entities/defs/SGWPlayer.def:120-145
```

Ranges are written `start-end`. Single-line citations are bare integers.

### `deprecated:` — deprecated-server evidence

Repo-relative path under `deprecated/`. Line numbers valid for the same reason — `deprecated/` is immutable by definition.

```yaml
deprecated:
  - deprecated/python/AbilityManager.py:847
  - deprecated/cpp/src/baseapp/mercury/sgw/SGWPlayer.cpp:1200-1280
```

### `rust:` — Rust expected/actual evidence

**No paths. No line numbers.** Cite by `crate::module::Type::method`. This is the no-line-numbers rule, canonized — see [why](#the-no-line-numbers-rule-for-sections-4-and-5).

```yaml
rust:
  - cimmeria-services::cell::combat::threat::ThreatList::add
  - cimmeria-mercury::wire::bundle::Bundle::encode
```

Crate names use the published crate name (the `name =` field in `Cargo.toml`), not the directory under `crates/`. So it is `cimmeria-services`, not `services`.

---

## The no-line-numbers rule for sections 4 and 5

**Sections 4 (expected Rust) and 5 (actual Rust) must never cite line numbers.** Not in body text, not in `evidence_refs`, not in inline backticks. Always cite by symbol: `crates/services/src/cell/combat/threat.rs::ThreatList::add` in prose, `cimmeria-services::cell::combat::threat::ThreatList::add` in frontmatter.

**Why:** Line numbers in Rust source rot on every refactor that moves a function around — and the Rust tree refactors often. A chapter that says "the cooldown check is at `threat.rs:147`" is wrong the day after the next `cargo fmt` adds an import. A chapter that says "the cooldown check is at `ThreatList::can_apply`" stays correct until the *symbol* moves, and if the symbol moves, that is a real semantic change that warrants a chapter revision anyway. The rule converts noise (line drift) into signal (semantic drift).

**Applies to:**

- Body prose in sections 4 and 5.
- `evidence_refs.rust` entries.
- Cross-references to other Rust code inside any bible chapter.

**Does not apply to:**

- Sections 1, 2, 3. Ghidra addresses are line numbers and they are stable. Client and deprecated paths are in immutable trees, so line numbers there are valid.
- Citations of test files when the test is fixed by name (e.g. `cimmeria-services::test_support::cell_combat_threat_tests::leashing_clears_threat` — that is a symbol, not a line).
- Doc comments inside Rust source. The rule is about chapters pointing at code, not code pointing at chapters.

If you find yourself wanting a line number in section 4 or 5, what you actually want is a smaller symbol. Split the function, name the helper, then cite the helper.

---

## Confidence tagging

Every section carries a confidence level: `high`, `medium`, or `low`. Values map to the same semantics as `docs/reverse-engineering/STATUS.md`:

| Level | Means |
|---|---|
| `high` | Directly verified. Decompiled and cross-referenced, or pulled from working production code with a regression test pinning it. |
| `medium` | Strong indirect evidence. Multiple consistent string references, RTTI matches, behavioral correlation, or a single high-quality source without independent cross-check. |
| `low` | Inferred. Naming patterns, partial decompilation, analogy with similar systems, or a claim that has not been re-checked since the last code change in the area. |

Confidence is a property of the *section*, not the chapter. A chapter can have `re: high` + `client: low` + `deprecated: high` + `rust_expected: high` + `rust_actual: medium` and that is fine — it tells the reader exactly which evidence to trust.

A chapter that has any section at `low` cannot be `verified`. It is `draft` until the low-confidence section is upgraded or explicitly marked `N/A`.

---

## N/A marking

Not every chapter has all five sections. Mark a missing section `N/A — <reason>`, never blank.

```markdown
## Section 2 — Client findings

N/A — server-only feature, no client-side presence. The client never observes vendor inventory rotation; it sees only the resulting `LootDisplay` wire on purchase.
```

```yaml
confidence:
  client: n/a
```

`n/a` reasons should be short and structural ("server-only feature", "not yet implemented; gap tracked in #279"). They should not be apologies ("we didn't have time to check") — that is a `draft` chapter, not an `N/A` section.

Empty sections are dishonest. Explicitly-marked-N/A sections are evidence of due diligence and let the reader know the absence was considered.

---

## Citation format (in-body)

When a chapter references another chapter, cite the `chapter_id`, not the path.

> The Mercury bundle layout for this RPC is canonized in `spec.protocol.mercury-wire-format`.

When a chapter references a finding doc, cite the path.

> See [`combat-damage-analysis.md`](../reverse-engineering/findings/combat-damage-analysis.md) for the QR result-code table.

When a chapter references a Rust symbol, cite the full path-qualified symbol with `::` separators.

> The hit roll is computed by `crates/services/src/cell/combat/hit.rs::HitRoll::compute`.

When a chapter references a Ghidra address, use the `ghidra://` scheme in inline code.

> The universal RPC dispatcher entry point is `ghidra://SGW.exe@0x00c6fc40`.

---

## Voice and form

The bible is a working reference. It is not a textbook, not a marketing doc, not an autobiography.

- **Second person, present tense, active voice.** "The dispatcher routes the call to..." not "the call will be routed by the dispatcher to...".
- **No filler subsections.** No "Conclusion", no "Summary", no "Overview" unless the chapter is genuinely 1500+ lines and a reader needs a TOC. Most chapters don't.
- **No emojis. No exclamation marks.** Match the rest of the repo.
- **Inline code for paths and identifiers.** Backtick file paths, `crates/...::Type::method` symbols, `chapter_id` strings, addresses (`0x00c79120`), and any wire-format byte name.
- **Lists for inventories. Prose for arguments.** A list of fields is a table or a bulleted list. A "why we chose this layout" paragraph is prose.

Citing a PR or issue is fine in the bible — PRs are the canonical record of a decision. (Inverse to the source-comment rule: `crates/` source must not cite PR/issue numbers; bible chapters may.)

---

## The chapter is one cohesive document

Each chapter answers a single question: *"what does X do, and how do we know?"* If you find your chapter answering two questions, split it. If you find two chapters answering the same question, merge them and put the older `chapter_id` in `supersedes:`.

**No length cap on bible chapters.** The repo's general file-org rule (soft 500 / hard 700) is for source code, where reviewability and LLM context window matter. Bible chapters are reference docs — readers want to understand a system without chasing five cross-references. Depth beats brevity. A 1,500-line chapter that fully canonizes a wire format is more useful than three short chapters with overlapping claims. If a chapter answers two questions, that is still a split signal; length alone is not.

---

## Rules that exist to prevent specific failures

| Rule | Failure it prevents |
|---|---|
| No line numbers in sections 4/5 | A chapter that drifts from current Rust on every `cargo fmt`. The reader has no way to know whether a line citation is stale. |
| `N/A — <reason>` not blank | A chapter that looks complete but actually skipped a section. The reader can't tell whether absence is "considered and not applicable" or "forgot". |
| `chapter_id` is the citation key | A cross-reference that breaks the moment a chapter moves directory. We restructure `docs/spec/` periodically; the IDs survive. |
| `evidence_refs.rust` is symbol-only | The same as the no-line-numbers rule, with teeth — the CI lint enforces it. |
| Status cannot be `verified` with any section at `low` | Canonization of a guess. If you don't know, say `draft` and let the reader weigh accordingly. |
| Frontmatter never blanks | Tooling assumes the schema parses. A half-set frontmatter is worse than a fully-`tbd` one because automated checks pass it. |

Each row above maps to a real authoring failure observed in the existing 188-doc tree under `docs/`. The bible exists to not repeat them.
