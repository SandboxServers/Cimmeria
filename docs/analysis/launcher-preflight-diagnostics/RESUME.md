# Resume: launcher preflight and actionable failure reporting

Ledger: [README.md](README.md). Packets: [work-packets.md](work-packets.md), [work-packets-display.md](work-packets-display.md), [work-packets-mac-extras.md](work-packets-mac-extras.md), [work-packets-wave3.md](work-packets-wave3.md). Owner rulings: [OWNER-DECISIONS.md](OWNER-DECISIONS.md) (D1-D4). D4 feasibility: [research-mac-extras.md](research-mac-extras.md).

## State (2026-10-24)

- Designed and revised for D3/D4 and the Mac host. Nothing dispatched, no worktrees, no board subcategory. Baseline `main` `2019922fe`.
- Ready now: wave 1 (PD-22, PD-01, PD-02), wave D (PD-23), wave M's PD-38 (RE only). PD-41 is independent and starts any time after wave 1.
- Waves: 1 -> 2 -> 3; D beside 1-2; M (PD-28, PD-31..35, PD-38) after D, beside 2-3; 4 (PD-27, PD-29, PD-30, PD-36, PD-37, PD-41); 5 gated (PD-19, PD-20, PD-39, PD-40); close-out PD-21.

## Next actions (orchestrator)

1. Ask the owner D-PD5 before PD-05 is dispatched. The other questions below can wait.
2. Hands-on session: the skill's § 0 peer check, the board campaign entry, then write `.claude/state/active-campaign`. Use `ops`' bash twins; if the board step has no bash twin, skip it and note that here.
3. Worktrees `pd-22`, `pd-01`, `pd-02` and `pd-23` with `bash tools/build-lane/mk-worktree.sh`. Dispatch one `hands-on` session per packet, in one message, each told to play the packet-coder role (R1). Dispatch PD-38 to `game-archaeology-specialist` at the same time.
4. Ship PD-22 and PD-01 with `ship.sh` as soon as their review loops say `ship` (hotfixes).

## Which host runs what (D-PD19)

- Everything is built on this Mac: `claude --agent hands-on` in the packet-coder role, `bash tools/build-lane/lane.sh`, and the `tools/build-lane/*.sh` twins for git and shipping. Reviews run as fresh hands-on sessions given the packet-reviewer brief (D-PD25).
- These need a CI job before they are `done`, because the Mac can't compile their Windows code:
  - WG (`native-engine (windows-latest)`): PD-07, PD-10, PD-20, PD-24, PD-29, PD-31, PD-36, PD-41.
  - RG (`runtime-probe`) and WR (release dry run, Windows): PD-25, PD-33.
- MG (the `native-engine (macos-latest)` CI job plus local `lane.sh`): every packet whose README Verify column names it.
- These need a live Mac check: PD-30, plus UAT-PD1-4, UAT-PD7, UAT-PD8 (PD-27), UAT-PD10 (PD-40), UAT-PD11 and UAT-PD12 (PD-38).
- These need a Windows PC: UAT-PD5, UAT-PD9, UAT-PD11 (Windows half), UAT-PD13, and the PD-41 install check. UAT-PD6 runs on both.

## Owner questions: all resolved (2026-10-24, D5 in OWNER-DECISIONS.md)

The owner accepted every recommended default:

- D-PD5 `os_error` integer in local events: **yes** (PD-05 unblocked).
- D-PD4 summary wire codes: **map onto the existing 18 values**, with no wire change (PD-19).
- D-PD6 Windows VC++/DirectX checks: **deferred** (PD-20 stays gated until an incident).
- D-PD22 cursor lock: **RE first (PD-38)**. The client-patch hook (PD-39) is pre-approved only if PD-38 finds no in-game switch.
- D-PD23 Wine RetinaMode: **off by default**; an opt-in experimental switch after UAT-PD10 (PD-40).
- The flagged items stand: Windows also defaults to windowed 1024x768 (D-PD14), `resolution` is left unwritten (D-PD21), and the frame cap is Mac only (D-PD20).

## Ledger (2026-10-24, autonomous mode)

The owner ordered fully autonomous execution: use best guesses and don't wait for the owner. Stand-in: `general-purpose` agents briefed with the packet-coder or packet-reviewer role, plus the bash lane and git twins (these agents run on darwin).

| Packet | Worktree | State |
|---|---|---|
| PD-22 | .claude/worktrees/pd-22 | review-1 ship (5cf2ba66e); PR #2004 open, CI pending; merge gated on UAT-PD7 (live Mac) |
| PD-01 | .claude/worktrees/pd-01 | coder dispatched |
| PD-02 | .claude/worktrees/pd-02 | coder done c380033e6; review-1 dispatched |
| PD-23 | .claude/worktrees/pd-23 | coder dispatched |
| PD-38 | (none, RE) | review-2 **ship** (cursor=none). PD-39 re-specced (`crates/client-patches`, D5 approves the hook). Next: campaign-docs PR (this folder plus PD-38 outputs plus agent-memory `client-display-knobs.md` and `client-mouse-lock-pd38.md` and `MEMORY.md`). |

## Dispatch constraint (orchestrator)

The orchestrator persona must not dispatch `hands-on` for packet work, and packet-coder, ops and architect refuse darwin. So the owner launches `claude --agent hands-on` sessions (one per packet worktree), each briefed with that packet's section and the packet-coder role. PD-38 (RE) can still go to `game-archaeology-specialist` from the orchestrator.
