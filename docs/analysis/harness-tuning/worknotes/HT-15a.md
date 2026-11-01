# HT-15a worknote (session 1 of at most 6)

Folder: `.claude/agent-memory/rust-gameserver-dev/`. Commits: part 1 d6f3a4c94, part 2 61335d3fc (branch ht/HT-15a-trim).

Lint for the folder, before: no_type 27, state_no_invalidated 35, unindexed 193.
Lint for the folder, after session 1: no_type 13, state_no_invalidated 11, unindexed 193.

Files done (38): all state/no_type flags up to and including the alphabetical range ai-state... through tell-channel..., except those listed under Next.
Counts: (a) about 27, (b) 4 (combat-debug-notes, gitignore-swallows, launcher-self-update-handoff, launcher-state-key), (c) 0, (d) 0 left unverified-only (cargo-test and discord got `checked: unverified` plus an INVALIDATED WHEN line). About 14 files also got a missing `metadata: type:`.
Deleted files: none.

Next (still flagged no_type/state_no_invalidated): build-environment, castle-staticmesh-coverage, cell-entity-direction-semantics, cell-startup-caches-vs-base-roundtrip, content-chain-authoring-traps, content-chain-condition-context-gaps, content-chain-dispatch-traps, cross-world-transfer-flow, db-test-revert-verification, debug-area-plaza-and-training-dummies, destroy-entity-vs-despawn-npc, dialog-button-strip-and-seed-agreement, dialog-set-bind-routing-and-edges, edge-trigger-replay-and-abandon, legacy-command-parity-scoping-judgment, navmesh-recast-and-castle-topology, python-write-mangles-utf8-and-crlf, resuming-a-dead-workers-wip, revert-verification-loses-uncommitted-fmt, ring-rig-clone-to-new-map, ring-transport-fsm, services-split-extraction-traps, session-scoped-cell-state-hooks, stargate-address-book-three-legs.

Open items:
- 193 `unindexed` files: MEMORY.md is 155 lines / about 19 KB (cap 200 lines, 25000 bytes), so adding one line each does not fit. They are mostly listed in the topic sub-indexes; needs a decision (lint checks only MEMORY.md).
- `concurrent-claude-sessions.md` and `tooling-filter-and-path-traps.md` contain local user paths; scrub when those files are reached.
- Several of these notes cite stale claims were fixed in place (duel scan test renamed, damage_apply/mod.rs line count, .gitignore server/ rule gone).
