# CS-01 preflight: isolated Castle server

> **Packet:** CS-01b (isolated Castle stack preflight). **Worktree:** `castle-cs-01b`. **Branch:** `castle/cs-01b-stack`.

## Commits

- `FEATURE_SHA`: `da2b82c709bb28a8d374f535b032f838ec0a743f`
- `HEAD` at start: `da2b82c709bb28a8d374f535b032f838ec0a743f` (base check passed, no own commits yet)

## Build and DB

- Exe: `cimmeria-server.exe` (copied from `B:\targets\castle-cs-01b\release\`)
- Exe SHA-256: `A4A7878C3ACA71EB9287AAAB59E76CD4A1D903A7C2F448B8273B71D52AB959C8`
- DB: `sgw_castle_cs_01b` (reloaded with `reload-db.ps1`, 350 content chains)
- `--check-db`: exit 0

## Running server

- PID: `36540`
- Start time: `2026-10-17 18:52:15` (local)
- Stop: `Stop-Process -Id 36540`

| Listener | Port | Owner |
|---|---|---|
| Login HTTP (SOAP) | TCP 18081 | 36540 |
| Admin REST (loopback) | TCP 18443 | 36540 |
| Lab MCP (loopback) | TCP 8452 | 36540 |
| Minigame TCP | TCP 30100 | 36540 |
| BaseApp UDP | UDP 32932 | 36540 |

Cellblock's listeners (8081, 32832, 8443, 8451, 30000) were not touched.

## Session env line for the orchestrator restart

`CIMMERIA_LAB_MCP_URL=http://127.0.0.1:8452/mcp`

## Lab instance facts

- `lab instances status`: `p4` (account `lab4`) and `p5` (account `lab5`) are seeded. Neither has a client running.
- Castle DB `account` table: `lab4` and `lab5` both present, `accesslevel` 2.
- No passwords or secrets recorded.
