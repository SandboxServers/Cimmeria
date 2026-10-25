# RESUME: launcher-linux-play-proton

Ledger: README.md. Packets: work-packets.md (rules R1-R9, contract, waves
0-2), work-packets-w1b.md (wave 1B, R10, C-7), work-packets-w2b.md (PP-S2,
PP-15, PP-16, PP-08W, C-8), work-packets-w3.md (waves 3-4). Host: linux.

## State (2026-10-25)

- PP-S1 done and ruled (worknotes/PP-S1-ruling.md). Base: origin/main 0a25ef5d4.
- PP-S2 done and ruled (worknotes/PP-S2-ruling.md). The probe fails at
  AddDllDirectory on the `\\?\` path (F-24). PP-08W is specced and Ready.
  PP-17 step 3 seeds SavedSystemOptions.xml on the first Play (D-PP27).
  Linux stays windowed only (D-PP13 final). C-8 is unchanged.
- In flight: PP-01, PP-02, PP-20 (0e64f5b3a, PP-20-ruling-1), PP-24, PP-25.

## Owner rules

- One final PR (D-PP17). Packets merge-noff into
  `launcher-linux-play-proton/integration` (worktree pp-integration).
- No Wine for cabinet expansion. No client patch beyond what macOS does.
- OD-1..OD-4 decided 2026-10-25. OD-4: wait for U3 (the owner chose the
  default). No early draft PR; the i686 worker is built by CI at the final
  PR, and the PP-08W Deck check runs then. PhysX risk stays open until U3.
- Pass-through from reviews: PP-02 telemetry-DLL wording ("stays optional" vs
  all pins required) needs an architect confirm; tagged Linux builds need
  PP-03's fourth pin (PP-03 in review).
- Merged to integration (tip a51239334): PP-02, PP-25, PP-01, PP-03, PP-04,
  PP-24. Wave 1 review pass 1: ship (all four). PP-08W reviewed ship
  (b74d28aa5), merge after PP-08. PP-20 in review pass 2 (fix 1 09fe38073).
- Packet worktrees from the integration tip need a hands-on agent
  (mk-worktree.sh cuts from origin/main only).

## Next action

1. Copy PP-S2-findings.md to worknotes/PP-S2.md (ops commits the ledger).
2. Dispatch PP-08W (rust-gameserver-dev, worktree pp-08w). It can run now
   and merges after PP-08. Then ops gets the CI artifact (OD-4), and a
   hands-on Deck check writes worknotes/PP-08W-deck.md.
3. W1B: PP-20a, then PP-20b; ops merges pp-20 after PP-25; then PP-22,
   PP-21, PP-23, then the wave-1B Deck UAT.
4. W2 after W1: PP-05, PP-06, PP-07, PP-15, PP-16, PP-08, PP-09, PP-08W.

## Waves (merge order)

W1 PP-01, PP-04, PP-02, PP-03 | W1B PP-24, PP-25, PP-20 (a+b), PP-22, PP-21,
PP-23 | W2 PP-05, PP-06, PP-07, PP-15, PP-16, PP-08, PP-09, PP-08W | W3
PP-10, PP-11, PP-17, PP-12, PP-13 | W4 PP-U, PP-14.
