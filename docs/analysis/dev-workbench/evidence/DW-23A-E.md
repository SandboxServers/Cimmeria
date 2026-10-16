# DW-23A-E: dot query, maintenance and meta command evidence

This is the source contract for DW-23A, not a report of live execution. The inventory is exactly the 8 + 6 + 5 `Spec` names in `crates/cell-console/src/cell/console/registry/commands/{query,maintenance,meta}.rs`. Names below include the `.` prefix; the registry does not. All commands enter through `console/chat/mod.rs`'s server-side GM access-level gate, then `dispatch.rs::handle_console_command` checks argument count and selected-target policy. Arguments are whitespace tokens (`split_whitespace`), without quoting or escaping. An accepted audit row is not a completion receipt. No disruptive command was invoked to collect this evidence.

The query family contains **no counter increment or resource reload** in its current handlers: `.bug` *reads* the caller's counters and writes telemetry, while the three searches perform base-side `SELECT`s. The six maintenance names currently provide explanatory feedback rather than a reload, log-level change or save. `.seedconfirm` is the actual live-DB maintenance operation in this source set; a seed-file commit remains a separate human action. `Target::Mob` only checks `!is_player`, and `Target::Spawnable` accepts any cell entity (`registry/mod.rs::TargetPolicy::matches`). These labels are not a proof of NPC class.

## .searchitem

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/query.rs::SPECS`; `crates/cell-console/src/cell/console/query.rs::{search_item,search}`; `crates/base-session/src/base/console_authoring/mod.rs::{handle_console_search,run_console_search}`; `crates/base-session/src/base/console_authoring/tests.rs` literal-pattern and truncation tests; `docs/commands.md` Search row.
- **Arguments:** Required name token, optional second name token; 1–2 whitespace tokens, joined with one space. No quoted multiword form; no character/length bound in the handler.
- **Target/scope:** No selected target; item designs in `resources.items` on the base's live DB.
- **Preconditions and authority:** Cell GM gate and argument count; base requires a live DB pool. The base search handler does not independently recheck GM/session generation when it receives the cell message.
- **Effects and impact:** Read-only parameter-bound, case-insensitive literal substring `SELECT`, ordered by `item_id`, at most 25 displayed hits; no counter mutation, cache reload or grant. Query and feedback incur DB/network work.
- **Verification:** Positive results send `searchitem '<query>': N match(es)` and `id: name` rows; zero hits instead send `searchitem: no matches for '<query>'`. Compare with read-only `server_db_query` on `resources.items` for the same literal substring, allowing concurrent seed changes. A truncated reply is not the full result set. Missing DB reports `no live DB connection`.
- **Compensation:** None needed for data state; query/audit log cannot be erased by a command.
- **Availability:** Disabled in the palette until the request and every base reply are fenced to the original GM session; recycled entity IDs could route results to another occupant. Also add an input-length bound before enabling.

## .searchmission

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/query.rs::SPECS`; `crates/cell-console/src/cell/console/query.rs::{search_mission,search}`; `crates/base-session/src/base/console_authoring/mod.rs::{handle_console_search,run_console_search}`; its mission literal-pattern test; `docs/commands.md` Search row.
- **Arguments:** Required name token, optional second; 1–2 whitespace tokens joined by one space, no quoting or character bound.
- **Target/scope:** No selection; `resources.missions.mission_defn` in the live DB.
- **Preconditions and authority:** Cell GM gate and count; live DB required; base receives caller entity ID without independent session-generation check.
- **Effects and impact:** Read-only, parameter-bound literal `ILIKE` `SELECT` ordered by `mission_id`, 25 displayed hits maximum. Does not mutate mission progress or counters and does not reload resources.
- **Verification:** Positive results send `searchmission '<query>': N match(es)` and `id: name` rows; zero hits instead send `searchmission: no matches for '<query>'`. Match feedback IDs/names to a read-only `server_db_query` on `resources.missions`; respect the explicit truncation flag and `no live DB connection` failure.
- **Compensation:** None for state; accepted-command audit remains.
- **Availability:** Disabled in the palette until immutable GM/session binding protects base replies; recycled entity IDs could route results to another occupant. Add an input-length bound too.

