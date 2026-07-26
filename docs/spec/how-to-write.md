---
title: How to Write a Bible Chapter
chapter_id: spec.meta.how-to-write
status: verified
last_verified: 2026-07-25
verified_by: documentation-writer
audience: bible authors
type: how-to
---

# How to Write a Bible Chapter

This is the author's guide. You are about to add a chapter to `docs/spec/`. Read this end-to-end before you open the template — the steps build on each other and skipping one usually means rewriting later.

The companion docs you will keep open while writing:

- [`conventions.md`](conventions.md) — the rulebook (frontmatter, citation format, no-line-numbers rule).
- [`../../.templates/spec-chapter.md`](../../.templates/spec-chapter.md) — the 5-section skeleton you start from.
- [`glossary.md`](glossary.md) — bible vocabulary. If your chapter uses a term not defined there, add it to the glossary in the same PR.
- [`../reverse-engineering/findings/`](../reverse-engineering/findings/) — the V5 evidence pool. Your section 1 cites one or more of these.

---

## The shape of a chapter

Every chapter answers one question: *"what does X do, and how do we know?"* The answer is structured as five sections, each grounding the next.

```text
Section 1 — RE findings              ← what the 2009 binary does
        ↓ grounds
Section 2 — Client findings          ← what the client expects
        ↓ grounds
Section 3 — Deprecated server        ← what the original server did (ground truth #2)
        ↓ grounds
Section 4 — Expected Rust            ← what Cimmeria must do, derived from 1–3
        ↓ compared against
Section 5 — Actual Rust              ← what Cimmeria currently does
```

Section 5 is allowed to differ from Section 4. When it does, that gap is either a bug (file an issue, link it in the chapter) or a deliberate divergence (link the ADR in `docs/architecture/` that justifies it). A silent unexplained gap is the failure the bible exists to prevent.

