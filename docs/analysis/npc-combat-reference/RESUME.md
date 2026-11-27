# RESUME: npc-combat-reference (NC)

Ledger: README.md. Packets: work-packets.md (rules, contract, W0),
work-packets-w1-w2.md and work-packets-w3-w5.md. Rulings: worknotes/wave0.md.

## State (2026-11-27, W0 shipping)

- W0 packets NC-01..07 all passed packet review, the wave review and the
  wave 0 ruling check (D1 class rule applied; two last notes fixed).
- PRs: NC-01 #2082, NC-04 #2083 (merged), NC-03 #2084, NC-02 #2085,
  NC-07 #2086, NC-05 #2087, NC-06 #2088 (merged). Plan docs: #2081.
- Main is red (#2080 cancelled nightly, #2072 old container smoke). The
  owner authorised --allow-red-main for the W0 PRs. A fresh ci run on main
  was dispatched (46029521588).
- Failed non-ci jobs on #2082/#2085/#2086/#2087 were 10-minute timeouts;
  reruns started. ops cannot rerun non-ci workflows; use gh directly.

## Next action

1. Merge the rest in order: NC-01, NC-03, NC-07, NC-02, NC-05 once green.
2. Retire leftovers: branch nc-04-seed-trace (folder already gone).
3. Ask the owner D-NC1..D-NC8 (README § Decisions), then dispatch W1
   (NC-10, 11, 12, 14, 15) once NC-01 and NC-03 are merged. NC-12 needs D-NC2.
4. NC-31 carries: README F6/F3 rewrite (note 16), stale "mob notifies" rows,
   MeleeDistance wording.

## Waves

| Wave | Packets | Gate |
|---|---|---|
| W0 | NC-01..07 | none (shipping) |
| W1 | NC-10, 11, 12, 14, 15 | NC-01 and NC-03 merged; NC-12 needs D-NC2 |
| W2 | NC-13, 20, 21a-c, 23, 24 | NC-12, NC-15; D-NC4, D-NC5 |
| W3 | NC-22, 30, 31, 33, 34 | D-NC1, D-NC3 |
| W4 | lab NC-40..44 (D-NC7), plus NC-32 docs | W3 merged |
| W5 | NC-50 review, NC-51 close-out | W4 |

Open: D-NC1..D-NC8, all BlockedDecision. D-NC9 decided (spawns 14300-14319).