## .searchtemplate

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/query.rs::SPECS`; `crates/cell-console/src/cell/console/query.rs::{search_template,search}`; `crates/base-session/src/base/console_authoring/mod.rs::{handle_console_search,run_console_search}`; its template literal-pattern test; `docs/commands.md` Search row.
- **Arguments:** Required name token, optional second; 1–2 whitespace tokens, no quoting or character bound.
- **Target/scope:** No selection; `resources.entity_templates.template_name` in live DB, not spawned NPCs.
- **Preconditions and authority:** Cell GM gate, count, live DB. Base-side search has no independent GM/session-generation check.
- **Effects and impact:** Read-only parameter-bound literal `ILIKE` `SELECT`, ordered by `template_id`, at most 25 displayed rows. No spawn, template cache reload or counter mutation.
- **Verification:** Positive results send `searchtemplate '<query>': N match(es)` and `id: name` rows; zero hits instead send `searchtemplate: no matches for '<query>'`. Match feedback IDs/names to `server_db_query` on `resources.entity_templates`, accounting for truncation and DB-unavailable feedback.
- **Compensation:** None for state; accepted-command audit remains.
- **Availability:** Disabled in the palette until immutable GM/session binding protects base replies; recycled entity IDs could route results to another occupant. Add an input-length bound too.

## .players

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/query.rs::SPECS`; `crates/cell-console/src/cell/console/query.rs::players`; `crates/cell-console/src/cell/console/tests/p04.rs` cross-space tests; `docs/commands.md` Search row.
- **Arguments:** None, exactly zero tokens.
- **Target/scope:** No selection; all player entities already placed in loaded spaces on this CellApp service, not every connected player across the cluster or those still in transition.
- **Preconditions and authority:** Cell GM gate and caller entity; no DB requirement.
- **Effects and impact:** Read-only cell enumeration, sorted by entity ID; sends a header and one feedback line per found player. No online-presence mutation or counter increment. Potentially many feedback sends.
- **Verification:** Compare listed entity IDs, names and worlds with `server_sessions`/`server_entity_query` for the same CellApp and time; expect transition/other-service omissions. The handler's lines show names/worlds but not IDs, so correlate carefully; no exact stable snapshot across concurrent joins.
- **Compensation:** None for state; audit/feedback already emitted.
- **Availability:** Runnable with scope labeled precisely as this CellApp's placed players.

## .listabilities

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/query.rs::SPECS`; `crates/cell-console/src/cell/console/query.rs::list_abilities`; `crates/cell-console/src/cell/console/tests/p04.rs`; `crates/cell-console/src/cell/console/dispatch.rs::resolve_target`.
- **Arguments:** None.
- **Target/scope:** Required selected player in caller's current space and view; lists that player's cell-known ability IDs resolved to names, sorted by ID. Unresolved IDs appear as `<unknown ability N>`.
- **Preconditions and authority:** Cell GM gate and `Target::Player` selection check; stale, out-of-view, cross-space or non-player selections refuse before handler.
- **Effects and impact:** Read-only ability-set inspection; no learn, reset, counter mutation or resource reload. The list is sent as feedback lines.
- **Verification:** Feedback header gives target ID and ability names, with unresolved IDs numeric. `server_ability_state` exposes warmup, cooldowns, effects and stats, **not** the complete known-ability set; `server_entity_get` also omits it. No existing independent exact-list readback exists. A cell feedback list is not proof of persisted ability rows.
- **Compensation:** None for state; no way to retract already emitted feedback.
- **Availability:** Disabled in the palette until an independent known-ability-ID readback exists and binds to the same entity/session; direct command remains registered.

## .info

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/query.rs::SPECS`; `crates/cell-console/src/cell/console/query.rs::info`; `crates/cell-console/src/cell/console/dispatch.rs::resolve_target`; `crates/cell-console/src/cell/console/tests/p02.rs` inspection tests.
- **Arguments:** Zero or one `u32` `entityId`; with a current selected in-view target the selection wins and the argument is ignored, including malformed text. No quoted form.
- **Target/scope:** Selected same-space/in-view entity if present; otherwise an explicit same-space entity ID. No target gives `Could not find entity`. Any cell entity type can be inspected.
- **Preconditions and authority:** Cell GM gate; explicit ID must parse and resolve in caller's space. A current selection is validated by dispatch before reaching this handler.
- **Effects and impact:** Read-only dump of cached cell flags, template, tags, alignment/faction and level; no counter mutation or reload. It may expose entity metadata through GM feedback.
- **Verification:** Compare feedback `ENTITY ... (id)`, template ID, level, faction and alignment with `server_entity_get` for the same-space entity. That snapshot does **not** expose `entity_flags`, static-mesh text or full `interaction` flag decoding; those details remain feedback-only. Selection precedence must be shown in the form. Missing entity/malformed ID has distinct feedback.
- **Compensation:** None for state; emitted metadata/audit cannot be retracted.
- **Availability:** Runnable when subject resolution is explicit; do not silently treat an entered ID as authoritative if a selection exists.

