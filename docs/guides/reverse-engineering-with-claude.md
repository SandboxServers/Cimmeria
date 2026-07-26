---
type: how-to
audience: Cimmeria contributors using Claude Code (or any LLM agent) to drive reverse-engineering work
last_updated: 2026-07-25
prerequisites: [re-toolchain-setup.md completed and MCPs reachable]
companion_docs:
  - re-toolchain-setup.md
  - sgw-live-debugging.md
  - ../reverse-engineering/evidence-standards.md
  - ../reverse-engineering/PLAN.md
---

# Reverse-Engineering with Claude (and other LLMs)

This is the workflow doc — what to do once Ghidra MCP, x64dbg MCP, and the Cimmeria RAG MCP are wired up per [`re-toolchain-setup.md`](re-toolchain-setup.md). It covers when to invoke the specialist agents the repo ships, what to expect back, what to hand off, and — most importantly — what *not* to delegate.

The Cimmeria RE practice was designed around the [`game-archaeology-specialist`](../../.claude/agents/game-archaeology-specialist.md) agent and the [`documentation-writer`](../../.claude/agents/documentation-writer.md) agent working in partnership: the specialist digs, the writer publishes. Treat both as collaborators — they are not autonomous, and the human contributor (you) is the one who owns the truth of each claim.

## When to invoke the `game-archaeology-specialist` agent

The persona lives at [`.claude/agents/game-archaeology-specialist.md`](../../.claude/agents/game-archaeology-specialist.md). It's the right agent when:

- You have an unknown opcode, method index, or symbol and need to recover its intent from the binary.
- A system's runtime behavior diverges from what the server emits and you need to trace which side is wrong.
- You're about to implement a new game system and want to survey the binary first ("recon before code").
- You found a finding doc whose claim looks load-bearing but unverified — the specialist re-runs the dig against current Ghidra state.

It is the *wrong* agent when:

- You just want to read a doc — use a normal Read or the `cimmeria-rag` MCP.
- The recovery work has already been done and lives under [`docs/reverse-engineering/findings/`](../reverse-engineering/findings/) — cite the finding doc; don't re-dig.
- You need a working PR — the specialist produces evidence, not implementation. Hand the evidence to a domain advisor (combat, mission, inventory, etc.) for the actual code.
- The question is "should we modernize X" — that's an architecture decision, not archaeology. Use `bigworld-engine-advisor` or open an architecture issue.

## Phase mapping — how the Six Phases map to a Claude Code session

