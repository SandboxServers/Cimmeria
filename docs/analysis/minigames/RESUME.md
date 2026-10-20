# Minigames resume card

> Ledger: [README.md](README.md). Packets: [work-packets.md](work-packets.md) and [packets/](packets/). Closed 2026-10-20 (MG-99).

## State

**Phase closed.** Every packet is merged into `minigames/integration` (worktree `mg-integration`), the campaign review passed after two fix passes, and the status docs changed once in MG-99. Nothing is pushed.

- Ported and server-validated: CrystalGame, Alignment, GoauldCrystals, ConverseBasicHumanoid, plus Livewire. Client-declared and hardened: Hack, Activate, Analyze, Bypass, Converse.
- Not applicable: MG-23, MG-25, MG-26, MG-65. MG-63 was replaced by MG-63a and MG-63b.
- No lab run happened. Lab use stays ask-first (D-MG16).

## Next action

1. Ops: open one PR from `minigames/integration` (the ship skill), list the doc-update-map rows in the body.
2. After it merges: retire every packet worktree and `mg-integration` (`rm-worktree.ps1 --merged-into <PR> <name>...`), delete branch `backup/mg-al3a-rules-pre-reset`, empty `.claude/state/active-campaign`, post the board handoff.

## Owner list

The full list, with the reasons, is [README § Open owner items at close-out](README.md#open-owner-items-at-close-out). In short: D-MG11 default on after lab UAT; an offer path for mission 582 (M10); lab UAT of the terminals (placement, click, game window); the social layer; deferred authority finding 3; and two files to split (`listener_tests.rs` at 692 lines, cell-world `entities.rs` at 700), plus session.rs (694), tests.rs (690) and session_tests.rs (674) in crates/minigame to watch.