## .facing

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/query.rs::SPECS`; `crates/cell-console/src/cell/console/query.rs::{facing,facing_angle,facing_class}`; `crates/cell-console/src/cell/console/dispatch.rs::resolve_target`; `crates/cell-console/src/cell/console/tests/p02.rs` bearing tests.
- **Arguments:** None.
- **Target/scope:** Required selected `Target::Spawnable` in caller's space/view; that policy matches every entity. Reports caller-to-target angle, class and 3D distance.
- **Preconditions and authority:** Cell GM gate, live caller and fresh selection; cross-space/out-of-view target refuses.
- **Effects and impact:** Read-only geometry calculation and two feedback lines; no facing mutation, counter increment or reload.
- **Verification:** Capture caller and target `server_entity_get.position` plus caller `direction`; compare distance and angle/class to feedback at the same moment. Moving entities make later comparisons approximate.
- **Compensation:** None for state; feedback/audit is historical.
- **Availability:** Runnable if the selection is fresh; avoid implying that `Spawnable` excludes props or pets.

## .combatinfo

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/query.rs::SPECS`; `crates/cell-console/src/cell/console/query.rs::combat_info`; `crates/cell-console/src/cell/console/dispatch.rs::resolve_target`; `docs/commands.md` Entity/combat inspection row.
- **Arguments:** None.
- **Target/scope:** Required selected non-player in caller's space/view. `Target::Mob` is `!is_player`, so a prop or pet can pass; there is no NPC-class check.
- **Preconditions and authority:** Cell GM gate and selection check; stale/out-of-view/wrong-type target refuses.
- **Effects and impact:** Read-only two-check diagnostic: missing `template_id` and empty known-ability set. A healthy target produces **no feedback**. No combat state/counter mutation or reload; weapon and ability-type diagnostics from legacy are absent.
- **Verification:** `server_entity_get.template_id` can verify the first check, but neither it nor `server_ability_state` returns the complete known-ability ID set. The second check lacks independent readback. Silence is not a complete combat-readiness certificate or execution receipt.
- **Compensation:** None for state.
- **Availability:** Disabled in the palette as a trustworthy NPC combat diagnostic until a cell-side NPC-class guard and complete readback or clearly scoped two-check form exist; the direct command remains registered.

