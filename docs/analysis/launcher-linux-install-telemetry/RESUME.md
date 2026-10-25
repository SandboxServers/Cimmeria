# RESUME: launcher-linux-install-telemetry

Ledger: README.md. Packets: work-packets.md, plus work-packets-li09.md for LI-09. Host: linux (Steam Deck).

## State (2026-10-24, after the merges of #2025, #2026, #2027)

- Ruling: LI-02-ruling-1 (owner/architect, 2026-10-24; not in git). LI-01..LI-04 are Superseded: #2020/#2023 landed `unpack::cab_portable` on main (F-15).
- LI-05 (#2025), LI-06 (#2026), LI-08 + LI-07 (#2027): merged to main. LI-09: head 3d2c72a34, packet review `ship`; waits for G-LI9 (FAILED, see below).
- Integration branch: abandoned, and it IS on origin (tip `9d61134f7`, the LI-01 merge). Close-out deletes it on origin and locally, plus local li-01/li-02/li-03, after their worktrees are removed (README § Close-out). ops cannot delete branches: hands-on or the owner does it.

## Owner PR rule (2026-10-24)

- No new PRs until the end of the campaign. Merged: #2025 (LI-05), #2026 (LI-06), #2027 (LI-08+LI-07). LI-09 and the close-out docs (project-status) go in ONE final PR.

## Gate G-LI9 (2026-10-24): FAILED

- Owner ran the read-only colo check. `upload_endpoint_source="env"` on all 40 log lines since the 2026-10-23 start; `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT` starts with `http://` and is not loopback; boot log has one `upload_endpoint_plain_http` warning (the code uses `plain_http` for http values, `not_https` only for other schemes, so grep the former too). `upload_endpoint_not_https` count 0.
- Fix is the owner's (colo, not the repo): set that env var to an `https://` URL, or unset it so the request-host default applies. Then re-run the check. LI-09 (head 3d2c72a34, reviewed ship) must not merge before that, or colo game telemetry stays on plain http. Owner earlier said "skip the gate"; this result supersedes it. Re-confirm before the final PR.

## Next action

1. ops runs G-LI9 once the owner has fixed the colo env (see § Gate G-LI9).
2. LI-09 and the close-out docs ship as the final PR (see § Owner PR rule).
3. UAT U1-U4 (README § UAT).

## Open owner decisions (none block a packet)

- OQ-3 (D-LI13): should LT-41's default still ship, and apply to third-party http servers?
- Confirmable: D-LI14 keeps loopback http for game telemetry.

## Close-out

- UAT rows U1-U4 (README § UAT). Campaign review loop.
- Delete the abandoned branches (README § Close-out; needs hands-on or the owner).
- Update `docs/project-status.md` once.
