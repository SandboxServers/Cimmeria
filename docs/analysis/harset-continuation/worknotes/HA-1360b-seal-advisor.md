# HA-1360b seal: advisor report

Seal written to docs/analysis/harset-continuation/worknotes/HA-1360b-seal.md (CRLF), plus work-packets.md status rows (HA-1360a3c Integrated, HA-1360b Sealed).

- Branch verdict: harset/ha-1360b-marsh-receipt (3e0b0212d) is Castle CS-08a1 work (patch-equivalent to 65d3e88be), not an ancestor of the campaign tip, and predates A1-A3c. Stale for 1360b; retire, cherry-pick -x the carry-in onto the campaign, recreate the worktree from that SHA. The base SHA is therefore a verifiable precondition, not a literal.
- Split: 1360b-1 (seed, cap, replay/linter tests) and 1360b-2 (new services live-DB guard, revert probes), each about 35k.
- Next sealable packets: HA-1360a2c (not re-read, verify), HA-00c static audit half, HA-15b/c audit-only slice.
- Not done: no code, seed, lane run, lab or client use; the services sentinel window recommendation (0x7000_8600) was not scanned for free and the coder must confirm.