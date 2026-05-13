---
title: How to Read the Bible
chapter_id: spec.meta.how-to-read
status: verified
last_verified: 2026-05-13
verified_by: documentation-writer
audience: anyone reading or citing a bible chapter
type: how-to
---

# How to Read the Bible

You arrived at a bible chapter. This page tells you how to use it — how to read the confidence tags, what the chapter ID is for, when "stale" is still safe to cite, and how to challenge a claim you think is wrong.

The companion docs:

- [`conventions.md`](conventions.md) — the citation format and frontmatter schema you are about to interpret.
- [`glossary.md`](glossary.md) — bible vocabulary (Cell, AoI, BSF_*, propID, methodID, ENABLE_ENTITIES, etc.).
- [`README.md`](README.md) — the master index. Start here if you don't know which chapter you need.

---

## What a chapter is

Each chapter answers one question: *"what does X do, and how do we know?"* The answer is structured as five sections that walk from ground truth (the 2009 SGW binary) out to current Rust:

1. **RE findings** — what the binary does.
2. **Client findings** — what the client expects.
3. **Deprecated server** — what the original server did. By definition correct behavior.
4. **Expected Rust** — what Cimmeria *must* do given 1–3.
5. **Actual Rust** — what Cimmeria *currently* does.

When 5 disagrees with 4, the chapter says so. That gap is either a bug (linked to an issue) or a deliberate divergence (linked to an ADR in `docs/architecture/`). A chapter never lets the gap sit silent.

