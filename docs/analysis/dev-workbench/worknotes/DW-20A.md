# DW-20A worknote — dot travel and spawn metadata

The reviewed DW-20A-E evidence covers exactly eight travel/placement and
eight spawn commands. This packet adds typed arguments, exact accepted forms,
preconditions, impact, readback and compensation in the registry and projects
them through `server_console_list` for the local companion palette.

The packet's original ownership omitted the existing catalog seam. The scope
was corrected to include only `registry/{mod,workbench}.rs`, `console/mod.rs`
and `crates/lab-mcp/src/tools/console.rs` to register and project the metadata.
The DW-20A paragraph in `work-packets.md` records that correction.

Three entries are marked runnable: `.gotolocation`, `.spawn`, `.despawn`.
`.gotoxyz` has typed metadata but is disabled until an
execution-time expected-subject gate can preserve its optional selection
without silently falling back to the caller. Its impact is persistent for a
player: base attempts an `sgw_player` position write and sends
`BASEMSG_FORCED_POSITION`; both require separate readback. `.despawn` selected
form can remove a pet, while its explicit ID form refuses pets. `.delspawn`
is disabled until a server-side player guard runs before `record_spawn` and a
regression proves a seeded-looking selected player survives with no pending
SQL. The remaining eleven entries are also disabled in the companion catalog.
Their evidence requires server repairs or a conditional execution gate beyond
this packet: `.goto`, `.summon`, `.gotospace` (subject-world GM
policy and exact arrival), `.location`, `.rotation`, `.movehere`, `.savespawn`,
`.autosavespawn` (pet class guard and player orientation), `.respawnall`
(canonical respawn/grid/witness lifecycle), `.spawnset` (peer teardown before
target validation) and `.spawnrandom` (server bounds and derived-position
validation). The in-game dot console behavior remains as documented in the
evidence; catalog disabling is a palette disposition, not a server refusal.

The command-level availability flag also disables two safe subforms:
`.autosavespawn 0|false` and `.spawnset` with no arguments or `list`. Their
metadata names them safe and deferred to DW-29/30 for per-form availability;
the rest of those commands remain disabled. `.delspawn` targets a selected
seeded spawnable with a
template and `spawn_id`, not only an NPC. `.despawn` readback captures
`spawn_id` before runtime removal and uses a read-only query of that exact
`resources.spawnlist` row when one exists; a null entity alone does not prove
the persistent row remains. `.delspawn` queues its DELETE before attempting
runtime removal; the primitive can still refuse a player, so that path leaves
a pending SQL change to inspect before confirmation.

`.gotolocation` remains runnable in the catalog with a conditional readback:
same-space player snaps require a `wire.out.forced_position` `send=sent` row from
`server_log_tail`; cross-space arrival requires destination re-entry, a
matching `sgw_player` row and inspection of GateTravel persistence log
warnings/errors (`rows_affected_zero` or `persist_arrival_failed`). Missing
evidence means unknown. Explicit coordinates pass finite checks
only; navmesh and Z containment are not validated, so geometry risk is shown
before execution.

Existing handler tests cover wrong world/target refusal and same-space
travel's position and forced-position output. This packet's tests pin exact
family coverage, the three runnable contracts and disabled-form projections. The
companion GUI forms and execution flow are pending DW-28 and DW-29. No live
client or lab lease was used.

Validation note: the sandboxed PowerShell lane saw a virtual LOCALAPPDATA and
reported two occupied slots indefinitely. Re-running the same command against the
shared machine-wide lane completed normally. Compiling checks used that shared lane.

Checks through the shared lane: cell-console fmt/clippy status=ok and
557/557 lib tests passed; lab-mcp fmt/clippy status=ok and 60/60 lib tests
passed. No new SQL query was added, so no live-DB check was required.