## .save

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/maintenance.rs::SPECS`; `crates/cell-console/src/cell/console/server.rs::dispatch` `save` arm; `docs/commands.md` Server/maint row.
- **Arguments:** None.
- **Target/scope:** Caller only; selection ignored.
- **Preconditions and authority:** Cell GM gate; no persistence operation is attempted.
- **Effects and impact:** Feedback-only explanation that other mutations persist incrementally. **No full save occurs** despite the command name.
- **Verification:** Feedback text itself proves only that the explanatory arm ran; no DB write or save receipt exists for this command.
- **Compensation:** None; no state change.
- **Availability:** Hidden from action forms or disabled with an explicit no-op explanation. It must not present a Save button.

## .reloadmap

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/maintenance.rs::SPECS`; `crates/cell-console/src/cell/console/server.rs::dispatch` `reloadmap` arm; `docs/commands.md` Server/maint row.
- **Arguments:** None.
- **Target/scope:** Caller only; selection ignored.
- **Preconditions and authority:** Cell GM gate; no map reload path is called.
- **Effects and impact:** Feedback-only `not wired` message; it neither reloads a map nor teleports the caller. Relog/gate travel suggestions are separate, disruptive operations.
- **Verification:** No reload receipt or changed map state exists; `server_entity_get` continuing in the same space only confirms no move, not a reload.
- **Compensation:** None for this no-op; a separate relog/travel has its own consequences.
- **Availability:** Disabled, because the named action is unimplemented.

## .reloadres

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/maintenance.rs::SPECS`; `crates/cell-console/src/cell/console/server.rs::dispatch` `reloadres` arm; `docs/commands.md` Server/maint row.
- **Arguments:** Zero or one arbitrary token accepted by `Spec`; the current handler ignores it. No resource-ID type or bound is enforced.
- **Target/scope:** Server resource caches in the name/help, but **no cache is actually touched**.
- **Preconditions and authority:** Cell GM gate; resource caches load at startup.
- **Effects and impact:** Feedback-only message that runtime reload is not wired and a restart is needed for seed edits; no counter mutation or reload. Restart is a separate disruptive operator action.
- **Verification:** There is no cache generation or reload receipt from this command; feedback explicitly states the absence.
- **Compensation:** None for no-op; no automatic inverse for a separate restart.
- **Availability:** Disabled; do not offer a functional Reload form or treat accepted optional text as a resource selector.

## .removerespawner

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/maintenance.rs::SPECS`; `crates/cell-console/src/cell/console/server.rs::dispatch` `removerespawner` arm; `crates/cell-console/src/cell/console/dispatch.rs::resolve_target`; `docs/commands.md` Server/maint row.
- **Arguments:** Required one signed `i32` `respawnerId`; parse only, with no positivity/existence check because the parsed value is unused.
- **Target/scope:** Registry requires selected player in caller's space/view, but handler ignores target and does not edit per-player or global respawners.
- **Preconditions and authority:** Cell GM gate, selected player, integer parse; no mutation path is wired.
- **Effects and impact:** Feedback-only `not wired` message pointing to `db/resources/Worlds/Seed/respawners.sql`; no removal, cache reload or seed edit.
- **Verification:** No remove receipt; `server_entity_get` cannot establish a nonexistent respawner mutation. Feedback is a refusal/explanation.
- **Compensation:** None for no-op; editing seed rows requires normal human review.
- **Availability:** Disabled, with no invented inverse or success state.

## .loglevel

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/maintenance.rs::SPECS`; `crates/cell-console/src/cell/console/server.rs::dispatch` `loglevel` arm; `docs/commands.md` Server/maint row.
- **Arguments:** Required level token and optional category token, accepted as arbitrary strings; no enum/range validation because both are ignored.
- **Target/scope:** Intended server logging; selection ignored.
- **Preconditions and authority:** Cell GM gate; runtime tracing reload is not wired.
- **Effects and impact:** Feedback-only explanation that `RUST_LOG`/environment sets logging at startup. Does not change tracing filters, log rate or client forwarding.
- **Verification:** No runtime log-level state/readback or effect is produced; an accepted audit row is not evidence of a changed filter.
- **Compensation:** None for no-op; an environment/restart change is outside this command.
- **Availability:** Disabled; do not expose a seemingly functional logging control.

## .logclient

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/maintenance.rs::SPECS`; `crates/cell-console/src/cell/console/server.rs::dispatch` `logclient` arm; `docs/commands.md` Server/maint row.
- **Arguments:** None.
- **Target/scope:** Intended caller-specific server-to-client logging; selection ignored.
- **Preconditions and authority:** Cell GM gate; forwarding is not wired.
- **Effects and impact:** Feedback-only pointer to SigNoz/OTLP; no toggle, log stream, counter or reload. Despite `Toggle` legacy language, repeated calls change nothing.
- **Verification:** No forwarding-state readback or client log stream exists for this command; feedback says `not wired`.
- **Compensation:** None for no-op.
- **Availability:** Disabled, with no fabricated current/on/off state.

