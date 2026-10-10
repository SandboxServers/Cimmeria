---
name: re-lookup
description: Answer a question about how the 2009 Stargate Worlds client or original server behaves (an opcode, method index, wire layout, entity typeID, a function at an address, what the client expects) in the cheapest trustworthy order - repo docs and dispatch tables first, then the Ghidra MCP, then headless Ghidra, then x64dbg on the live client - and record what you find. Use for "what does the client do with X", "which index is method Y", "decompile 0x...", "is this constant right", "verify this reconstruction against the binary" (see re-verify.md), or before writing any throwaway disassembler script.
---

# RE lookup: docs, then Ghidra, then the live client

Most "unknown" client behaviour is already written down. Every step below is
cheaper than the one after it, so stop at the first step that answers the
question with a citation.

## 1. Search the docs first

1. Dispatch tables, for any method index, opcode or message ID:
   `docs/protocol/message-dispatch-table.md`,
   `docs/protocol/cell-method-dispatch-table.md`,
   `docs/protocol/client-method-dispatch-table.md`,
   `docs/protocol/sgwplayer-base-method-dispatch-table.md`.
   Never count `.def` entries by hand when a table exists.
2. Address-cited findings: `docs/reverse-engineering/findings/` (about 100
   files; grep by system, class or address) and
   `docs/reverse-engineering/decompiled/00_INDEX.md`.
3. The rest of `docs/` (grep the system name), then agent memory under
   `.claude/agent-memory/` (leads, not evidence).

Rules that cost the most when missed
([docs/agents/rules-and-gotchas.md](../../../docs/agents/rules-and-gotchas.md)):

- **Wire entity typeIDs are the client's clientIndex**: `<ServerOnly/>`
  entries are skipped when numbering, so `Account = 0x07`, not its
  `entities.xml` row.
- **A ticket, draft chapter or handoff is a claim, not evidence.** Trust
  order is in [docs/agents/domain.md](../../../docs/agents/domain.md): the
  binary and client files, then captures, then `docs/protocol/` and cited
  findings, then drafts, then issue text. `deprecated/` shows original-server
  intent only.

## 2. Ghidra MCP (GUI project open)

- Load the tool group you need (`list_tool_groups`, then `load_tool_group`),
  then `list_instances` / `connect_instance` to attach to the SGW project.
- Decompile, list xrefs and search strings there. Batch related lookups.
- Setup is per machine: [docs/guides/re-toolchain-setup.md](../../../docs/guides/re-toolchain-setup.md).
  The MCP needs someone to have opened the project in CodeBrowser.

## 3. Headless Ghidra (no GUI, or many lookups at once)

Use [tools/re/ghidra-headless/Probe.java](../../../tools/re/ghidra-headless/README.md).
It decompiles (`D:`), lists xrefs (`X:`), searches strings (`S:`, `U16:`)
and function names (`FNSUB:`), dumps vtables (`VT:`), and **disassembles**
(`I:<addr>+<n>`, `IF:<addr>`, `RE:<regex>`, `BYTES:<addr>+<n>`).

- **Do not write a throwaway disassembler.** Sessions keep rebuilding
  `sgwdis.py` / `pe_dis.py` in a scratchpad. `Probe.java` already covers
  disassembly. If you truly need something it can't do, check `tools/` first,
  and if you write a tool, commit it under `tools/re/` with a README line.
- One run at a time, never while the GUI holds the project, always
  `-readOnly`. A killed run leaves `SGW.lock` / `SGW.lock~`. Delete them only
  when no `java`/`javaw` process is running.
- Each run costs 10 s to 2 min of JVM and project open, so put every token
  in one run.
- Write `<addr>+<n>`, not `<addr>,<n>`: `cmd` splits at commas.
- A compile error skips the script but the run still exits 0. Grep the
  output for `error:`.

## 4. Live client (x64dbg or the lab)

Only when static analysis can't settle it (runtime values, which branch is
taken):

- **Breakpoints must never pause a server-connected `SGW.exe`.** A paused
  client stalls Mercury and gets disconnected. Use a logging breakpoint:
  `SetBreakpointCondition <addr>, 0`, `SetBreakpointLogCondition <addr>, 1`,
  `SetBreakpointLog <addr>, "<marker {expr}>"`, and leave **fast resume
  off** (with it on, the log never fires). Encode the values you want in the
  log text, since you can't read registers at the hit.
- For Lua, UI state or memory reads on a running client, the Live Research
  Lab is usually easier (see the `lab-uat` skill; ask the user before using
  the lab).

## 5. Verify a reconstruction

To check a ported or reconstructed function against the bytes, follow
[re-verify.md](re-verify.md): Ghidra ground truth, then your reconstruction,
then the LLM-free gate `tools/re_parity.py`, looping at most 4 rounds.

## 6. Record what you found

- Cite a Ghidra address for every binary claim and tag confidence per
  [docs/reverse-engineering/evidence-standards.md](../../../docs/reverse-engineering/evidence-standards.md).
- A durable, verified finding goes in `docs/reverse-engineering/findings/`
  (or the matching `docs/protocol/` table). A lead or session fact goes in
  agent memory, committed with the change.
- If a doc, ticket or memory disagreed with the binary, **fix the losing
  source in the same PR** and say so in the PR body.
- Keep RE intermediates (decompiles, listings, probe output) in the session
  scratchpad, never in the repo.
