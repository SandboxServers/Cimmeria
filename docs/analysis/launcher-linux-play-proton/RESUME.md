# RESUME: launcher-linux-play-proton

Ledger: README.md. Packets: work-packets.md, -w1b.md (PP-20a in -w1b-pp20a.md), -w2b.md, -w3.md.

## State (2026-10-26, PP-14 close-out written)

- Every code packet is Done on the integration branch; PP-U is UATPending. PP-U attempt 4 passed up to
  the EULA screen on the Deck: install, Proton pick, prefix, DirectX, PhysX
  load, windowed window. Login, world entry and in-world PhysX are unverified.
- Draft PR #2032 (integration -> main). Not merged, not pushed by PP-14.
- Post-spec packets: PP-U-FIX (#2036, merged to main; lld replaces mold),
  PP-U-F6 (missing Proton outranks "Game exited"), PP-U-R1 (test-only, never
  merged), PP-F1 (store-reopen flake, own PR after #2032).
- Docs done in PP-14: launch.md split into five pages, project-status.md once,
  unified-uat.md section, README ledger, architect memory note.

## Next (owner and ops)

- Campaign review, then `full-ci` (Windows and macOS jobs) on the branch.
- Owner rows: U8 phase copy, "Preparing Proton" wording, picker hidden with
  no Proton, login and world with PhysX, SigNoz rows tagged `proton`.
- Launcher release built after #2036 (the published one cannot install on Linux).
- Then ops opens the final PR for merge and retires the packet worktrees.

## Open items

- Follow-ups FU-1..FU-17, PP-F1, F-U5 and CI-1/CI-2 are in the ledger README.
- First-campaign cleanup (branches, li-* worktrees, build output) needs
  hands-on. Never run git worktree prune.
- Game Mode (U6) is a follow-up per OD-2.
