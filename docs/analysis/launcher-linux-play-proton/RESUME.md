# RESUME: launcher-linux-play-proton

Ledger: README.md. Packets: work-packets.md (rules R1-R9, contract, waves
0-2), work-packets-w1b.md (wave 1B, R10, C-7), work-packets-w2b.md (PP-S2,
PP-15, PP-16, PP-08W, C-8), work-packets-w3.md (waves 3-4). Host: linux.

## State (2026-10-25)

- PP-S1 done, ruled (worknotes/PP-S1-ruling.md): GE-Proton 11 reaches login
  with bare `runinprefix`, native d3dx9_40 + xinput1_3, windowed profile.
- Base: origin/main 0a25ef5d4. Wave 1 status: ask ops.

## Owner rules

- One final PR (D-PP17); packets merge-noff into
  `launcher-linux-play-proton/integration` (worktree pp-integration).
- No Wine for cabinet expansion. No client patch beyond what macOS does.

## Owner decisions

- OD-1, OD-2: decided 2026-10-25 (no GE download; Desktop Mode gates merge).
- OD-3: decided 2026-10-25: drop the staged re-read. PP-20 re-specced
  (step 9, C-7b, D-PP26, F-23); downloads and patches keep their hash.

## Next action

1. ops: commit the ledger (README, RESUME, work-packets*.md, worknotes/) to
   the integration branch; it is untracked in the main checkout only.
2. Dispatch W1B beside W1: PP-24, PP-25, PP-20 Ready now; PP-22, PP-21,
   PP-23 after PP-20. Dispatch PP-S2 (hands-on, owner present).
3. W2 after W1 merges: PP-05, PP-06, PP-07, PP-15 (after PP-20, PP-25),
   PP-16, PP-08, PP-09.
4. After PP-S2: architect ruling worknotes/PP-S2-ruling.md (PP-08W, PP-17).
5. PP-13 waits for the sibling campaign's final PR (its U1 row).

## Waves (merge order)

W1 PP-01, PP-04, PP-02, PP-03 | W1B PP-24, PP-25, PP-20, PP-22, PP-21,
PP-23 | W2 PP-05, PP-06, PP-07, PP-15, PP-16, PP-08, PP-09, PP-08W | W3
PP-10, PP-11, PP-17, PP-12, PP-13 | W4 PP-U, PP-14.
