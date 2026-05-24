---
type: reference
audience: Cimmeria contributors setting up the Ghidra MCP bridge for the first time
last_updated: 2026-05-24
prerequisites: [Java 21 JDK, Python 3.11+, Ghidra 12.0.4]
companion_docs:
  - ../../guides/re-toolchain-setup.md
  - ../../guides/reverse-engineering-with-claude.md
  - ../../guides/sgw-live-debugging.md
---

# GhidraMCP Plugin — Installation Reference

The [bethington/ghidra-mcp](https://github.com/bethington/ghidra-mcp) bridge connects Ghidra to MCP-capable clients (Claude Code, GitHub Copilot CLI, others). This page is the reference for what gets installed and where; for the end-to-end walkthrough including x64dbg, start at [`docs/guides/re-toolchain-setup.md`](../../guides/re-toolchain-setup.md).

The fastest path is `pwsh setup.ps1 -WithReToolchain` — it downloads Ghidra and the bridge, builds and deploys the plugin, sets up the Python venv, and writes `.mcp.json` for you. This document covers the manual install and the layout the bootstrap produces.

## What the install puts on disk

| Component | Location | Why |
|---|---|---|
| Ghidra 12.0.4 | `ghidra/ghidra_12.0.4_PUBLIC/` (gitignored) | Static analyzer |
| GhidraMCP plugin extension | `%APPDATA%\ghidra\ghidra_12.0.4_PUBLIC\Extensions\GhidraMCP\` | Discovered at Ghidra startup |
| Plugin source / bridge | `external/ghidra-mcp/` (gitignored) | Source of the plugin + Python bridge |
| Python venv | `.venvs/ghidra-mcp/` (gitignored) | Isolates bridge runtime deps |
| MCP config | `.mcp.json` (gitignored — template at [.mcp.json.example](../../../.mcp.json.example)) | Per-machine wiring for Claude Code |

The `ghidra/`, `dbg/`, `external/`, and `.venvs/` paths are all excluded from git. A fresh clone needs the bootstrap (or this doc) to populate them.

## Manual install

> Skip this section if you ran `pwsh setup.ps1 -WithReToolchain`. Come back here to verify state or troubleshoot a partial install.

### 1. Prerequisites

- **JDK 21+** — required by Ghidra 12. Verify with `java -version`. On Windows, install [Adoptium Temurin 21](https://adoptium.net/) or any LTS distribution; ensure `JAVA_HOME` is set.
- **Python 3.11+** — used for the bridge venv. The user-profile install from [python.org](https://www.python.org/) is fine.
- **Maven** *(only if you intend to rebuild the plugin from source)* — the upstream bridge's own `ghidra-mcp-setup.ps1` will install it automatically when run with `-Deploy`.

### 2. Install Ghidra 12.0.4

Download [ghidra_12.0.4_PUBLIC_20260303.zip](https://github.com/NationalSecurityAgency/ghidra/releases/tag/Ghidra_12.0.4_build) from the official NSA release and extract it to `ghidra/ghidra_12.0.4_PUBLIC/` inside your Cimmeria checkout. The bootstrap places it there too — keeping the install inside the repo workspace means the `<CIMMERIA_ROOT>\ghidra\...` path in `.mcp.json.example` works without further edits.

First launch (`ghidraRun.bat`) sets up the user-profile project directory at `%APPDATA%\ghidra\ghidra_12.0.4_PUBLIC\`.

### 3. Install the GhidraMCP plugin

Two options:

**A. Pre-built extension zip** — Download the latest `GhidraMCP-<version>.zip` from [bethington/ghidra-mcp releases](https://github.com/bethington/ghidra-mcp/releases). In Ghidra: `File > Install Extensions > +` and select the zip. Restart Ghidra.

**B. Build from source** — Clone the repo:

```powershell
git clone https://github.com/bethington/ghidra-mcp.git external/ghidra-mcp
cd external/ghidra-mcp
.\ghidra-mcp-setup.ps1 -Deploy -GhidraPath "$PWD\..\..\ghidra\ghidra_12.0.4_PUBLIC"
```

The upstream script handles Maven, builds the plugin, and copies it to `%APPDATA%\ghidra\ghidra_12.0.4_PUBLIC\Extensions\GhidraMCP\`.

After restart: `File > Configure > Configure All Plugins`, tick **GhidraMCP**, then **Tools > GhidraMCP > Start MCP Server**.

> **Port note (Windows).** The plugin defaults to TCP 8089. On Windows, `iphlpsvc` (IP Helper service) reserves 8089–8099. The plugin walks 8089..8104 and binds the first free port — on most Windows boxes that ends up being **8100**. Confirm the actual bound port from Ghidra's console output, not the status dialog (upstream display bug). Match `GHIDRA_MCP_URL` in `.mcp.json` to the real port.

### 4. Create the Python venv

```powershell
python -m venv .venvs\ghidra-mcp
.\.venvs\ghidra-mcp\Scripts\Activate.ps1
pip install -r external\ghidra-mcp\requirements.txt
deactivate
```

Use a 32-bit Python if you intend to attach debuggers later (the upstream pybag integration is 32-bit). For pure static analysis through MCP, the architecture doesn't matter.

### 5. Wire `.mcp.json`

Copy the template and substitute your absolute repo path:

```powershell
Copy-Item .mcp.json.example .mcp.json
# Edit .mcp.json and replace <CIMMERIA_ROOT> with your absolute Cimmeria path
```

The `.mcp.json` file is gitignored — never commit credentials or local paths.

### 6. Sanity check

Open Claude Code in the repo and run:

```text
List tools from the ghidra MCP server.
```

You should see ~245 `mcp__ghidra__*` tools. If the call returns "connection refused":

1. Confirm Ghidra is open with SGW.exe loaded.
2. Confirm the plugin is started (`Tools > GhidraMCP > Server Status`).
3. Confirm the port in `.mcp.json` matches the bound port.

## Cross-references

- [docs/guides/re-toolchain-setup.md](../../guides/re-toolchain-setup.md) — end-to-end how-to for both Ghidra MCP and x64dbg MCP, plus prerequisites and troubleshooting.
- [docs/guides/reverse-engineering-with-claude.md](../../guides/reverse-engineering-with-claude.md) — workflow doc: when to invoke `game-archaeology-specialist`, evidence handoff, what NOT to delegate.
- [docs/guides/sgw-live-debugging.md](../../guides/sgw-live-debugging.md) — manual fallback when MCP-driven flows fail. Notes the pybag incompatibility with SGW.
- [docs/guides/reading-decompiled-code.md](../../guides/reading-decompiled-code.md) — how to interpret Ghidra decompiler output.
- [docs/guides/evidence-standards.md](../../guides/evidence-standards.md) — confidence tiers, citation grammar.
- [`.mcp.json.example`](../../../.mcp.json.example) — the template `.mcp.json`.
