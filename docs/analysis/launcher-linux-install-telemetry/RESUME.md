# RESUME: launcher-linux-install-telemetry

Ledger: README.md. Packets: work-packets.md, plus work-packets-li09.md for LI-09. Host: linux (Steam Deck).

## State (2026-10-25)

- All packets are done. The final PR (LI-09 + docs + ledger) is ready to open.
- Ruling: LI-02-ruling-1 (owner/architect, 2026-10-24; not in git). LI-01..LI-04 are Superseded: #2020/#2023 landed `unpack::cab_portable` on main (F-15).
- LI-05 (#2025), LI-06 (#2026), LI-08 + LI-07 (#2027): merged to main. LI-09: head 3d2c72a34, packet review `ship`, gate G-LI9 PASSED (below).

## Owner PR rule (2026-10-24)

- No new PRs until the end of the campaign. Merged: #2025 (LI-05), #2026 (LI-06), #2027 (LI-08+LI-07). LI-09 and the close-out docs (project-status) go in ONE final PR.

## Gate G-LI9 (2026-10-25): PASSED

- Colo env endpoint now holds an `https` value: `upload_endpoint_source="env"`, https, zero `upload_endpoint_plain_http` / `upload_endpoint_not_https` since the restart.
- Owner confirmed a real launcher's Upload debug logs completed over https (upload complete).
- The 2026-10-24 failure (plain-http endpoint on colo) is resolved by the owner's colo env fix.

## Next action

1. Open the final PR: LI-09 (head 3d2c72a34) + close-out docs + ledger (see § Owner PR rule).
2. UAT U1-U4 (README § UAT).
3. Cleanup (README § Close-out; needs hands-on or the owner):
   - delete origin branch `launcher-linux-install-telemetry/integration`
   - retire worktrees li-01, li-02, li-03, li-integration
   - after merge, delete local branches li-01, li-02, li-03, integration, li-09
   - delete the debug build output under `~/.cache/cimmeria-lane-targets/src/`

## Open owner decisions (none block a packet)

- OQ-3 (D-LI13): should LT-41's default still ship, and apply to third-party http servers?
- Confirmable: D-LI14 keeps loopback http for game telemetry.

## Close-out

- UAT rows U1-U4 (README § UAT). Campaign review loop.
- Delete the abandoned branches (README § Close-out; needs hands-on or the owner).
- Update `docs/project-status.md` once.
