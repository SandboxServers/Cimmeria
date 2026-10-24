# LT-00 worknote: SigNoz triage (read-only)

Window: last 14 days, read through the SigNoz MCP on 2026-10-21, after the colo deploy. Counts only; no addresses, names, session ids or paths.

## 1. Mint rows ("Minted dev-session telemetry token"), service cimmeria-server

| session_kind | rows |
|---|---|
| lab | 24 |
| player | 8 |

Total 32. Only 8 player mints in 14 days.

## 2. Boot line "dev-session telemetry:" (cimmeria-server)

- "mint and ingest enabled": 22 rows. `upload_endpoint` is **loopback: no** on every sampled row (the 5 newest, including the current deploy): a public DNS name with plain `http`, port 8081. So the colo env var is set and the server is not handing out localhost. It is plain http, which directive 5 / LT-41 (D-LT17) is about.
- "every mint and upload will be refused: ... HMAC secret not set": 3 rows (older boots, since fixed).
- No `upload_endpoint_unset`, `_loopback` or `_plain_http` boot WARNs exist yet, as expected (LT-01 not shipped).

## 3. cimmeria-client rows by client_target

- `client.launcher.*`: 1 row in total, `client.launcher.install_result` (1). No `client.launcher.adoption` or any other launcher target.
- `launcher.summary`: 0 rows in any service (body, client_target and scope_name checked across all services).
- Everything else is game-DLL telemetry (dominated by audio, mercury packets and rollups; about 8.5M client rows in total with a `client_target`).

## 4. cimmeria-server rows with scope `launcher.ingest` (grouped by body)

| body | rows |
|---|---|
| upload-chunk accepted | 125832 |
| telemetry upload refused: token_expired | 1366 |
| telemetry upload refused: busy | 347 |
| telemetry upload refused: bad_token | 189 |
| entity-ID label channel info line | 25 |
| telemetry upload refused: over_budget | 3 |
| telemetry upload refused: body_read_failed | 3 |
| telemetry upload refused: missing_token | 1 |

## Findings

- Launcher event forwarding is effectively absent in SigNoz: 1 launcher row and 0 summary rows in 14 days, against 8 player mints. This matches the campaign premise (F-series): few real launchers upload, and only the install-result row has ever reached the server.
- Ingest works for the game DLL path (125k chunks accepted). The refusals are dominated by `token_expired` (1366), worth a look in a later packet; not a launcher-target issue.
- The colo endpoint is non-loopback but plain http. LT-01 (no localhost fallback) is unaffected by this; LT-41 remains the https decision.
- Caveat: row 3 counts only rows with a `client_target` attribute. Summary rows might carry another attribute; the broad filter over body, client_target and scope_name found none.
