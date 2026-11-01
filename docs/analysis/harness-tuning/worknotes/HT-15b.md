# HT-15b worknote

Folders: `.claude/agent-memory/main-session/` and the 18 orphan folders the lint reported.

## Lint (whole-repo summary line)

- Before: `memory_lint files=777 no_type=68 state_no_invalidated=127 index_over=1 index_missing=1 unindexed=203 orphan_folders=18 ip_like=4`
- After: `memory_lint files=768 no_type=32 state_no_invalidated=119 index_over=0 index_missing=0 unindexed=199 orphan_folders=0 ip_like=1`
- No flag is left for `main-session/` or any orphan folder. The remaining counts belong to other folders (HT-15a, HT-15c).

## Counts for main-session (45 flagged files; over the 40-file session cap)

- (a) verified state: 0.
- (b) rewritten as method: 1 (`reference_launcher_implementation_plan.md`, a 500-line progress log cut to a pointer plus traps).
- (c) deleted: 0 in main-session.
- (d) typed, given `INVALIDATED WHEN:` and `checked: unverified 2026-11-01` without code verification: 44.
- Four `ip_like` values were replaced by prose (3 files); `unindexed` fixed for the 4 old files and the 3 new ones.

## Orphans

All 18 orphan folders were removed. Their content was folded into three new main-session files:
`reference_launcher_desktop_native_traps.md`, `reference_wine_guest_app_identity.md`, `reference_lab_route_and_livewire_traps.md` (all marked unverified).

Deleted paths (folders, 28 files): `.claude/agent-memory/{adoption-ui,cellblock_route,effective-settings,launcher-game-update,launcher-launch-engine,launcher-mac-graphics-fix,launcher-migration,launcher-migration-ui,launcher-play-integration,launcher-signed-updater,launcher-update-uat,launcher-updater-minimum,livewire-atlas,native-adoption,published-adoption,updater-apply,updater-handoff-fix,wine-computer-use}/`.

## Index

`main-session/MEMORY.md`: 210 to 170 lines, 36 KB to 24 KB (the lint also caps bytes at 25000), one line per file. Hooks were truncated to about 60 characters; the full description is in each file's frontmatter.

## Next

Nothing left in these folders. A later pass could verify the 44 unverified files against code.