# Harset coordinator resume (session 4, written 2026-09-19 afternoon)

Read this first. Older handoffs: [session-3-resume.md](session-3-resume.md) (wave 2 history), [session-2-resume.md](session-2-resume.md) (PR #662 review), [session-1-resume.md](session-1-resume.md). Method for this phase: [../placements/METHOD.md](../placements/METHOD.md). Ledger: [../work-packets.md](../work-packets.md). Operator guide: [../../zone-restoration-operator-guide.md](../../zone-restoration-operator-guide.md).

## State of the campaign

- PR #662 (wave 1) merged as `f661a97d` and PR #682 (wave 2: playtest fixes, advisory navmesh, address grant, dial and more) merged as `3d3385bd`, both by the owner. Castle CA00-CA10 and Cellblock C00-C08 are merged. Navmesh pipeline PR #683 is merged; `resources.worlds.navmesh_mode` (H53) is on `main`; world 57 is seeded `advisory`.
- `content/harset-wave2` and `content/harset-rebuild` are finished; do not merge `main` into them.

## Owner decision (2026-09-19, said directly in the coordinator session)

Stop waiting for the M0 in-client placement walk. Place everything M0 blocked from map data and navmeshes ("lean into the new tools"), label each coordinate with an evidence class and confidence, keep ONE list of guesses so the owner can correct them in one pass after a playtest, and at the end surface, specifically, the things there is no evidence for. This supersedes the "no coordinate without an in-game pin" wording of D-H15 for anything not walked by the owner.

## What was built this session

- Branch `harset/placement` (worktree `.claude/worktrees/harset-placement`, based on `origin/main` at 447ddf9d) holds `placements/METHOD.md` (evidence classes, tools, verified coordinate facts, Castle lessons, the landmark table) and `placements/data/` (archetype-census TSVs for all four Harset maps, telemetry probe file).
- Three worker branches off it, each in its own worktree (all with an `external` junction; REMOVE the junction with `cmd /c rmdir` BEFORE `git worktree remove`, or the real `external/` is at risk): `harset/placement-A` (`harset-pl-A`: gate-3 arrival pin, respawner rows 20/22/23, ring pads, chain 6007 and the 6511-6513 / 1361 abandon twin / 6331 enables, Market and Storage doors), `harset/placement-B` (`harset-pl-B`: world 57 population and named regions), `harset/placement-C` (`harset-pl-C`: worlds 68/69/70 population, interior regions, encounter-anchor candidates). Agents `harset-place-A/B/C` (rust-gameserver-dev, default model) were launched at about 15:30 CDT; each writes `placements/<X>-*.md` in the ledger row format and commits per item, no push.

## Key evidence findings (all reproducible from METHOD.md)

- The maps have NO NPC spawn markers (those lived in the lost server DB). They DO have semantic prefab mesh names (`GA-Bank00`, `JF-*` Jaffa buildings, `EM-*` Earth Military, `GA-MerchantTent*`, three `GA-Tow*` shield towers, `GA-GuardPost00`, `HP-Brazier00` palace quarter), TriggerVolumes (generic names), InterpActor doors, and 274/353 `SGWSpecCoverNode` actors in Market/StorageRm. Landmark table is in METHOD.md.
- Calibration passed: all 33 telemetry probe points that were checked lie on `harset.nav`; the gate row is 4.6 m off-mesh and about 2 m above the plaza floor; the Command Center respawn (0, 0.355, -20) sits on a floor at Y about 0.
- The Castle-nav session (cimmeria-79) supplied rebuilt meshes (`...\navmesh\harset\Harset\mse13.nav`, 374 components vs 1,939) that lose 12 real player positions: use as a reachability second opinion only. Its cleaned telemetry files: `harset_last_valid_probes.txt`, `harset_storagerm_last_valid_probes.txt`, `harset_suspicious_points.txt`. About 77% of the "131k Harset rejects" are one entity parked at last_valid (0,0,0): a bug worth a ticket.

## Next actions

1. Collect the three workers (`git -C .claude/worktrees/harset-pl-<X> log --oneline harset/placement..HEAD`, `git status --short`). Anything uncommitted is finishable by a fresh `rust-gameserver-dev` agent from the prompts' scope lists above.
2. Merge A, then B, then C into `harset/placement` (expect trivial conflicts in seed files and `chain_replay_tests/mod.rs`; keep `entity_templates` `setval` at 248 or higher). Validate on a private DB (`reload-db.sh` from the worktree): fmt, clippy `-D warnings`, content-engine + entity + services live-DB suites.
3. Assemble the single ledger `placements/README.md` from the three worker files, with a top section "NO IDEA: things I could not place". Update the operator guide's Harset section (what is now placed, how to check each guess in-client, how to correct it).
4. Open a PR `harset/placement` -> `main`. Merge only on the owner's explicit go-ahead in the coordinator session (peer-relayed go-aheads are not enough).
5. Not started and not authorized by the placement instruction: authoring the blocked mission packets (H23-H28, H32-H37, H42-H46) and H05. They need these placements first; report their status when the ledger is done.