The specialist executes every investigation in [six phases](../../.claude/agents/game-archaeology-specialist.md#methodology--the-six-phases). Here's what each phase looks like in practice and what artifacts to expect:

| Phase | What the agent produces | What you do with it |
|---|---|---|
| **1 — Reconnaissance** | A function-address shortlist, call-graph sketch, list of known unknowns | Read it. If the question rests on a wrong assumption (e.g., "this is a server message" when the binary shows it's client-only), course-correct here — it's much cheaper than later. |
| **2 — Intent Reconstruction** | Per-function plain-language intent + cited evidence (addresses, decompiled snippets) | Verify two or three of the cited addresses yourself in Ghidra. The agent's pattern-matching is good but not infallible; a doc with one wrong address taints everything downstream. |
| **3 — Planning** | An ordered implementation plan + flagged 2009-vs-2026 divergences + test strategy | This is the handoff point to a domain advisor (e.g., `combat-systems-advisor` for damage formulas, `mission-systems-advisor` for objective primitives). Don't ask the archaeologist to also implement. |
| **4 — Implementation** | Rust code written to match the recovered spec | Owned by the domain advisor or `rust-gameserver-dev`. The archaeologist may pair on tricky byte-layout questions but the implementation isn't its primary craft. |
| **5 — Verification** | Wire-format / live-DB / unit tests proving the implementation matches the binary | This is non-negotiable for shipped behavior. Per project policy, a regression guard must *fail when the fix is reverted* — see [TESTING.md](../../TESTING.md). |
| **6 — Retrospective** | An evidence packet for the `documentation-writer` agent + proposed updates to indexes/READMEs | Forward the packet. Don't freehand a chapter or finding doc — that's the writer's craft. |

A "good" investigation produces evidence files at `docs/reverse-engineering/findings/<system>.md` and (optionally) an updated address-map entry in [`address-map.md`](../reverse-engineering/address-map.md). It does not produce a bible chapter directly — that comes later via the [bible authoring flow](../spec/how-to-write.md).

## Evidence handoff — what the `documentation-writer` agent expects

When you (or the specialist) recommend invoking the writer, hand it a structured packet:

```markdown
## Evidence Packet — <system name>

### Summary
One paragraph, technical-but-accessible, what was recovered.

### Plain-language explanation
What this system does, for a curious non-engineer.

### Technical detail
Ghidra addresses, decompiled pseudocode (cleaned up), byte layouts, struct
definitions, call graphs. Cite using the project's standard format —
`ghidra://SGW.exe@0xXXXXXXXX` or `<function-name> @ 0xXXXXXXXX`.

### Evidence trail
For each non-trivial claim, the address / function / packet that supports it.
This is what `evidence-standards.md` calls the "corroboration" — at least two
independent sources to claim HIGH confidence.

### 2009-vs-2026 notes
Original intent vs. how Cimmeria implements it today, and why.

### Open questions
What's still unknown. What evidence would close each one.

### Cross-reference targets
Which existing docs need updates. Map them to the "what changed → what to
update" table in CLAUDE.md.
```

The writer turns this into a publication-quality doc following the Diátaxis split (reference / how-to / tutorial / explanation) and the existing `docs/` voice. It will *not* invent claims — it asks for more evidence if a packet has gaps, which is why structured handoffs save round-trips.

## The `/re-verify` gate — mechanical structure-checking

The single highest-value failure mode in LLM-driven RE is a *confident stub*: the agent writes a tidy six-line reconstruction of a function whose binary body is eighty instructions and nine calls. It reads well, it cites a real address, and it is wrong.

> **Availability: not on `main` yet.** As of 2026-07-25 both files this
> section describes — `.claude/commands/re-verify.md` and
> `tools/re_parity.py` — exist only on a feature branch and are not present
> in a fresh `main` checkout. If `/re-verify` isn't offered and
> `python tools/re_parity.py` says "No such file or directory", that's why.
> Delete this note once they land.

There is a deterministic gate for exactly this. The [`/re-verify`](../../.claude/commands/re-verify.md) slash command runs a reverser/checker loop where Claude produces the reconstruction and [`tools/re_parity.py`](../../tools/re_parity.py) — pure Python, no network, no LLM, no API tokens — judges it:

```text
/re-verify <function address|name> [path/to/reconstruction] [--live]
```

The loop is: pull ground truth from Ghidra MCP (decompile, disassembly listing, authoritative callee count from the call graph), write or read the reconstruction, then run the parity engine as the objective gate:

```bash
python tools/re_parity.py \
  --decompile <slug>.dc.c --source <slug>.recon.rs \
  --asm <slug>.asm --callee-count <N> --json
```

The engine reports 11 structural heuristics classified RED (blocking) / YELLOW (inspect) / INFO — `trivial_stub`, `large_asm_tiny_source`, `stub_markers`, and so on — plus a conservative objective verdict of PASS / FAIL / UNKNOWN over call-count and control-flow gaps. Exit code `1` means blocking (any RED, or an objective FAIL). The command caps the revise-and-recheck loop at 4 rounds; `--live` adds an optional post-PASS confirmation via a **non-freezing** x64dbg breakpoint (a freezing BP on the live client makes the server drop the connection).

Two things to hold onto:

- **UNKNOWN is not a pass.** It means the gate lacked reference data — go get more ground truth rather than shipping on it.
- **PASS is not proof.** It means "structurally consistent with the binary," not "semantically correct." The parity engine is advisory; YELLOW signals are for a human. Never edit `re_parity.py` to make one function go green — if a signal misfires on a legitimate idiom, use the documented per-invocation flags (`--wrapper-prefix`, `--call-tol`, `--cf-tol`).

All RE intermediates belong in the session scratchpad, never in the repo.

## What NOT to delegate

This is the most important section. The MCP toolchain is powerful enough to convincingly fabricate things — addresses that look right but point to padding, decompiled pseudocode that pattern-matches the question instead of the binary, byte layouts that "should" be the answer. Some failure modes:

### Verify load-bearing claims yourself

Pre-V5 finding docs under `docs/reverse-engineering/findings/` (and especially the `analysis/` legacy docs) are **hypotheses recorded by earlier sessions** — not all of them are still correct. Examples found in past V5 audits:

- Method indices off by one because an annotation script had a shift bug (`annotation-script-shift-bugs.md` catalogues these).
- Wire formats stated as N bytes that are actually N+M because a custom-alias type was missed.
- Function names pinned to addresses that have since been renamed by a newer annotation pass.

Before you pin a claim from a finding doc into a bible chapter, into the dispatch table, or into a Rust struct, **re-verify it in Ghidra**. If the user has loaded SGW.exe and run the annotation scripts, this is a single `mcp__ghidra__decompile_function` call. Cheap.

### Don't let the agent vote on its own work

If a specialist agent produces a finding and you ask the same agent to review it, you'll get a sycophantic confirmation. Use a *different* agent — `bigworld-engine-advisor` for engine-level claims, `combat-systems-advisor` for damage math, `network-security-auth` for auth flows. Adversarial review across personas is how you catch the convincing-but-wrong claim.

### Never auto-merge specialist output

The archaeology specialist will happily write findings docs. The documentation-writer will happily polish them. Neither is allowed to push commits without you reading the diff. Treat agent output exactly like a junior contributor's PR — review every claim, every citation, every byte.

### Don't outsource intent to the binary alone

The 2009 developers made bugs. Sometimes the binary "specifies" a behavior that was clearly a regression in 2009 and shouldn't be replicated in 2026. Distinguish:

- **The binary as spec.** What the shipped client actually does — this is what the server must match for compatibility.
- **The binary as developer intent.** What the 2009 devs *meant* to ship — sometimes recoverable from comments, naming, or surrounding code.

When they diverge, document both and ask the user which to implement. Common case: a damage formula in the binary that under-flows on negative armor; the original intent was clearly to clamp, the binary doesn't. We replicate the binary's behavior because the client expects it, but we note the gap so future-us doesn't waste a session re-debugging it.

### Don't trust LLM-produced docs as primary evidence

A document produced by an LLM agent is at the same confidence level as the evidence it cites — no higher. If the cite is `ghidra://SGW.exe@0x00c6fc40 and the decompiled function shows X`, the doc is HIGH. If the cite is "the agent inferred from naming patterns", the doc is LOW, no matter how confidently it's written. Use [`evidence-standards.md`](../reverse-engineering/evidence-standards.md) verbatim — agent-produced ≠ verified.

## Confidence-tier interaction with LLM agents

The three tiers from [`evidence-standards.md`](../reverse-engineering/evidence-standards.md) apply unchanged:

- **HIGH** still requires two independent corroborating sources. An agent that says "I verified this in Ghidra and it matches the .def file" is one source (the agent's claim) — you need to verify *both* citations yourself before promoting to HIGH.
- **MEDIUM** is the default for single-source agent output. Most archaeology-specialist findings start MEDIUM and graduate to HIGH after a second agent or a human spot-checks them.
- **LOW** is for inference, pattern-matching across analogous systems, and anything the agent says without citing an address. Use as a hypothesis to test, not as something to implement against.

The agent should *self-rate* its findings — if it produces a packet with no confidence tags, send it back and ask. A packet without tags will get pushed back by reviewers anyway, so catching it before the writer touches it saves a cycle.

## When MCP-driven flows don't work — the manual fallback

The MCP bridges are convenient but not omnipotent. Reach for the manual flow when:

- **pybag (the Ghidra MCP debugger plugin) freezes SGW.** This is a known compatibility issue documented in [`sgw-live-debugging.md`](sgw-live-debugging.md). For dynamic analysis on SGW, use x32dbg manually (or via the x64dbg-automate MCP) — never pybag.
- **You need to halt-break on a hot function.** Hot functions (cursor target tracking, animation ticks) disconnect the client when paused. Log breakpoints are the answer; an agent can drive them via the x64dbg-automate MCP, but you should read the technique in `sgw-live-debugging.md` first so you can spot when the agent picks a hot BP location.
- **The decompile is garbled.** Some functions have control-flow that the decompiler mis-renders. Read the disassembly directly (`mcp__ghidra__disassemble_function`) and walk it yourself; pattern-matching agents struggle with mangled output.
- **The investigation is broader than one or two sessions.** Long-running campaigns (like the V5 campaign that produced 19 findings docs) need human curation across sessions — agent memory helps, but the campaign's shape is yours to maintain.

## Practical constraints — context and rate limits

The MCP tools amplify what a single Claude Code session can do, but the session itself still has a budget:

- **Context window.** Long Ghidra dumps eat context fast. `mcp__ghidra__decompile_function` on a large function can return 5–10 KB; doing that on 20+ functions inside one investigation will push you toward compaction. Mitigations: ask the agent to summarize after each phase and drop the raw decompile before moving on; use `mcp__ghidra__get_function_signature` instead of `decompile_function` when only the shape matters; spawn subagents for parallel recon so each returns a digest instead of raw output.
- **Rate limits.** Each Claude plan has its own per-window quota. A six-phase investigation that bounces between Ghidra MCP and x64dbg MCP many times is closer to "intensive" than "incidental." If you hit a limit mid-investigation, pause, save the evidence packet to `.claude/agent-memory/game-archaeology-specialist/`, and resume in a fresh session.
- **Tool-call cost.** Each MCP call is a tool invocation. Batching helps — `mcp__ghidra__batch_decompile` over 10 addresses is much cheaper than 10 separate decompiles. Same for `batch_set_comments`, `batch_create_labels`.
- **Don't paste decompiled blobs into chat.** If you need to discuss a function with another agent, hand it the address (`ghidra://SGW.exe@0x00c6fc40`) rather than the full decompile. The receiving agent can fetch what it needs.

A practical pattern: scope each session to **one** Six-Phase pass on **one** subsystem. Cross-system investigations stretch context without producing better evidence.

## A typical session shape

A representative half-day archaeology session, scoped to one mid-sized system:

1. **Pre-flight (5 min).** Skim `docs/reverse-engineering/findings/` for existing coverage. Read the relevant `.def` file. Note any address-map entries.
2. **Invoke the specialist (10 min recon).** Ask for Phase 1 + 2 only, scoped to the specific question. Read the returned packet; sanity-check two or three addresses.
3. **Decide: more recon, or plan.** If recon turned up surprises, ask for another phase 1+2 cycle on the surprise. Don't push to phase 3 with shaky evidence.
4. **Planning + handoff (30 min).** Phase 3 produces the plan. Hand to a domain advisor or to yourself for implementation.
5. **Implement (1–3 hours).** This is normal Rust work, guided by the plan.
6. **Verify (30 min).** Wire-format / live-DB / unit tests per [TESTING.md](../../TESTING.md).
7. **Documentation packet (15 min).** Specialist returns a Phase 6 packet. Forward to documentation-writer if the work is publishable; otherwise file under `.claude/agent-memory/game-archaeology-specialist/` for the next dig.

Bad shape: "use the specialist to research and implement and document and verify, then come back when it's done." That conflates phases, hides the verification step, and produces work that nobody can review piece by piece.

## Cross-references

- [.claude/commands/re-verify.md](../../.claude/commands/re-verify.md) — the `/re-verify` reverser/checker loop
- [tools/re_parity.py](../../tools/re_parity.py) — the LLM-free structural parity engine (`--selftest` runs its own fixtures)
- [.claude/agents/game-archaeology-specialist.md](../../.claude/agents/game-archaeology-specialist.md) — the persona
- [.claude/agents/documentation-writer.md](../../.claude/agents/documentation-writer.md) — the publication partner
- [docs/guides/re-toolchain-setup.md](re-toolchain-setup.md) — get the MCPs working in the first place
- [docs/guides/sgw-live-debugging.md](sgw-live-debugging.md) — manual dynamic-analysis techniques, pybag warning
- [docs/guides/reading-decompiled-code.md](reading-decompiled-code.md) — interpret Ghidra output
- [docs/reverse-engineering/evidence-standards.md](../reverse-engineering/evidence-standards.md) — confidence tiers and citation grammar
- [docs/reverse-engineering/PLAN.md](../reverse-engineering/PLAN.md) — campaign-level RE plan
- [docs/reverse-engineering/STATUS.md](../reverse-engineering/STATUS.md) — what's been recovered so far
- [docs/reverse-engineering/findings/](../reverse-engineering/findings/) — the V5 evidence pool
- [docs/spec/how-to-write.md](../spec/how-to-write.md) — how findings become bible chapters
- [TESTING.md](../../TESTING.md) — the regression-guard rules every implementation must follow