A section may be missing — marked `N/A — <reason>`. That is honest. A blank section is a bug; report it (see [challenging a claim](#challenging-a-claim) below).

---

## Reading the frontmatter

Every chapter opens with a YAML frontmatter block. The keys you need to scan before trusting a chapter:

| Key | What it tells you |
|---|---|
| `status` | Canon weight. See [status lifecycle](#status-lifecycle) below. |
| `last_verified` | When a human last walked the chapter end-to-end against current evidence. Anything older than ~6 months is at risk of being out of date with the Rust side. |
| `verified_by` | Who signed off. If this is `automated-agent`, only `draft` status is valid — a verified chapter has a human handle. |
| `confidence` | Per-section confidence (`high` / `medium` / `low` / `n/a`). A chapter at `verified` has no section at `low`. |
| `disputed_by` | If non-empty, the chapter is being challenged. Treat the contested sections as not-canon until the issue resolves. |
| `supersedes` | If non-empty, this chapter replaces older chapters (listed by `chapter_id`). The old chapters should be `deprecated`. |

The frontmatter is your at-a-glance trust signal. Read it first, then the body.

---

## Status lifecycle

Each `status` value answers "how should I treat this chapter right now?"

| Status | Treat as... |
|---|---|
| `draft` | Working notes. Not canon. May have open questions in prose. Confidence may include `low`. Lives under `docs/drafts/spec/`, not `docs/spec/`. |
| `verified` | **Canon.** Citable. A human signed off; no section is at `low` confidence. |
| `stale` | Canon-with-caveat. Was verified, but a flagged Rust change happened in the area without re-verification. Trust sections 1–3; double-check sections 4–5 against current code. |
| `disputed` | An open issue contests specific claims. The contested sections are tagged inline. Do not cite the contested parts; the uncontested parts are still fine. |
| `deprecated` | The behavior was removed or replaced. The chapter is retained for history. Useful when reading old code or commit messages; not useful as a current-behavior reference. |

When in doubt: `verified` is canon, everything else needs a "but check..." caveat.

---

## When `stale` is still safe to cite

A chapter goes `stale` when CI detects a PR touched Rust symbols in `evidence_refs.rust` without touching the chapter. That means *one of three things has happened*:

1. The Rust change was cosmetic (rename, file move, type extraction) and the chapter's claims are still correct. Stale is just a "please re-verify" flag.
2. The Rust change was behavioral and the chapter's Section 5 is now wrong. Sections 1–4 are still fine; Section 5 needs an update.
3. The Rust change reflects a behavior fix that means the chapter's Section 4 is also stale — the expected behavior shifted.

Sections 1, 2, 3 are immune to Rust drift because their evidence is immutable (binary, `game/sgw/`, `deprecated/`). **A `stale` chapter is still trustworthy for sections 1–3.** Sections 4 and 5 deserve a quick check against current code before you cite them.

If a `stale` chapter's `last_verified` is more than 6 months old, treat with more caution — even sections 1–3 may be missing context from V5 follow-up work.

---

## Reading confidence tags

Confidence is per-section, not per-chapter. A chapter can have `re: high`, `client: low`, `deprecated: high`, `rust_expected: high`, `rust_actual: medium` and that is fine — it tells you exactly which evidence to trust.

| Confidence | What it means for citing |
|---|---|
| `high` | Direct verification. Cite freely. |
| `medium` | Indirect evidence or single high-quality source without independent cross-check. Cite with awareness; if your decision hinges on this section, verify before committing to it. |
| `low` | Inferred. Pattern-matched, partially decompiled, analogized. Do not cite as definitive. |
| `n/a` | Section does not apply. The chapter explains why inline. |

A chapter at `status: verified` cannot have any section at `low`. If you find one, that is a bug — file an issue.

---

## Citing a chapter

When citing a bible chapter from any other doc, code comment, PR description, or issue, **cite by `chapter_id`**, not by file path:

> Per `spec.combat.damage-pipeline`, the QR result-code table maps codes 0–14 to designer channels.

This survives restructures. File paths under `docs/spec/` will move as the bible grows; `chapter_id`s never change.

For inline links inside other docs:

```markdown
See [`spec.combat.damage-pipeline`](../spec/combat/damage-pipeline.md) for the QR table.
```

Where possible, prefer the `chapter_id` in prose and the path only inside the markdown link target.

---

## Challenging a claim

The bible is canonical, not infallible. Every chapter is open to challenge. The protocol:

### Step 1 — Read the chapter carefully

Specifically:

- Read sections 1, 2, 3 first. The bible's claim is grounded there; if you disagree with section 4 or 5 but the prior sections check out, you are probably disagreeing with the *derivation*, not the evidence.
- Note the `confidence` for the section you doubt. If it is `low`, you are probably right to be skeptical.
- Check `last_verified`. If it is old and the chapter is `stale`, the gap may already be known.

### Step 2 — File an issue

Open a GitHub issue with:

- The `chapter_id` and section number you are contesting.
- Your counter-evidence (Ghidra address, client file:line, deprecated server file:line, or Rust symbol).
- Your proposed correction.

Title format: `bible: dispute spec.X.Y — <one-line summary>`.

### Step 3 — Update the chapter's frontmatter

In the same PR (or a follow-up if the dispute needs discussion first), flip the chapter's `status` to `disputed` and add the issue number to `disputed_by`:

```yaml
status: disputed
disputed_by: [279]
```

Tag the contested sections inline:

```markdown
> [!WARNING] **Disputed by #279.** The claim that BSF_Holster is bit 8 is contradicted by W3 evidence at `ghidra://SGW.exe@0x00ec0840` showing it is only ever set via `entity+0x3D2`.
```

### Step 4 — Resolution

The dispute resolves one of three ways:

1. **Chapter was right** — issue closes, frontmatter flips back to `verified`, inline `> [!WARNING]` tag is removed.
2. **Chapter was wrong, fix is small** — chapter is updated, frontmatter stays `verified` with a new `last_verified` date, inline tag removed.
3. **Chapter was wrong, structurally** — a new chapter supersedes the old. New chapter's `supersedes:` lists the old `chapter_id`; old chapter flips to `deprecated`.

Resolution is the chapter author's responsibility (or anyone else who picks it up). The dispute does not block other work; the chapter is just temporarily not-canon for its contested claims.

---

## What the bible is not

So you know what *not* to expect:

- **Not a tutorial.** If you want to learn the codebase, start with the README and `docs/guides/`. Bible chapters assume you already know what a Cell is.
- **Not an ADR.** Bible chapters say *what the system does* and *how we know*. They do not justify *why we chose X over Y*. ADRs live in `docs/architecture/`.
- **Not a roadmap.** If you want to know what is being built next, see `docs/project-status.md`. The bible is what *is*, not what *will be*.
- **Not a gameplay design doc.** The bible specifies what the 2009 SGW server did. Deliberate Cimmeria deviations are documented as ADRs, with the resulting behavior captured in a bible chapter.

---

## When the bible contradicts another doc

The bible wins. By design, the bible is the canonical source of truth for any behavior it documents; other docs that disagree are either stale or wrong.

If you find a contradiction:

1. **If the bible chapter is `verified`** — the other doc is wrong. File a PR to either delete the contradicting section (preferred — pin a stub redirect to the chapter) or mark it `> [!WARNING] Superseded by spec.X.Y`.
2. **If the bible chapter is `draft` or `stale`** — both might be wrong; the bible is just slightly more likely to be right. Run [the challenge protocol](#challenging-a-claim) on the chapter and let the issue thread sort it out.
3. **If the bible chapter is `disputed`** — neither is canon. Cite the underlying evidence (finding doc, deprecated source, etc.) directly until the dispute resolves.

The point of single-source-of-truth is that *you do not have to choose* between competing docs for canonical behavior. When you find yourself choosing, that is a bug in the docs, not a feature.

---

## TL;DR

- Each chapter answers one question; cite by `chapter_id`.
- Frontmatter tells you status, confidence, when it was last verified, and whether it is disputed. Read it first.
- `verified` is canon. `stale` is canon for sections 1–3, double-check 4–5. `draft` is not canon.
- Confidence is per-section. `low` means do-not-cite-as-definitive.
- To challenge a claim: file an issue, flip the chapter to `disputed`, tag the contested sections inline.
- The bible wins when it contradicts another doc. If you found a contradiction, you found a bug to file.