## .help

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/meta.rs::SPECS`; `crates/cell-console/src/cell/console/query.rs::{help,help_specs}` and its unit tests; `crates/cell-console/src/cell/console/registry/mod.rs::arg_specs`.
- **Arguments:** Zero or one substring token, matched case-insensitively against command names; no quoting for spaces.
- **Target/scope:** No target; full local `COMMANDS` registry or filtered subset, sorted by command name.
- **Preconditions and authority:** Cell GM gate and argument count.
- **Effects and impact:** Read-only registry enumeration; sends one feedback line per command and argument detail when result count is at most three. No counter mutation or resource reload; unfiltered output can be large.
- **Verification:** Compare returned command names to the `COMMANDS` catalog/API version and filter; unit tests assert sort, full inventory and substring matching. No server state change to read back.
- **Compensation:** None for state; feedback/audit remains.
- **Availability:** Runnable, but the companion already has the catalog and should avoid flooding chat for routine discovery.

## .bug

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/meta.rs::SPECS`; `crates/cell-console/src/cell/console/bookmark.rs::{bug,capture,emit,emit_entity}`; `crates/cell-console/src/cell/console/tests/bookmark.rs` header/entity and ability-snapshot tests; `docs/commands.md` Playtest bookmark row.
- **Arguments:** Zero to 64 whitespace tokens joined by spaces into a free-text note. No character/byte ceiling or secret scrub in the handler; quoted phrases are not preserved as a distinct syntax.
- **Target/scope:** Caller, current selected in-view same-space target if any, and up to 32 nearby same-space entities within 60 units (selected target is retained even outside radius). Captures missions, **reads** counters, journal and ability state.
- **Preconditions and authority:** Cell GM gate; caller entity must still exist or feedback says `caller entity not found`. No live client or DB requirement; telemetry sink availability is not checked before ack.
- **Effects and impact:** **Telemetry write, not ReadOnly:** emits `playtest.bookmark` with note, account/player identity and scene, `playtest.bookmark.entity` per captured entity, plus `abilities.snapshot` for caller/target. Before this handler runs, `dispatch.rs::handle_console_command` also exports accepted raw arguments through `cimmeria_discord::emit_gm_command`; `crates/discord/src/embed/format_gameplay.rs::format_gameplay` uses those arguments as the GM audit embed description. No counter increment, game-state mutation or resource reload. Free text and snapshots may contain sensitive data; 64 tokens is not an output-size bound.
- **Verification:** Match `Bookmark <id> recorded` feedback to `server_log_tail`/SigNoz rows by `bookmark_id` and target; assert header, captured entity rows and ability snapshots share that ID. Ack follows local tracing calls, not verified OTLP delivery. Tests assert emitted rows and ack, not remote retention.
- **Compensation:** No inverse for arguments already exported to Discord or for telemetry. Apply note limits and redaction before dispatch to either sink; deletion from a downstream sink is a separate operator action.
- **Availability:** Disabled for arbitrary free-text palette submission until note size/redaction is enforced before the Discord audit and bookmark telemetry sinks, and telemetry delivery disposition is defined. A note-free form remains gated on proving the pre-dispatch audit contains no private text. The direct GM command remains registered.

