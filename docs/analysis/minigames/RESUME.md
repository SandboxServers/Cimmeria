# Minigames resume card

- **Phase:** designed 2026-10-19 by the architect against `main` @ `0d71b3b30`. Nothing has been dispatched or built.
- **Ledger:** [README.md](README.md): findings F1-F19, decisions D-MG1..16, the missing-content register M1-M9, gate G1, and 42 packet rows. **Packets:** [work-packets.md](work-packets.md) (dispatch rules, contract C1-C7, waves 0A, 0B and 1), plus [packets/](packets/) (waves 2, 3-5 and 6-7).
- **Goal:** every minigame with client content runs as a real server-validated game. The scope is Livewire (done; parity audit only), Alignment, CrystalGame, GoauldCrystals and ConverseBasicHumanoid. The five shells (Hack, Activate, Analyze, Bypass, Converse) have no rules to recover; they stay client-declared and hardened unless the owner rules otherwise (D-MG3).
- **Next action:**
  1. Run the peer check (work-packets rule 1).
  2. Create the board subcategory "minigames" through ops.
  3. Dispatch these two sets together:
     - **wave 0A:** MG-R0, MG-R6 and MG-R7, into integration worktree `mg-wave0a`;
     - **wave 1 Ready:** MG-01, MG-02, MG-03 and MG-04, one PR each.
  4. MG-05 follows MG-03, and MG-06 follows MG-02.
- **After 0A merges:** dispatch wave 0B (MG-R1..R5), then the architect runs G1. G1 fills the port packets and the `<from MG-R5/R7>` values, and rules on MG-25, MG-LW1 and MG-23.
- **Owner directives 2026-10-19 (override anything above):** run autonomously to release-ready; NO push and NO PR until the whole restoration is done; packets merge locally into integration branch `minigames/integration` (worktree `mg-integration`), one PR at the end; retire each packet worktree on local merge and delete its target dir (including on the B drive).
- **Decisions taken (orchestrator, high confidence):** D-MG3 = A (shells stay client-declared, hardened); D-MG6 = A (keep Harset D-H13 for objective 4203); D-MG11 = ship pre-game dialog default off; D-MG4 and D-MG8 stand.
- **In flight:** peer check + integration worktree setup (ops).
- **Owner decisions (BlockedDecision; ask before dispatching what depends on them):**
  - D-MG3, the shell games: A, keep them client-declared (recommended); B, alias a shell to a real game; C, out of scope. MG-26 and MG-64 wait on it.
  - D-MG6, objective 4203: A, keep Harset D-H13 (recommended); or B, a real minigame. MG-65 waits on it.
  - D-MG11, the pre-game dialog: ship it default off, then turn it on after lab UAT? MG-21 ships with it off regardless.
  - The owner may also overrule D-MG4 (the social layer is out) and D-MG8 (an effect script rather than `ability_used`).
- **Lab:** ask the user before any lab use. MG-73 only writes recipes.
- **Worker rules:** PowerShell only; cargo only through the lane; CRLF docs; no stash, prune or push; no db/scripts migrations; one worktree and one test DB per packet.
- **Status docs** (`gap-analysis*`, `project-status.md`) change once, in MG-99.
