# Launcher UAT fixes: follow-up resume card

> **Type:** Reference · **Audience:** Resuming engineers and owner · **Updated:** 2026-10-23
> **Companions:** [Ledger](README.md), [main layout design](design-main-layout.md), [UAT](../../guides/unified-uat.md#launcher-uat-fixes)

**Baseline:** U1-U5 (#1984–#1988) and close-out #1989 are merged.

| Task | State | Next action |
|---|---|---|
| Direct-root adoption | #1992 merged `16ff568b8`; retired | Owner reports successful adoption in 2026.10.2302; other AIP rows remain pending |
| Repeated signed releases/content logs | #1991 merged `7be8413b0`; retired | Complete; D-LX15 approved |
| Path display/startup update | #1993 merged `7459ef2cc`; retired | Automated native/package CI passed; PDU packaged acceptance pending |
| Launcher CI speed | #1994 merged `380e904`; retired | Owner-authorized release 2026.10.2303 completed from this revision |
| Wider CI iteration | #1995 merged | See [CI design](../ci-iteration-speed.md); separate from this UI follow-up |
| Main-page layout/branding | Implementation in progress; no PR yet | Finish tests, mutation proof, logic UAT, visual check and fresh ship-pr review |

**Current worktree:** `.claude/worktrees/launcher-main-layout`, branch `fix/launcher-main-layout`.
**Design:** main Play page holds saved patch/diagnostics controls and expandable native client changes. Settings retains maintenance. Native readiness hides setup buttons; completed adoption must not show unfinished compatibility text. Adoption selects SGW.exe and confirms the resolved root before changes. No Patch Notes tab. Cimmeria branding preserves legacy disk/asset identities.
**Limits:** no new packaged picker/layout, actual restart, live game, Wine or SigNoz acceptance. ML-U1..U6, PDU and unattested AIP/historical rows remain pending. Ask before laptop/lab use.
**Approvals:** Windows/macOS adopt at selected root; explicit per-file changes/backups allowed, bulk relocation/copy prohibited. D-LX15 approved. No further launcher release authorized after 2026.10.2303.
**Rules:** preserve others' edits; no Plan fields; file caps; PowerShell only; compiling Cargo through lane; ship-pr and retire after merge; never git worktree prune.
