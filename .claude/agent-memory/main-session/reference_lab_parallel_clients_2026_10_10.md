---
name: reference_lab_parallel_clients_2026_10_10
description: Up to five lab clients in one daemon (CIMMERIA_LAB_INSTANCES), a lease per instance, per-instance USERPROFILE fixes the cache-lock freeze; routing traps and what is still unverified live
metadata:
  type: reference
---

Lab parallel clients campaign (#1312, ledger `docs/analysis/lab-parallel-clients/`), merged 2026-10-10 except LP-05b2 and the LP-07 live UAT.

- **Root cause of "two clients freeze":** a running `SGW.exe` holds all 22 `Cache.en-US\*.pak` open read/write with read-only sharing. A second client on the same Documents folder gets cooked version 0 and a ~59,000-entry full resync at every login (~75 s on loopback). Not a server bug; fixed lab-side.
- **Fix:** every lab client, default included, launches with `USERPROFILE` = `Binaries\sessions\instances\<label>\profile`, seeded once from the real `SGWGame` (top files, Config, Content, Cache.en-US). The seed never reruns: a later change to the real `Config\*.ini` does not reach a seeded instance; delete its `profile` folder (client stopped) to reseed. `CIMMERIA_LAB_SHARED_USER_DIR=1` opts out.
- **OneDrive / absolute Documents defeats it** (warning `user_dir_shared`, `reason = documents_not_redirectable`); the durable fix is a DLL `SHGetFolderPathW` hook (tooling backlog B9).
- **Setup:** `CIMMERIA_LAB_INSTANCES=default,p2,p3,p4,p5` in `labd.env`, `pwsh tools/lab/instances.ps1 init`, `daemon.ps1 restart`, then `/mcp` in every session (the `instance` argument is advertised only when more than one instance is hosted).
- **Routing trap:** a call routes by `instance` (label `p2` or account `lab2`; label wins), else by its `lease_id`, else to the **first** instance. Read-only tools carry no lease, so `lab_screenshot` without `instance` shows the default client, not yours. Account-name routing needs a readable account file.
- Leases are per instance: a `lab2` lease is refused on `lab3`, refusal text ends `(instance p3)`. `lab_lease_status` without `instance` lists all instances, never lease ids.
- Cap default = hosted count (at least 2), ceiling 5. Watchdog boot grace 90 s (a bridge took ~25 s to come up with other clients running).
- Proven by hand: four clients in world at once with own profiles. **Not yet proven live on the merged code:** five clients in one daemon with five leases (LP-07).

Runbook: `docs/guides/live-research-lab.md#parallel-clients-up-to-five`; evidence: `docs/reverse-engineering/findings/multi-client-lab.md`. Related: [[reference_lab_mcp_token_cost_2026_10_10]], [[project_mac_client_cooked_version_zero]].
