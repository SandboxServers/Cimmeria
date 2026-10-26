# launcher-telemetry-linux: resume card

> **Type:** Reference · **Audience:** The orchestrator resuming this campaign · **Updated:** 2026-10-26 (close-out)
> **Companions:** [Ledger](README.md), [work packets](work-packets.md), [wave 1 ruling](worknotes/wave1.md), [B1 research](worknotes/LTL-B1.md), [Deck UAT steps](../../guides/unified-uat.md#launcher-telemetry-on-linux-ltl-u)

**State: code complete, reviewed, not yet a PR.** All code packets (LTL-01 to 07, LTL-B2b, LTL-B2) are integrated in `launcher-telemetry-linux/forwarding` (worktree `ltl-int`). The campaign review ended `ship` at `69c6a1005`; LTL-08 (this close-out, docs only) sits on top. All owner decisions are made: D-LTL3 approved, D-LTL4 yes, D-LTL7 answered (the PR waits for LTL-B2, which is done). LTL-B1 confirmed H1.

**One PR for the whole campaign (D-LTL7).** No M/F/S/C split.

**Next, in order:**

1. Wait for the client-tls-oidc PR (#2031) to merge. Ops has the campaign PR queued behind it.
2. `ops`: merge `origin/main` into `ltl-int`. Conflicts with #2031 are expected in `shell/src/main.rs` (the `.run(..)` tail), `privacy-inventory.md` (row "Launcher forwarding") and `frontend/ui/index.html` (line 91). Give them to a fresh packet-coder, keep both sides.
3. Re-run the E2E checks and the docs checks once, then `ship.ps1 pr` (one PR; body lists the doc-update-map rows) and merge on green CI. If the shell test `retained_host_handoff_...` fails there, it is the known intermittent `Busy` flake in the ledger: rerun the job.
4. Retire the worktrees: `rm-worktree.ps1 --merged-into <PR>` for every `ltl-*` worktree once the PR merges (never `git worktree prune`).
5. The owner closes PR #2038 himself (ops cannot).

**Spares to retire:** the leftover branch `launcher-telemetry-linux/LTL-04b` (it points at an `ltl-int` merge commit; delete it with the other `ltl-*` branches). `ltl-04` is the LTL-04 packet worktree, already merged; retire it with the rest.

**After deploy, the owner runs the Deck re-run** ([LTL-U1 to LTL-U6](../../guides/unified-uat.md#launcher-telemetry-on-linux-ltl-u)) with the Windows launcher closed. Nothing has been run on a real Deck or in the lab. Ask the owner before any lab use. Record the F8 300-row budget observation (LTL-U6) and the `caller_forwarded`/`peer_loopback` fields on the Deck's mint row (LTL-U2; they also answer ops item OPS-LT2).

**Parked (not sliced):** see the ledger's Parked table (Deck movement walk, first-run `launcher_started` consent, `CombatReticule00`, Windows Lua `ScriptException`s, `os` on game-client events, new Proton launcher events).

**Rules to remember:**

- `docs/**` is CRLF. Never `git worktree prune`. Never ship one packet alone.
- Return line: `<packet> <done|blocked|failed> <sha|-> <report>`; reports in `<worktree>\.claude\reports\launcher-telemetry-linux\`.