---
name: bw-reference-tree-absent
description: The BigWorld 2.0.1 reference tree is NOT in this checkout and nothing fetches it — all "BW lib/..." citations across docs/ are unfalsifiable
metadata:
  type: project
---

# The BigWorld 2.0.1 reference tree is not in this repository

`external/engines/BigWorld-Engine-2.0.1/` **does not exist** in this checkout, is not in
git, and **nothing in `setup.ps1` or `bootstrap/` fetches it**. The only BigWorld artifact
present is `external/_downloads/BigWorld-Engine-1.9.1.zip` — a *different version*, placed
manually and left unextracted.

**Why:** confirmed during the 2026-07-25 `docs/engine/` accuracy audit. `external/` contains
only SDL, _downloads, openssl_src, postgresql, postgresql_server, python, qt, recast, signoz,
soci.

**How to apply:**

- Any doc claim citing `BW lib/...`, `BW cellapp/...`, `BW server/...` is **unverifiable
  here**. Do not treat those class listings, enum values, message IDs or code excerpts as
  re-checkable evidence — they are a record of a prior reading.
- When auditing or writing docs, add **one** per-file note saying so rather than annotating
  every line.
- Never re-derive or "confirm" a BigWorld internal from these docs and present it as
  verified. If a fact matters for wire compatibility, verify it against SGW.exe (Ghidra),
  `entities/`, `crates/`, or a live capture instead.
- Note that `deprecated/cpp/` (the original CME C++ server) **is** present and readable —
  paths like `deprecated/cpp/src/cellapp/space.hpp` were confirmed to exist. Those citations
  are checkable; BigWorld ones are not.

Related ground truth: [[entity-def-and-pak-ground-truth]].
