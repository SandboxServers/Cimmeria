---
name: rtti-table-shift-and-rva-va-traps
description: Two recurring doc-accuracy failure modes in RE tables — one-row RTTI address shifts, and Atrea config tables that silently mix RVAs with absolute VAs
metadata:
  type: project
---

Two failure modes keep recurring in hand-authored RE address tables. Both were hit again
during the 2026-07-25 `docs/technical/` accuracy audit.

**1. One-row shift in contiguous RTTI tables.** When a doc transcribes a run of adjacent
`.rdata` type-descriptor strings into a table, a single dropped or inserted row shifts every
subsequent address by one slot. The tell is a table whose *last* row has a blank address
while every other row is populated — the missing entry got pushed off the end.

**Why:** the addresses are individually plausible (they all point at real type descriptors
in the right region), so nothing looks wrong on spot-check. Only a full sequential re-read
catches it. Same class as the `annotation-script-shift-bugs.md` contactList / Mercury 6 /
SGWNetworkManager 20 corrections.

**How to apply:** when auditing any table of consecutive RTTI addresses, sweep *every* row
with `inspect_memory_content` and diff name-vs-name — never sample. Watch for the blank
trailing cell as the cheap early signal.

**2. Atrea `AtreaLoader.config.xml` mixes RVAs and absolute VAs in one column.** The
`<Symbol>` entries are not uniform: some declare image-relative offsets (add `0x00400000`),
others declare absolute VAs. Entries with leading zeros below `0x00400000` are RVAs.

**Why:** reading the whole column as VAs puts several entries below the `.text` floor at
`0x00401000`, where they resolve to nothing — which then gets written up as "address not
found in binary" or, worse, silently mis-attributed to whatever function does sit there.

**How to apply:** before citing any address from an Atrea config table, test both
interpretations in Ghidra and prefer the one that lands on a *function entry* with a
plausible signature. `appFailAssertFunc` is the easiest anchor — its `(char* Expr, char*
File, int Line)` signature is unmistakable.

Related: [[mercury-wire-format-openqs]], [[project_annotation_sweep_s3]].
