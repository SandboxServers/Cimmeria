# Launcher UAT fixes: follow-up resume card

> **Type:** Reference · **Audience:** Resuming engineers and owner · **Updated:** 2026-10-23
> **Companions:** [Ledger](README.md), [legacy PhysX design](design-legacy-physx.md), [timing validation](worknotes/phase-timing-validation.md)

**Baseline:** U1-U5 (#1984–#1988) and close-out #1989 are merged.

| Task | State | Next action |
|---|---|---|
| Direct-root adoption | #1992 merged `16ff568b8`; retired | Owner reports success in 2026.10.2302; other AIP rows pending |
| Repeated signed releases/content logs | #1991 merged `7be8413b0`; retired | Complete; D-LX15 approved |
| Path display/startup update | #1993 merged `7459ef2cc`; retired | PDU packaged acceptance pending |
| Launcher CI speed | #1994 merged `380e904`; retired | Released in 2026.10.2303 |
| Wider CI iteration | #1995 merged | See [CI design](../ci-iteration-speed.md) |
| Main-page layout/branding | #1996 merged `b7364c5d0`; retired | Native/package CI and full queue passed; ML packaged acceptance pending |
| Embedded Windows payload | #1997 merged `bff3b3d`; retired | Windows/Mac 2305 released; EP acceptance pending |
| Adoption/Play phase timing | #1998 merged `27493b05ae28dd8421419f1765b2982be3b3c04f`; retired | Windows/Mac 2306 released, run 40014376577 all green; no new live receipt |
| Early SGC session review | [Anonymized worknote](worknotes/sgc-session-review-2026-10-23.md) | Partial playthrough; reproduce prioritized reports separately |
| Legacy PhysX offer | Implemented; 1,186 native passed, 13 ignored; original six mutations/builds/clippy and prior review passed | PR #1999 open; privacy scope reviewed ship; final-head CI, then merge/retire and release |
| Development catalog key | Implemented; native policy/error tests, shared UI and regression proof passed | Included in final combined checks; packaged acceptance pending |
| Verified archive starts Install | Pre-privacy scope: 1,186 native/281 frontend passed, 13 ignored; six restored mutation guards; fresh review ship | PR #1999 open; privacy checks/builds/review passed; final-head CI before merge |
| Exhaustive privacy disclosure | Offline eight-section UI; 294 frontend passed, TS/build, JS REPL/browser fixture and restored field-omission mutation passed; [inventory](../../../crates/launcher/desktop/docs/privacy-inventory.md) | Native asset build passed before source-review wording corrections; corrected-head 294 tests/build/REPL and native asset build passed; fresh final review ship; CI pending |

**Active worktree:** `.claude/worktrees/launcher-legacy-physx`, branch `feat/launcher-legacy-physx`, from current main after #1998.
**PhysX design:** native Windows main-page offer when exact 2.6.3 core is missing; path-free install_legacy downloads pinned NVIDIA 9.13.0604 MSI only after a press. Reuse interactive elevated msiexec and native post-install core check; Settings retains original installer. No automatic installation or Play-policy change; Mac Wine flow unchanged.
**Catalog key:** Debug default test key refuses public fetch before networking; explicit release public verification key supports development builds. Local fixtures and signature/archive checks stay intact; archive key/network/verification errors are distinct. Use published ZIP or rebuild, never a private signer.
**Archive start:** Explicit verified selection starts only an eligible fresh Install through native status/admission. Optional response-only picker_cancelled distinguishes cancellation with an old choice. Inspect/reopen/failure/lost reply/disposal/existing or owned installs never start it; source RAR unchanged.
**Acceptance:** AS-U1..U3, LP-U1..U4, ML, PDU, EP and unattested AIP/historical rows remain pending. No real MSI/UAC, registry change, game/Wine, lab or player acceptance for the PhysX change. Ask before lab/client use or local prerequisite installation.
**Timing:** client.native replay carries client.launcher.adoption/client.launcher.launch; inspect JSON attributes_string['fields']. Bounded durations/correlation/counts; no Plan/schema fields, new destination, changed consent or quotas. Earlier rows cannot reconstruct missing durations.
**Approvals:** Windows/macOS adoption stays at selected root; explicit per-file changes/backups allowed, bulk relocation prohibited. D-LX15 approved. Standing owner instruction is PR, merge and launcher release after merge; fulfilled through 2306, applies to this follow-up. Gameplay fixes and deployment remain outside this task.
**Rules:** preserve others' edits; no Plan fields; file caps; PowerShell only; compiling Cargo through lane; ship-pr and retire after merge; never git worktree prune.
