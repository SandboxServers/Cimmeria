---
description: Structurally verify a reverse-engineered function against the binary via Ghidra MCP + the LLM-free parity engine (reverser/checker loop, no API tokens).
argument-hint: <function address|name> [path/to/reconstruction] [--live]
allowed-tools: Bash(python tools/re_parity.py*), Read, Write, Edit, Glob, Grep, mcp__ghidra__*, mcp__x64dbg__*
---

# /re-verify — reverser/checker loop with a deterministic gate

Reconstruct and **structurally verify** a function against the SGW binary,
using your Ghidra MCP for ground truth, Claude (this session, your
subscription) as the reverser/checker, and `tools/re_parity.py` as the
**LLM-free objective gate**. No API token is ever used — the parity engine
is pure Python and runs offline.

This is the adaptation of Dryxio/auto-re-agent with its billed LLM brains
removed: you are the reverser/checker; the parity script is the objective
verifier + parity engine.

## Arguments

- `$1` — target function: an address (e.g. `0x013A96E0`) or a name.
- `$2` — optional path to an existing reconstruction file to verify. If
  omitted, produce the reconstruction yourself in this loop.
- `--live` — after a static PASS, confirm behavior in x64dbg (optional).

## Procedure

### 1. Gather ground truth (Ghidra MCP)

Ensure the Ghidra analysis/decompiler tools are available first
(`mcp__ghidra__list_tool_groups` → `mcp__ghidra__load_tool_group` if the
decompiler group isn't loaded; `mcp__ghidra__list_instances` /
`connect_instance` to attach to the SGW project). Then for the target:

- **Decompile** the function → save the C pseudocode to a scratch file
  `.../scratchpad/re/<slug>.dc.c`.
- **Disassembly listing** → save to `.../scratchpad/re/<slug>.asm` (used
  by the large-asm-tiny-source and asm-call signals; optional but strong).
- **Callee count** — get the authoritative number of distinct callees from
  the call graph / xref data (not just what the decompile text shows). Hold
  it for `--callee-count`.

Use the session scratchpad root
(`C:\Users\Steve\AppData\Local\Temp\claude\...\scratchpad\re\`); create the
`re/` subdir if needed. Never write RE intermediates into the repo.

### 2. Reverse (the "reverser")

If `$2` was given, read it as the candidate. Otherwise write your best
reconstruction (Rust preferred, matching repo idiom; C or annotated
pseudocode acceptable for pure analysis) to
`.../scratchpad/re/<slug>.recon.rs`. Reconstruct the **real** control flow
and every call — do not stub. The whole point of the gate below is to catch
a reconstruction that is simpler than the bytes.

### 3. Objective gate (the "checker" — LLM-free)

Run the parity engine:

```bash
python tools/re_parity.py \
  --decompile ".../scratchpad/re/<slug>.dc.c" \
  --source    ".../scratchpad/re/<slug>.recon.rs" \
  --asm       ".../scratchpad/re/<slug>.asm" \
  --callee-count <N> \
  --json
```

Exit code `1` = **blocking** (any RED signal or an objective FAIL). Exit
`0` = not blocking (PASS or UNKNOWN). Read the `signals`, `objective`, and
`metrics` fields.

Optional flags:
- `--wrapper-prefix <pfx>` (repeatable) — treat calls to a framework/SDK
  wrapper family as wrappers so the trivial-stub / call-heavy signals judge
  the *real* body. Leave unset unless a clear wrapper family exists.
- `--stub-marker <str>` (repeatable) — extra RED markers beyond the defaults.
- `--call-tol` / `--cf-tol` — loosen the objective gaps for genuinely
  divergent-but-correct idiomatic ports.

### 4. Loop (bounded)

If the verdict is **FAIL**, treat each signal as checker feedback and
revise the reconstruction, then re-run step 3. **Cap at 4 rounds** (the
auto-re-agent default). Per round, report which signal you're addressing and
how. If still FAIL after 4 rounds, stop and surface the residual signals —
do not force a green by gaming tolerances or deleting the asm input.

A `UNKNOWN` verdict means the gate lacked reference data (e.g. no decompile
or no callees) — go back to step 1 and get more ground truth rather than
treating UNKNOWN as a pass.

### 5. Live confirmation (`--live`, optional)

After a static **PASS**, if `--live` was passed and x64dbg is attached to
the running client, set a **non-freezing** breakpoint at the function entry
(log + fast-resume — never a freezing BP, or the server disconnects the
client), exercise the code path, and confirm the observed call sequence and
branch taken match the reconstruction. This is the verification step the
original static-only tool cannot do. Report any divergence.

### 6. Report

Summarize: final verdict, the metrics line (source vs binary call/CF/instr
counts), any remaining YELLOW/INFO signals worth a human look, and — if the
reconstruction is now trustworthy — the file path and a one-line note on
what the function does. Recording a confirmed finding into
`docs/reverse-engineering/` or agent memory is a good follow-up, but do it
only when asked or clearly warranted.

## Guardrails

- The parity engine is advisory structure-matching, **not** proof of
  semantic equivalence. A PASS means "structurally consistent with the
  binary," not "provably correct." YELLOW signals are for a human.
- Never edit `tools/re_parity.py` to make a specific function pass. If a
  signal misfires on a legitimate idiom, adjust the documented flags
  (`--wrapper-prefix`, `--call-tol`, `--cf-tol`) for that invocation.
- All RE intermediates stay in the scratchpad, out of git.
