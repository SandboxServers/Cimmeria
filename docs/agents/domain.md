# Domain Docs

How agent skills should find and use this repo's domain knowledge before exploring code.

Skills that default to a root `CONTEXT.md` and a `docs/adr/` directory (the Matt Pocock engineering skills do) must follow the overrides in this file instead. **Do not create `CONTEXT.md`, `CONTEXT-MAP.md`, or `docs/adr/` in this repo.** Both roles are already filled, and a second glossary or a second ADR directory splits the source of truth.

## Where things live

| Role | Location | Notes |
|---|---|---|
| Glossary | [`docs/spec/glossary.md`](../spec/glossary.md) | Use its terms in issue titles, test names, and proposals. Add missing terms here, not in a new file. |
| Architecture decisions (ADRs) | [`docs/architecture/`](../architecture/) | Every doc here is decision-bearing, whether or not it has a `Status` header. New decisions are a new or amended doc in this directory, per the doc-update map in [`CLAUDE.md`](../../CLAUDE.md). |
| Documentation index | [`docs/readme.md`](../readme.md) | Start here to find the doc for a system. Keep it in sync when adding or renaming a doc. |
| Wire protocol (authoritative) | [`docs/protocol/`](../protocol/) | Dispatch tables, message catalog, and [`client-verified-wire-formats.md`](../protocol/client-verified-wire-formats.md). |
| Reverse-engineering findings | [`docs/reverse-engineering/findings/`](../reverse-engineering/findings/) | Address-cited findings. Confidence rules: [`evidence-standards.md`](../reverse-engineering/evidence-standards.md). |
| Game systems | [`docs/gameplay/`](../gameplay/), [`docs/game-systems.md`](../game-systems.md) | Per-system mechanics. |
| Content engine and missions | [`docs/content/`](../content/) | Chains, triggers, interaction flags. |
| Engine internals | [`docs/engine/`](../engine/) | BigWorld, CME, cooked data, UE3 packages. |
| Spec "bible" | [`docs/spec/`](../spec/), drafts in [`docs/drafts/spec/`](../drafts/spec/) | Drafts are work in progress. See "When sources disagree". |
| Canonical entity definitions | `entities/entities.xml`, `entities/defs/*.def` | Source of method and property order. |
| Original server reference | `deprecated/` | Reference for original intent only. Not authoritative for client behaviour. |
| Prior agent findings | `.claude/agent-memory/<agent>/MEMORY.md` | Committed to the repo. Read the index for the agent whose domain you are in. |
| Project rules and gotchas | [`rules-and-gotchas.md`](rules-and-gotchas.md) | Decisions already made and traps already hit. Read before proposing an approach. |

## Before exploring

1. Search `docs/` for the system or keyword. The answer is usually already written down.
2. For a method index or message id, read the dispatch table in `docs/protocol/` before counting entries in a `.def` file. If the entry is missing, add it to the table after you derive it.
3. For client behaviour, check `docs/reverse-engineering/findings/` before opening Ghidra.
4. Read the `docs/architecture/` docs that touch the area you are about to change.
5. If the docs do not cover it, investigate, then document what you found in the same PR.

If a location above has nothing relevant, proceed without comment. Do not suggest creating placeholder docs.

## When sources disagree

Issue text and draft spec chapters are **claims**, not evidence. Before acting on a ticket that says a constant, index, or wire layout is wrong:

1. Check what `docs/protocol/` and the RE findings say about it.
2. If they disagree with the ticket, stop and verify against the binary or the code. Do not pick the most recently edited source.
3. Cite the Ghidra address for the claim you end up relying on, and correct the losing source in the same PR.

Order of trust: the client binary and client files, then captures of real client traffic, then `docs/protocol/` and address-cited findings, then draft chapters, then issue text. `deprecated/` shows what the original server did, which is useful for intent but does not override client evidence.

Worked example: an issue and a draft chapter both said the `Account` entity typeID should be `0x08`, quoting half of one decompiled function. `docs/protocol/client-verified-wire-formats.md` already said `0x07` and explained why. The change would have broken login for every player, and CI was green because the new test compared the constant with itself.

## Use the glossary's vocabulary

When your output names a domain concept (in an issue title, a refactor proposal, a hypothesis, a test name), use the term as defined in `docs/spec/glossary.md`. Do not drift to synonyms.

If the concept you need is not in the glossary, either you are inventing language the project does not use, or there is a real gap. For a real gap, propose the glossary addition in the same PR.

## Flag conflicts with decisions

If your output contradicts a doc under `docs/architecture/`, say so explicitly instead of silently overriding it:

> _Contradicts `docs/architecture/abilities-and-effects-system.md` (refcounted state flags), but worth reopening because…_