If a chapter has no section 2 (server-only feature) or no section 5 (not yet implemented), say so explicitly with `N/A — <reason>`. See [conventions.md § N/A marking](conventions.md#na-marking).

---

## Before you write — pick the question

The chapter is one question, not a system survey. Bad chapter titles:

- "Combat" — too broad, eight chapters in a trenchcoat.
- "How combat works" — Diátaxis-wrong, this is reference not explanation.
- "Vendor system overview" — "overview" is a smell; the chapter either covers the system or it doesn't.

Good chapter titles:

- "Damage pipeline" — one question: what is the chain from `useAbility` to `OnEffectResults`?
- "Mercury wire format" — one question: how are bytes on the wire structured?
- "World entry pipeline" — one question: what 8 phases run between connect and `ClientReady`?

If you cannot state the chapter's question in one sentence, you do not yet have a chapter. Go talk to the system advisor for the domain (see [`.claude/agents/`](../../.claude/agents/)) and shape the question first.

---

## Step 1 — claim the chapter ID

Pick a `chapter_id` from the namespaces listed in [`conventions.md § chapter ID grammar`](conventions.md#chapter-id-grammar). Confirm no existing chapter has that ID; if you are unsure, grep:

```bash
grep -r "chapter_id:" docs/spec/
```

If your chapter is the first in a namespace, that is fine — add the namespace to the README's master index when you commit.

The ID is permanent. Pick carefully. If you later realize the topic should split, you publish a new chapter that lists the original in `supersedes:` and flip the original to `deprecated`. You do not rename.

---

## Step 2 — draft in `docs/drafts/spec/`

New chapters land in `docs/drafts/spec/<chapter-id>.md` first, not in `docs/spec/` directly. Draft chapters have:

- `status: draft` in frontmatter.
- Whatever confidence levels honestly apply (often a mix of `medium` and `low`).
- Permission to have an open question or two written into the prose. The whole point of `draft` is "I am still figuring this out."

Copy the template:

```bash
cp .templates/spec-chapter.md docs/drafts/spec/<your-chapter-id>.md
```

Fill in the frontmatter first. The frontmatter forces you to commit to scope before you write the body, which catches "this is actually two chapters" before you wear yourself out drafting both.

---

## Step 3 — write Section 1 from the evidence pool

Section 1 cites one or more V5 finding docs under `docs/reverse-engineering/findings/`. The comment thread on issue [#264](https://github.com/SandboxServers/Cimmeria/issues/264) has the mapping of finding doc → bible chapter; if your chapter has a target finding doc listed there, that is your starting point.

Read the finding doc. Pull the load-bearing claims into Section 1 in your own words — do not copy-paste. The finding doc is dense reverse-engineering; the bible chapter is the *distilled fact* you can build the rest of the chapter on. If you copy-paste, you have made a longer copy of the finding doc and called it a chapter; future readers will not know what is canonical.

For Section 1, line numbers and addresses are valid. Cite `ghidra://SGW.exe@0x...` for binary anchors. Cite finding-doc paths for the body of evidence.

A good Section 1 leaves the reader knowing what bytes get moved, which functions touch them, and what the structural invariants are — without having to open the finding doc.

---

## Step 4 — write Sections 2 and 3

**Section 2 (client)**: file paths + line numbers under `game/sgw/`. Common evidence sources: UE3 `.ini` configs, compiled UnrealScript references, BigWorld entity defs (`Common/res/entities/defs/*.def`), animation maps. Line numbers are valid here — `game/sgw/` is immutable.

**Section 3 (deprecated server)**: file paths + line numbers under `deprecated/`. Common sources: `deprecated/python/base/`, `deprecated/python/cell/`, `deprecated/cpp/src/baseapp/mercury/sgw/`. Line numbers are valid here too — `deprecated/` is immutable.

Section 3 carries special weight. What the deprecated server did is by definition *correct behavior* (modulo bugs flagged in commit history). If section 4 ever diverges from section 3, that divergence is on you to justify in the section 4 prose: "Original server did X. Cimmeria does X' because [reason]."

If your chapter is server-only, mark Section 2 `N/A — server-only feature, no client-side presence`. If your chapter covers behavior the deprecated server never had (e.g. a Cimmeria-specific admin tool), mark Section 3 `N/A — Cimmeria-specific, no deprecated-server precedent`.

---

## Step 5 — write Section 4: expected Rust

Section 4 is where you derive what Cimmeria *should* do from sections 1–3. This is the load-bearing reasoning section. The reader should be able to read Section 4 in isolation and predict what `crates/` modules and types will appear in Section 5.

**No line numbers.** Cite by symbol path: `crates/services/src/cell/combat/threat.rs::ThreatList::add`. See [conventions.md § the no-line-numbers rule](conventions.md#the-no-line-numbers-rule-for-sections-4-and-5) for why.

Section 4 calls out divergences from Section 3 explicitly:

> The deprecated server stored cooldowns in a per-ability dict on the player entity (`deprecated/python/AbilityManager.py:847`). Cimmeria stores them in a flat `BTreeMap<AbilityId, CooldownState>` on the cell-side combat state because Rust ownership rules push us toward one cooldowns owner per cell-tick. The wire-format behavior is unchanged; only the in-memory representation differs.

A divergence without a stated reason is a bug. A divergence with a stated reason is an architectural decision — note it here and, if substantive, file a matching ADR under `docs/architecture/`.

---

## Step 6 — write Section 5: actual Rust

Section 5 documents what the code *currently* does. Open `cargo doc` JSON output, walk the modules, name the types. The no-line-numbers rule applies here exactly as it does in Section 4.

Section 5 is the gap-anchor. When Section 5 disagrees with Section 4, that is either:

1. **A bug.** File an issue, link it from Section 5, leave the chapter's `status: verified` if the bug is small/cosmetic, flip to `disputed` if the gap is structural.
2. **A deliberate divergence the chapter hasn't caught up to yet.** Update Section 4 with the new rationale.
3. **A deliberate divergence justified elsewhere.** Link the ADR in `docs/architecture/`, and Section 4 picks up the rationale at next edit.

Confidence in Section 5 is usually `medium` for any chapter older than three months. Rust changes fast enough that "is this still true?" needs re-verification on a regular cadence — see [status lifecycle](#status-lifecycle).

---

## Status lifecycle

```text
draft  ──promote──▶  verified  ──code-changes──▶  stale  ──re-verify──▶  verified
                          │                                                  ▲
                          └──contested──▶  disputed  ──resolve──────────────┘
                                                                             │
                                                          deprecated  ◀──────┘
```

- **`draft`** — lives in `docs/drafts/spec/`. Not linked from the master index. Open questions are allowed in prose. Confidence may include `low`.
- **`verified`** — lives in `docs/spec/`. Linked from the index. Citable as canon. No section at confidence `low`. A human (not just an agent) signed off via `verified_by`.
- **`stale`** — was `verified`, but a flagged change to the related Rust code happened without re-verification. Canon-with-caveat; the reader sees the `stale` tag and knows to double-check section 5 against current code.
- **`disputed`** — an issue contests the chapter's claims. The contested sections show the `disputed_by` issue link inline. Treat as not-canon until resolved.
- **`deprecated`** — the behavior was removed or replaced. Chapter retained for history (so future readers can understand a code archeology dig). Linked from the index in a dedicated "deprecated chapters" section.

### Promotion gate: `draft` → `verified`

To promote, the chapter must pass all of:

- All five sections are present, or explicitly marked `N/A — <reason>`.
- No section's confidence is `low`. (Upgrade evidence, mark `N/A` with reason, or stay `draft`.)
- All `evidence_refs.rust` entries resolve to existing symbols. [`tools/spec-lint`](../../tools/spec-lint/) checks this for you and runs in CI via [`.github/workflows/spec-lint.yml`](../../.github/workflows/spec-lint.yml), emitting inline PR annotations. It is **warn-only** — it always exits 0 — so a failure to resolve will not block your merge, but a reviewer will see it. Run it locally with `cargo run -p cimmeria-spec-lint` (the crate is `cimmeria-spec-lint`; the binary it produces is `spec-lint`).
- A human (not an agent) reviewed the chapter and put their handle in `verified_by`.
- The chapter is cross-linked from `docs/spec/README.md`'s master index.
- The chapter is cross-linked from any related-chapter's `related_chapters:` frontmatter.

### Trigger: `verified` → `stale`

Triggered when a PR touches paths the chapter cites in `evidence_refs.rust` (the symbols, transitively) without touching the chapter. CI flags via the `spec-touch.yml` workflow. The author of the next PR in the area either updates the chapter or flips its status to `stale` for someone else to pick up.

### Trigger: `verified` → `disputed`

Triggered when someone files an issue contesting a specific claim. Edit the chapter's frontmatter: `status: disputed`, `disputed_by: [<issue-number>]`. Inline-tag the contested sections:

```markdown
> [!WARNING] **Disputed by #279.** The claim that BSF_Holster is bit 8 is contradicted by W3 evidence showing it is only ever set via `entity+0x3D2`, not the `+0x158` flag byte.
```

Resolution flips back to `verified` (claim was right, issue closed) or to a new chapter (claim was wrong, supersede the old one).

---

## What makes a section land

A section "lands" when a future reader can rely on it without going back to the evidence. Concretely:

| Section | Lands when... |
|---|---|
| 1 — RE | Reader knows the bytes, the functions, the structural invariants, without opening the finding doc. |
| 2 — Client | Reader knows what the client sends/expects without grepping `game/sgw/`. |
| 3 — Deprecated | Reader knows the original behavior without reading the legacy Python. |
| 4 — Expected | Reader can predict Section 5's module layout and method names. |
| 5 — Actual | Reader can open the codebase and find the cited symbols without a search. |

If a section requires the reader to re-do your work to use it, it has not landed.

---

## Migration by resynthesis

Some chapters replace prior docs under `docs/` (e.g. `docs/gameplay/combat-system.md` → `spec.combat.damage-pipeline`). The migration rule is **author from evidence, not from the prior doc**:

1. Read the existing doc as a source of *starting hypotheses*. Note its claims as questions to answer.
2. Author the new chapter fresh from the 5 evidence sources. Do not copy-paste.
3. Cross-check your draft against the existing doc's claims. Where they agree, the new chapter inherits the claim (now with proper evidence chain). Where they disagree, the new chapter is authoritative; note the disagreement in your PR description so the project owner can audit the type of error that crept in.
4. Once the new chapter is `verified`, delete the existing doc (or shrink it to a stub redirect). Update cross-references to target the `chapter_id`.

The owner has explicitly authorized deletion of resynthesized prior docs. Do not preserve them for nostalgia.

---

## Common authoring mistakes

Patterns the project owner and reviewers have flagged in early drafts. Avoid them.

| Mistake | What it looks like | Fix |
|---|---|---|
| Two-chapters-in-one | Title is "Combat and damage and threat". Sections 4/5 sprawl across three modules. | Split. Pick the smaller question first. |
| Copy-paste from finding doc | Section 1 reads identically to `docs/reverse-engineering/findings/X.md`. | Section 1 is *distilled fact*. The finding is the evidence; the chapter is what we learned. |
| Line numbers in section 4/5 | "The threat update is at `threat.rs:147`". | Symbol-only: `ThreatList::update_after_damage`. |
| Section 5 disagrees with 4 with no note | Reader can't tell whether the gap is a bug, a TODO, or a deliberate divergence. | Note it. Link the issue or ADR. |
| `status: verified` with `confidence: low` in some section | Canonization of a guess. | Either upgrade the evidence or mark `draft`. |
| "Overview" or "Summary" subsection | Adds words without adding signal. | Cut. The chapter is the overview. |
| Citation as a path, not a `chapter_id` | "See `docs/spec/protocol/mercury-wire-format.md`". | "See `spec.protocol.mercury-wire-format`." Paths break on restructure; IDs don't. |
| Frontmatter blanks | `disputed_by:` with nothing after it. | Use `[]`. Schema must always parse. |
| Apology-style N/A | `N/A — didn't have time`. | That's a `draft`, not an N/A. N/A is for structurally-not-applicable. |
| Adding a chapter without index update | New file in `docs/spec/` but `README.md` doesn't list it. | Update the index in the same commit. |

---

## When to ask for help

The system advisors in `.claude/agents/` exist to be consulted. Each one owns a domain:

- `combat-systems-advisor` — abilities, weapons, effects, damage, threat.
- `mission-systems-advisor` — mission lifecycle, objectives, task primitives.
- `npc-ai-spawn-advisor` — NPC AI FSM, spawn regions, pathfinding.
- `minigame-systems-advisor` — lockpick, hacking, fishing.
- `bigworld-engine-advisor` — Cell/Base split, AoI, CME event signal, RPC dispatch.
- `network-security-auth` — Mercury, AES/HMAC, login handshake.
- `database-persistence` — schema, live-DB tests.
- `rust-gameserver-dev` — Rust patterns inside `crates/`.

For Section 1 evidence questions, talk to `bigworld-engine-advisor` (engine) or `game-archaeology-specialist` (gameplay reverse engineering). For Sections 4/5 questions, talk to `rust-gameserver-dev` or the domain advisor.

Read the advisor's persona file first. If the question is framing or glossary, that is enough — no need to spawn a sub-agent. If the question requires running Ghidra or grepping `crates/` at scale, then spawn.
