# Launcher UAT fixes: follow-up resume card

> **Type:** Reference · **Audience:** Resuming engineers and owner · **Updated:** 2026-10-23
> **Companions:** [Ledger](README.md), [timing design](design-phase-timing.md), [SGC session review](worknotes/sgc-session-review-2026-10-23.md)

**Baseline:** U1-U5 (#1984–#1988) and close-out #1989 are merged.

| Task | State | Next action |
|---|---|---|
| Direct-root adoption | #1992 merged `16ff568b8`; retired | Owner reports successful adoption in 2026.10.2302; other AIP rows remain pending |
| Repeated signed releases/content logs | #1991 merged `7be8413b0`; retired | Complete; D-LX15 approved |
| Path display/startup update | #1993 merged `7459ef2cc`; retired | Automated native/package CI passed; PDU packaged acceptance pending |
| Launcher CI speed | #1994 merged `380e904`; retired | Owner-authorized release 2026.10.2303 completed from this revision |
| Wider CI iteration | #1995 merged | See [CI design](../ci-iteration-speed.md); separate from this UI follow-up |
| Main-page layout/branding | [PR #1996](https://github.com/SandboxServers/Cimmeria/pull/1996) merged `b7364c5d0`; retired | All six desktop CI jobs and full merge queue passed; ML packaged acceptance pending |
| Embedded Windows payload | #1997 merged `bff3b3d`; retired | Windows/Mac 2305 released from `e102a21`, run 39995348002 success; EP acceptance pending |
| Adoption/Play phase timing | [Validation](worknotes/phase-timing-validation.md): 1,178 standalone passed, 11 ignored; mutations and fresh review passed | Open PR, complete CI/merge queue, merge and retire; not yet merged or released |
| Early SGC session review | [Anonymized worknote](worknotes/sgc-session-review-2026-10-23.md) | Partial playthrough; reproduce prioritized reports separately |

**Implementation worktree (retire after merge):** `.claude/worktrees/launcher-phase-timing`, branch `feat/launcher-phase-timing`, base `e102a21`.
**Design:** main Play page holds saved patch/diagnostics controls and expandable native client changes. Settings retains maintenance. Native setup_complete hides setup buttons independently of Play admission; Settings keeps Check Play status with adjacent feedback; completed adoption must not show unfinished compatibility text. Adoption selects SGW.exe and confirms the resolved root before changes. No Patch Notes tab. Cimmeria branding preserves legacy disk/asset identities.
**Limits:** no new packaged picker/layout, actual restart, live game, Wine or SigNoz acceptance. ML-U1..U6, PDU and unattested AIP/historical rows remain pending. Ask before laptop/lab use.
**Timing contract:** existing adoption/launcher.launch targets forward as client.native; read JSON fields, not launcher.summary. Bounded native durations/correlation/counts only; no Plan/schema fields, new destination, changed consent or quotas. Old rows cannot reconstruct missing durations.
**Approvals/scope:** Windows/macOS adopt at selected root; explicit per-file changes/backups allowed, bulk relocation/copy prohibited. D-LX15 approved. Earlier release GO fulfilled by 2305; no new release requested. Gameplay fixes, deployment and live/lab reproduction are outside this telemetry change and were not performed.
**Rules:** preserve others' edits; no Plan fields; file caps; PowerShell only; compiling Cargo through lane; ship-pr and retire after merge; never git worktree prune.
