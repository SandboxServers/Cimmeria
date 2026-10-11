# Reverse-engineering tools

Tools for reading the 2009 `SGW.exe` without the Ghidra MCP bridge.

| Entry point | What it does |
|---|---|
| [`ghidra-headless/Probe.java`](ghidra-headless/README.md) | Read-only headless-Ghidra probe: decompile, xrefs, string and function-name search, vtable dumps, run through `analyzeHeadless.bat -noanalysis -readOnly` |

`re_parity.py` one level up checks a Rust reconstruction against the Ghidra decompile. Check `docs/` and the dispatch tables before any of these; the order is in the `re-lookup` skill (`.claude/skills/re-lookup/SKILL.md`). Findings live in [docs/reverse-engineering/](../../docs/reverse-engineering/README.md).
