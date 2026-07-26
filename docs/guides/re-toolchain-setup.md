---
type: how-to
audience: New Cimmeria contributors setting up the reverse-engineering toolchain (Ghidra MCP + x64dbg MCP + Claude Code)
last_updated: 2026-07-25
prerequisites: [Windows 10/11, PowerShell 7+, ~10 GB free disk]
companion_docs:
  - reverse-engineering-with-claude.md
  - sgw-live-debugging.md
  - ../reverse-engineering/toolchain/install-ghidra-mcp.md
---

# Reverse-Engineering Toolchain Setup

End-to-end walkthrough for getting Ghidra, x64dbg / x32dbg, and their MCP servers wired into a fresh Cimmeria clone. By the end you can run static analysis on `SGW.exe` from Ghidra and dynamic analysis from x64dbg, with both reachable from Claude Code via MCP.

> **The short version.** If you trust the bootstrap, run this from the repo root:
>
> ```powershell
> pwsh setup.ps1 -WithReToolchain -NoLaunch
> ```
>
> That downloads everything, installs the plugins, builds the venvs, and writes `.mcp.json`. The rest of this guide walks the manual path and explains what the bootstrap is doing.

## Prerequisites

| Tool | Version | Used for | Install |
|---|---|---|---|
| Windows | 10 / 11 | The server runs on Windows next to the game client | — |
| PowerShell | 7.0+ | Bootstrap script | [`winget install Microsoft.PowerShell`](https://github.com/PowerShell/PowerShell) |
| Git | Any recent | Clone Cimmeria + ghidra-mcp | [git-scm.com](https://git-scm.com/) — or let the bootstrap winget-install it |
| JDK 21 | LTS | Ghidra 12 | [Adoptium Temurin 21](https://adoptium.net/) — or let the bootstrap winget-install it |
| Python | 3.11+ | MCP bridges | [python.org](https://www.python.org/) — or let the bootstrap winget-install it |
| Disk | ~10 GB | Ghidra ~1 GB, x64dbg ~50 MB, ghidra-mcp clone ~100 MB, venvs ~200 MB, the rest for caches | — |

The Cimmeria game-server prerequisites (Rust, PostgreSQL, etc.) are covered separately in [bootstrap/README.md](../../bootstrap/README.md) — you don't need them just for RE.

> **Auto-install.** The bootstrap (`Install-CimmeriaReToolchain`) detects missing Git / JDK 21 / Python 3.11+ and tries `winget install` first. If winget is unavailable, it prints the manual download URL and aborts. If you'd rather skip the auto-install (corporate-managed laptop, custom JDK, etc.), install them yourself before running `setup.ps1` and the bootstrap will detect them on PATH.

## Path overview

After the bootstrap (or following this guide), your tree will look like:

```text
Cimmeria/
├── ghidra/
│   └── ghidra_12.0.4_PUBLIC/        # gitignored
├── dbg/
│   └── release/
│       ├── x96dbg.exe                # launcher; spawns x32 or x64
│       ├── x32/plugins/x64dbg-automate.dp32 + libzmq-mt-4_3_5.dll
│       └── x64/plugins/x64dbg-automate.dp64 + libzmq-mt-4_3_5.dll
├── external/
│   └── ghidra-mcp/                   # gitignored; bethington/ghidra-mcp clone
├── .venvs/
│   ├── ghidra-mcp/                   # Python bridge for Ghidra MCP
│   └── x64dbg-mcp/                   # PyPI x64dbg-automate package
└── .mcp.json                         # gitignored; copy of .mcp.json.example
```

The `ghidra/`, `dbg/`, `external/`, `.venvs/`, and `.mcp.json` paths are all in `.gitignore`. A teammate's fresh clone will look "broken" until they run the bootstrap or this guide.

## Path A — automated (recommended)

From the Cimmeria repo root in PowerShell 7:

```powershell
pwsh setup.ps1 -WithReToolchain -NoLaunch
```

The bootstrap is idempotent — re-runs detect existing installs and skip work that's already done. Internally it:

1. Verifies JDK 21 and Python 3.11+ are on `PATH`.
2. Downloads Ghidra 12.0.4 and extracts to `ghidra/ghidra_12.0.4_PUBLIC/`.
3. Clones `bethington/ghidra-mcp` to `external/ghidra-mcp/`, downloads the pre-built extension zip, and deploys it to `%APPDATA%\ghidra\ghidra_12.0.4_PUBLIC\Extensions\GhidraMCP\`. If the extension zip fails, the upstream `ghidra-mcp-setup.ps1 -Deploy` is your fallback (requires Maven).
4. Looks for an x64dbg snapshot in `external/_downloads/snapshot*.zip` and extracts it to `dbg/`. **You have to drop the zip there manually** — x64dbg's snapshot release URLs rotate weekly, so the bootstrap can't pin one. See the manual step below.
5. Downloads the latest `x64dbg-automate` release and drops the `.dp32` / `.dp64` plugin DLLs (and `libzmq-mt-4_3_5.dll`) into `dbg/release/x32/plugins/` and `dbg/release/x64/plugins/` (only if `dbg/release/` exists, i.e. step 4 succeeded).
6. Creates two Python venvs in `.venvs/` and installs the bridge dependencies.
7. Generates `.mcp.json` from `.mcp.json.example` with your absolute repo path. **Skipped if `.mcp.json` already exists** — the bootstrap will never clobber an existing config.

**The x64dbg pre-step.** Before running `setup.ps1 -WithReToolchain`, download the latest x64dbg snapshot zip and place it in `external/_downloads/`:

1. Open <https://github.com/x64dbg/x64dbg/releases/tag/snapshot>
2. Download `snapshot_<latest-date>.zip` (the most recent asset).
3. Save it as `external/_downloads/snapshot.zip` (any filename starting with `snapshot` and ending in `.zip` works).
4. Then run `pwsh setup.ps1 -WithReToolchain -NoLaunch`.

Skipping this step is fine if you only want Ghidra MCP — the bootstrap will continue and just print a hint for x64dbg.

When it finishes:

```text
Open Ghidra (ghidra/ghidra_12.0.4_PUBLIC/ghidraRun.bat),
  load SGW.exe (game/sgw/Binaries/Win32/SGW.exe),
  run the annotation scripts (docs/reverse-engineering/annotation-scripts/),
  then Tools > GhidraMCP > Start MCP Server.
Open x64dbg (dbg/release/x96dbg.exe).
Restart Claude Code so it picks up .mcp.json.
```

Skip to [Sanity-check the wiring](#sanity-check-the-wiring) to verify.

## Path B — manual

Follow this when you want to learn what's happening, when the bootstrap fails partway, or when you're installing on a platform the bootstrap doesn't cover.

### Step 1 — Install Ghidra and the GhidraMCP plugin

Walk the reference at [`docs/reverse-engineering/toolchain/install-ghidra-mcp.md`](../reverse-engineering/toolchain/install-ghidra-mcp.md). Return here when `Tools > GhidraMCP > Start MCP Server` works in Ghidra.

### Step 2 — Install x64dbg

Download the latest x64dbg snapshot from [x64dbg.com](https://x64dbg.com/) (the SourceForge mirror) or the [x64dbg/x64dbg GitHub releases](https://github.com/x64dbg/x64dbg/releases). Extract the contents of the `release/` folder into `dbg/release/` inside your Cimmeria checkout. After extraction you should have `dbg/release/x96dbg.exe` (the launcher), `dbg/release/x32/x32dbg.exe`, and `dbg/release/x64/x64dbg.exe`.

> **Use x32dbg.exe for SGW work.** SGW.exe is 32-bit — `x64dbg` will not attach. The `x96dbg.exe` launcher detects the target's bitness and spawns the right binary, so launching via `x96dbg.exe` is the safe default.

### Step 3 — Install the x64dbg-automate plugin

Download the latest release from [dariushoule/x64dbg-automate](https://github.com/dariushoule/x64dbg-automate/releases) — at time of writing that's `v0.6.1-green_pepe`. The release ships `.dp32` / `.dp64` plugin DLLs and a `libzmq-mt-4_3_5.dll` runtime.

Copy:

- `x64dbg-automate.dp32` and `libzmq-mt-4_3_5.dll` into `dbg/release/x32/plugins/`
- `x64dbg-automate.dp64` and `libzmq-mt-4_3_5.dll` into `dbg/release/x64/plugins/`

Launch x64dbg (via `x96dbg.exe`) and confirm the plugin loaded: **Plugins menu > about** should list `x64dbg-automate`.

### Step 4 — Create the x64dbg-automate venv

```powershell
python -m venv .venvs\x64dbg-mcp
.\.venvs\x64dbg-mcp\Scripts\Activate.ps1
pip install x64dbg-automate
deactivate
```

This installs the [`x64dbg-automate`](https://pypi.org/project/x64dbg-automate/) PyPI package which provides the `x64dbg-automate-mcp` MCP server entrypoint.

### Step 5 — Write `.mcp.json`

```powershell
Copy-Item .mcp.json.example .mcp.json
```

Open `.mcp.json` and replace every `<CIMMERIA_ROOT>` with the absolute path to your Cimmeria checkout (e.g. `C:\\Users\\you\\source\\projects\\Cimmeria` — Windows paths inside JSON need double backslashes).

For the `cimmeria-rag` block: replace `REPLACE_WITH_FUNCTIONS_KEY` with the Azure Functions key for the project's RAG MCP. The key isn't checked into git — ask in the project chat (or, if you're a contributor without access, just delete the entire `cimmeria-rag` block; the Ghidra and x64dbg MCPs work standalone). When the bootstrap generates `.mcp.json` for you it leaves the placeholder in place unless you pass `-CimmeriaRagKey <value>` — note that's a parameter of the `Install-CimmeriaReToolchain` bootstrap function, not of `setup.ps1`, so to use it you invoke the function directly rather than going through `setup.ps1 -WithReToolchain`.

Verify the Ghidra `GHIDRA_MCP_URL` port matches the port Ghidra's plugin actually bound to (see the port-note in [install-ghidra-mcp.md](../reverse-engineering/toolchain/install-ghidra-mcp.md#3-install-the-ghidramcp-plugin)).

`.mcp.json` is gitignored — never commit it.

## Sanity-check the wiring

1. **Ghidra is open with SGW.exe loaded.** Run the [annotation scripts](../reverse-engineering/annotation-scripts/) in order so the binary has the ~101,909 named functions other RE docs cite.
2. **GhidraMCP server is started.** Confirm with `Tools > GhidraMCP > Server Status` and check Ghidra's console for the actual bound port.
3. **x64dbg is open** (just `x96dbg.exe` is enough — you don't need to attach yet).
4. **Restart Claude Code** so it re-reads `.mcp.json`.
5. Ask Claude Code:

   ```text
   List the MCP tools you have available. Group them by server prefix.
   ```

   You should see three groups: `mcp__cimmeria-rag__*` (~34 tools), `mcp__ghidra__*` (~245 tools), and `mcp__x64dbg__*` (~60 tools).

If a server is missing, re-read its section above. The most common failure is a path mismatch in `.mcp.json` — the bridge process spawns silently and the error only surfaces as "no tools from this server."

## Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| `mcp__ghidra__*` tools missing | Ghidra not open, plugin not started, or wrong port in `.mcp.json` | Open Ghidra, start the plugin, match the port |
| `mcp__ghidra__*` calls return "connection refused" | Plugin bound to a different port than configured (Windows iphlpsvc reservation) | Check the bound port in Ghidra's console; update `GHIDRA_MCP_URL` |
| Ghidra status dialog says "port 8089" but the bridge can't connect | Known upstream display bug: dialog shows configured, not bound port | Trust the console output, not the dialog |
| `mcp__x64dbg__*` tools missing | x64dbg not launched, plugin not installed, or wrong venv path in `.mcp.json` | Launch `x96dbg.exe`, confirm `x64dbg-automate` is in Plugins menu, verify `.venvs/x64dbg-mcp/Scripts/x64dbg-automate-mcp.exe` exists |
| `pip install x64dbg-automate` fails | Wrong Python version or pip cache poisoning | Use Python 3.11+; `pip install --no-cache-dir x64dbg-automate` |
| Ghidra prompts about extension version mismatch | The plugin was built for a different Ghidra version | Rebuild from source: `external/ghidra-mcp/ghidra-mcp-setup.ps1 -Deploy -GhidraPath <path>` |
| Bootstrap hangs on Ghidra download | NSA GitHub release rate-limited | Re-run with `-SkipDownload` and place the zip manually in `external/_downloads/` |
| `pwsh setup.ps1 -WithReToolchain` says it's done but `.mcp.json` is missing | A pre-existing `.mcp.json` blocked overwrite — bootstrap never clobbers | Delete `.mcp.json` and re-run, or copy from `.mcp.json.example` manually |

## What this enables

With all three MCPs wired, Claude Code can:

- Run static analysis through Ghidra: decompile, follow xrefs, rename functions, extract strings, dump structs — see [`docs/reverse-engineering/`](../reverse-engineering/) for the methodology and [`docs/guides/reading-decompiled-code.md`](reading-decompiled-code.md) for interpreting output.
- Run dynamic analysis through x64dbg: set log breakpoints, read memory, attach to the live SGW process — see [`docs/guides/sgw-live-debugging.md`](sgw-live-debugging.md) for the techniques and gotchas.
- Search the Cimmeria knowledge graph: code, docs, entity defs, findings — through the cloud-hosted `cimmeria-rag` server.
- Structurally verify a reconstruction against the binary with the [`/re-verify`](../../.claude/commands/re-verify.md) slash command, which pairs Ghidra MCP ground truth with the LLM-free parity engine at [`tools/re_parity.py`](../../tools/re_parity.py). No extra setup beyond the MCPs above and a Python 3 on PATH — the engine is pure Python with no network calls. Details in the workflow doc. **Note:** as of 2026-07-25 neither file is on `main` yet, so a fresh `main` checkout won't have them.

The workflow that puts them together — when to invoke which agent, how to hand off findings, what NOT to delegate — is documented in [`docs/guides/reverse-engineering-with-claude.md`](reverse-engineering-with-claude.md). Read it before your first dig.

## Cross-references

- [docs/guides/reverse-engineering-with-claude.md](reverse-engineering-with-claude.md) — the workflow doc
- [docs/reverse-engineering/toolchain/install-ghidra-mcp.md](../reverse-engineering/toolchain/install-ghidra-mcp.md) — Ghidra MCP reference
- [docs/guides/sgw-live-debugging.md](sgw-live-debugging.md) — manual x32dbg techniques and the pybag warning
- [docs/guides/reading-decompiled-code.md](reading-decompiled-code.md) — interpret Ghidra output
- [docs/reverse-engineering/evidence-standards.md](../reverse-engineering/evidence-standards.md) — confidence tiers, citation grammar
- [`.mcp.json.example`](../../.mcp.json.example) — template
- [bootstrap/README.md](../../bootstrap/README.md) — bootstrap module docs
- [docs/reverse-engineering/](../reverse-engineering/) — RE plan, status, findings