## .seedconfirm

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/meta.rs::SPECS`; `crates/cell-console/src/cell/console/seed.rs::{confirm,record,record_spawn}` and queue/confirm tests; `crates/base-session/src/base/console_authoring/mod.rs::handle_execute_authoring_sql`; `docs/commands.md` authoring workflow.
- **Arguments:** None; the implicit argument is **every** statement currently buffered for this caller, possibly across seed files.
- **Target/scope:** Caller-owned cell authoring buffer; sends each server-generated SQL statement to BaseApp for live DB execution and emits per-file/structured spawn telemetry and an on-disk authoring block. Does not edit repository seed files.
- **Preconditions and authority:** Cell GM gate; pending buffer required for an effect. Cell removes it before sends. Base executor trusts a cell-generated SQL message identified by entity ID; it has no independent current-GM/session-generation or per-statement approval check. DB pool may be absent.
- **Effects and impact:** Disruptive live DB writes, one per queued statement, plus telemetry/log output, clearing the entire buffer. A live write can fail or affect zero rows while telemetry says confirmed; next deploy rebuilds from seeds. `confirmed_new_spawns` is updated for queued inserts before base confirms a DB result. No atomic transaction across statements and no committed seed change.
- **Verification:** `.seedpending` returns only counts per file. The on-disk authoring log is process-run global and includes queued, canceled and previously confirmed statements from multiple callers; it cannot identify the caller's exact current buffer or serve as an approval preview. After confirmation, correlate batch ID in `authoring` telemetry, each base `authoring SQL executed`/zero-row/failure row and a read-only `server_db_query` on every affected table/key. Feedback `saved N` only proves cell emission; `live on this server now` reflects successful channel sends, not DB commit. Git seed diff/commit is the only seed durability proof.
- **Compensation:** No general inverse. Manual reviewed corrective SQL/seed edit may be needed; `.seedcancel` after confirm cannot undo sent DB writes or telemetry.
- **Availability:** Disabled in the palette until a caller-scoped API exposes the exact current SQL/file batch for preview, explicit operator confirmation, immutable GM/session fencing across the cell-to-base message and per-statement result/readback are implemented. The shared log and `.seedpending` count summary cannot replace that API. No one-click blind confirm.

## .seedpending

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/meta.rs::SPECS`; `crates/cell-console/src/cell/console/seed.rs::pending`; `docs/commands.md` authoring workflow.
- **Arguments:** None.
- **Target/scope:** Caller-owned authoring buffer only; groups pending statement counts by seed-file path, without exposing SQL text.
- **Preconditions and authority:** Cell GM gate; empty buffer is valid.
- **Effects and impact:** Read-only count/file summary feedback. No DB write, seed edit, buffer mutation, counter increment or reload.
- **Verification:** Feedback `seedpending: N statement(s)` and per-file counts; the per-session authoring log can show queued SQL but also contains earlier canceled/confirmed entries, so it is not an exact current-buffer readback. No existing structured API returns current SQL statements.
- **Compensation:** None for state.
- **Availability:** Runnable as a count summary, not a full approval preview for `.seedconfirm`.

## .seedcancel

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/meta.rs::SPECS`; `crates/cell-console/src/cell/console/seed.rs::{cancel,queue}`; `docs/commands.md` authoring workflow.
- **Arguments:** None; implicitly targets all pending statements for caller.
- **Target/scope:** Entire caller-owned cell authoring buffer, across every seed file and action; does not roll back the already-applied in-memory edits or remove the per-session SQL log.
- **Preconditions and authority:** Cell GM gate; no pending buffer is a valid zero-discard result.
- **Effects and impact:** Destructive buffer removal, with `discarded N` feedback. No live DB or committed seed write is undone; telemetry/file log written at queue time remains. A previous `.seedconfirm` is unaffected.
- **Verification:** Capture `.seedpending` before, then require `seedpending: no pending changes` after; feedback count gives the removed statement count. There is no exact SQL snapshot from the pending command, so the UI cannot prove which statements were discarded from counts alone.
- **Compensation:** No automatic restore of the buffer or in-memory edits; recover SQL manually from the per-session log and reauthor intentionally.
- **Availability:** Disabled in the palette until exact whole-buffer preview and explicit discard confirmation exist. The direct GM command remains registered.
