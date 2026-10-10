# Re-verify: structurally check a reconstruction against the binary

A reverser/checker loop with a deterministic gate. You (the agent) are the
reverser and the checker. Ghidra is the ground truth, and
[tools/re_parity.py](../../../tools/re_parity.py) is the **LLM-free
objective gate**: pure Python, offline, no API calls. It is a port of the
deterministic layer of Dryxio/auto-re-agent.

Inputs: a target function (address such as `0x013A96E0`, or a name), an
optional existing reconstruction to check, and optionally a request for live
confirmation.

## 1. Ground truth

Using the Ghidra MCP (or headless `Probe.java`, see [SKILL.md](SKILL.md)):

- Decompile the function and save it to `<scratchpad>/re/<slug>.dc.c`.
- Save the disassembly listing to `<scratchpad>/re/<slug>.asm` (headless:
  `IF:<addr>`). It's optional but feeds the strongest signals.
- Get the authoritative number of distinct callees from the call graph or
  xrefs, not from the decompile text. Use it for `--callee-count`.

RE intermediates stay in the session scratchpad, never in the repo.

## 2. Reverse

If a reconstruction was given, it is the candidate. Otherwise write one to
`<scratchpad>/re/<slug>.recon.rs` (Rust in repo idiom preferred; C or
annotated pseudocode is fine for pure analysis). Reproduce the **real**
control flow and every call, with no stubs. The gate exists to catch a
reconstruction that is simpler than the bytes.

## 3. Gate

```powershell
python tools/re_parity.py `
  --decompile <scratchpad>/re/<slug>.dc.c `
  --source    <scratchpad>/re/<slug>.recon.rs `
  --asm       <scratchpad>/re/<slug>.asm `
  --callee-count <N> --json
```

- Exit `1` = blocking: any RED signal or an objective FAIL. Exit `0` = PASS
  or UNKNOWN. Read `signals`, `objective` and `metrics`.
- `UNKNOWN` means the gate lacked reference data (no decompile, no callees).
  Go back to step 1; it is not a pass.
- Optional flags: `--wrapper-prefix <pfx>` (repeatable; treat a wrapper
  family's calls as wrappers so the real body is judged), `--stub-marker <s>`
  (repeatable; extra RED markers), `--call-tol N` (default 3) and
  `--cf-tol N` (default 2) for genuinely divergent but correct idiomatic
  ports.
- `python tools/re_parity.py --selftest` checks the engine itself.

## 4. Loop (at most 4 rounds)

On FAIL, treat each signal as checker feedback, revise, and re-run step 3.
Each round, say which signal you addressed and how. After 4 failed rounds,
stop and report the residual signals. Never force green by raising
tolerances, dropping `--asm`, or editing `re_parity.py` for one function.

## 5. Live confirmation (optional)

After a static PASS, with x64dbg attached to the running client: put a
**logging, non-pausing** breakpoint on the entry (condition `0`, log
condition `1`, a log string with `{expr}` captures, fast resume **off**),
exercise the path, and confirm the call sequence and branch match the
reconstruction. A pausing breakpoint disconnects the client.

## 6. Report

Give the verdict, the metrics line (source vs binary call, control-flow and
instruction counts), any YELLOW/INFO signals worth a human look, and, if the
reconstruction is trustworthy, its path and one line on what the function
does. A PASS means structurally consistent with the binary, not proven
semantically equivalent. Record a confirmed finding per step 6 of
[SKILL.md](SKILL.md).
