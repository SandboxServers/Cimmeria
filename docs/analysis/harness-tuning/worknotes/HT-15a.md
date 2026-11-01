# HT-15a worknote (session 1 of at most 6)

Folder: `.claude/agent-memory/rust-gameserver-dev/`. Commits: part 1 d6f3a4c94, part 2 61335d3fc (branch ht/HT-15a-trim).

Lint for the folder, before: no_type 27, state_no_invalidated 35, unindexed 193.
Lint for the folder, after session 1: no_type 13, state_no_invalidated 11, unindexed 193.

Files done (38): all state/no_type flags up to and including the alphabetical range ai-state... through tell-channel..., except those listed under Next.
Counts: (a) about 27, (b) 4 (combat-debug-notes, gitignore-swallows, launcher-self-update-handoff, launcher-state-key), (c) 0, (d) 0 left unverified-only (cargo-test and discord got `checked: unverified` plus an INVALIDATED WHEN line). About 14 files also got a missing `metadata: type:`.
Deleted files: none.

## Session 2 (continuation 2)

Commit: part 3 (see git log). Lint for the folder after session 2: no_type 0, state_no_invalidated 0, unindexed 193 (unchanged). Whole-repo summary line: memory_lint files=777 no_type=41 state_no_invalidated=92 index_over=1 index_missing=1 unindexed=203 orphan_folders=18 ip_like=4.
Counts this session (24 flagged files): (a) 19 reclassified or given frontmatter (13 no-frontmatter files got name/description/type; 6 project to reference), (b) 2 trimmed (build-environment lost the obsolete rust-lld section and the stale Cargo.lock section; fixed in place), (c) 0, (d) 4 kept as project with an INVALIDATED WHEN line, checked unverified (castle-staticmesh-coverage, edge-trigger-replay-and-abandon, navmesh-recast-and-castle-topology, ring-rig-clone-to-new-map). Local user paths scrubbed from concurrent-claude-sessions and tooling-filter-and-path-traps. Deleted files: none.
Next: no no_type or state_no_invalidated flag left in this folder. Only the 193 unindexed flags remain (needs the decision below).

Open items:
- 193 `unindexed` files: MEMORY.md is 155 lines / about 19 KB (cap 200 lines, 25000 bytes), so adding one line each does not fit. They are mostly listed in the topic sub-indexes; needs a decision (lint checks only MEMORY.md).
- `concurrent-claude-sessions.md` and `tooling-filter-and-path-traps.md` contain local user paths; scrub when those files are reached.
- Several of these notes cite stale claims were fixed in place (duel scan test renamed, damage_apply/mod.rs line count, .gitignore server/ rule gone).


## Session 3 (ruling 1, part 4)

Applied notes 1-5 of HT-15a-ruling-1 with a scratchpad script: appended minigame section moved to `.claude/agent-memory/rust-gameserver-dev/minigame-session-lifecycle.md` (type reference, 4 bullets verbatim, CRLF) with one index line under Cell systems; stray top line moved under Testing patterns; every index line converted to the bare `- x.md — hook` form; 8 `Files` lines added (tooling-quirks 22, launcher 11, injected-client-dlls 16, ue3-and-navmesh 18, seeds-and-content-chains 24, cell-systems 64, observability 19, testing-patterns 19 = 193 names, each once, all verified to exist on disk); header sentence rewritten.
Lint for the folder, before: unindexed 193. After: no line for rust-gameserver-dev. Summary: unindexed 203 -> 10, index_over unchanged at 1 (other folder).
MEMORY.md before: 155 lines, 19,356 bytes. After: 157 lines, 22,236 bytes (gates: <=180 lines, <=23,500 bytes). Pure CRLF checked for MEMORY.md and the new file. Unit tests green.
Packet-wide counts (sessions 1-3): (a) 46, (b) 6, (c) 0, (d) 4. Deleted files: none.
## Review fixes, pass 1

N1-N7 applied (code re-verified at HEAD: BeforeTravelSend hook, patch 012 present, range.rs/cast_range.rs, view.test.ts list). No rejections. Lint: memory_lint files=778 no_type=41 state_no_invalidated=92 index_over=1 index_missing=1 unindexed=10 orphan_folders=18 ip_like=4. unittest discover green.
