# Named Telemetry Work Packets

> Type: how-to (packet specifications). Audience: the coordinator and packet workers.
> Updated: 2026-10-04. Companions: [README.md](README.md) (goal, decisions, ledger), [instrumentation-discipline.md](../../architecture/instrumentation-discipline.md), [discord-notifications.md](../../architecture/discord-notifications.md), [TESTING.md](../../../TESTING.md).

## Dispatch rules

- **One PR per packet.** The ledger in [README.md](README.md#ledger) is the status of record; update it in the packet's own PR.
- **Worktrees and builds.** Each worker gets its own worktree (`tools/build-lane/mk-worktree.sh`). Every compiling `cargo` call goes through the build lane. Iterate with `-p <crate>`.
- **No behaviour change.** Sweep packets add log fields and Discord fields only. A sweep that changes game behaviour, a wire byte or a metric label is out of scope and goes back.
- **Hot paths.** Resolve a name inside the branch that logs, never at function entry (Rule 5 § "Resolve late"). If a call site needs more than one map lookup, the reviewer checks the path isn't per-tick.
- **Tests.** Each converted subsystem gets at least one capture-layer test asserting the pair is present on its most-read event, and that fails with the name field removed. TESTING.md has the capture-subscriber pattern (`identity_propagation.rs` is the model).
- **Ratchet.** Once NT-03 lands, every sweep shrinks the baseline file in the same PR. A sweep never grows it.
- **Docs.** List the [doc-update-map](../../agents/doc-update-map.md) rows you touched in the PR body. Field-name changes update the field catalog in [negative-logging-convention.md](../../architecture/negative-logging-convention.md).

---

## Phase 0: Foundations

### NT-00 Rule 6 and the canonical key table

**Depends:** none. **Effort:** S. **Agent:** documentation-writer.

**Scope.** Add **Rule 6 — Every ID field is paired with its name** to [instrumentation-discipline.md](../../architecture/instrumentation-discipline.md). It covers:

- The pairing contract: same prefix, ID kept, name next to it, `Option` left out when unresolved, never `"unknown"`, `""` or `"None"`.
- **The default pairing rule:** every field whose key ends in `_id`, plus the bare entity keys `target`, `attacker`, `entity` and `witness`, pairs with the same prefix ending in `_name` (`witness_id` → `witness_name`, `subject_player_id` → `subject_player_name`, `target_player_id` → `target_player_name`). The table below lists only the **exceptions**, where the name key or the lookup differs. A key that fits neither the rule nor the table needs an explicit exemption (NT-03).
- The canonical key table (exceptions and lookup sources):

  | ID key | Name key | Source |
  |---|---|---|
  | `entity_id`, `target`, `attacker` | `entity_name`, `target_name`, `attacker_name` | player: character name; NPC: `name_id` text, and NPC lines also carry the template pair (D-NT5) |
  | `template_id` | `template_name` | `entity_templates.template_name` |
  | `item_id` (instance), `item_type_id` / `type_id` / `design_id` | `item_name` | `items.name` via the type. A log `item_id` is often an instance ID, while the seed's `items.item_id` is a type ID: resolve an instance through its type, never by looking its ID up in `items` |
  | `ability_id` | `ability_name` | `abilities.name` |
  | `effect_id` | `effect_name` | `effects.name` |
  | `mission_id`, `step_id`, `objective_id` | `mission_name`, `step_name`, `objective_name` | `mission_label`, step / objective display text |
  | `dialog_id`, `dialog_set_id`, `speaker_id` | `dialog_name`, `dialog_set_name`, `speaker_name` | `dialogs.name` etc. |
  | `space_id`, `world_id` | `world` | `SpaceManager` / `Worlds` |
  | `account_id` | `account_name` | session |
  | `player_id` | `player_name` | session / `sgw_player` |
  | `org_id` | `org_name` | organizations |
  | `archetype` | `archetype_name` | `archetype_name()` |
  | `error_code`, `moniker_id` | `error_name`, `moniker_name` | `error_texts.moniker_name`, `monikers.name` |
  | `opcode`, `msg_id`, `method_id`, `method_index` | `method_name` | NT-30 table |

  Existing names that disagree (`character_name` vs `player_name`) are listed with the one to keep. Sweeps converge on it.
- The metric-label prohibition (a cross-reference to Rule 4) and the Discord rules (`Name (#id)`, no internal links).
- An anti-pattern entry: names only on a span.

Update [negative-logging-convention.md](../../architecture/negative-logging-convention.md)'s field catalog to match.

**Acceptance.** The rule is merged; the README design summary links to it.

### NT-01 NameBook

**Depends:** none (D-NT1: new crate `cimmeria-names`). **Effort:** M. **Agents:** database-persistence (loader), rust-gameserver-dev.

**Scope.**

- A `NameBook` (`Arc`, read-only, swapped whole on reload with `ArcSwap`) loaded at boot from `db/resources` tables:
  - `items`, `abilities`, `effects`, `missions` (with steps and objectives), `dialogs`, `dialog_sets`, `speakers`
  - `entity_templates`, `monikers`, `texts`, `error_texts`
  - `stargates`, `respawners`, `spawn_sets`, `containers`, `item_lists`, `applied_science`
  - world names
- **Placeholders are unresolved.** Seed rows whose name is a placeholder load as `None`, not as a name. As of `a679e748c` that covers `NO ITEM NAME` (89 rows), `UNUSED DIALOGUE`/`UNUSED DIALOG` and variants (about 265), `NO MISSION DISPLAY NAME` (4), `UNUSED. DELETED.` (2), `UNUSED ERROR CODE` (1). Match them with one case-insensitive pattern list in the crate (`^(NO .*NAME|UNUSED.*)$`), so new placeholders of the same shape are caught too.
- Typed lookups returning `Option<&str>`: `item(type_id)`, `ability(id)`, `template(id)`, `text(name_id)`, and so on.
- Fold in the ad-hoc helpers as thin wrappers or re-exports: `archetype_name`, `ammo_name`, `racial_paradigm_name`. Existing call sites keep compiling.
- Wired into base and cell startup, and into the existing content-reload path (`server_content_reload`), so a seed edit renames without a restart.
- A `names.loaded` info event at boot with per-table counts, and a warn listing any table that loaded zero rows.

**Tests.**

- Unit: every lookup returns `None` for an unknown ID; reload swaps atomically.
- Unit: each placeholder form (`NO ITEM NAME`, `UNUSED DIALOGUE.`, …) resolves to `None`.
- Live-DB (`live_db_namebook_*`): every seeded row in each table resolves to a non-empty, non-placeholder name, except the pinned gap list: blank and placeholder rows, by table and ID. A new blank or placeholder row fails the test until it's added to the list or given a real name.

### NT-02 Name helpers on the existing resolvers

**Depends:** NT-01. **Effort:** S.

- Extend `PlayerIdentity` with `player_name` and `account_name` on both resolvers. Rule 5 call sites get names for free.
- `SpaceManager::entity_label(entity_id) -> Option<&str>`: character name for players, `name_id` text for NPCs. NPC log lines carry this **and** `template_id` + `template_name` (D-NT5).
- Snapshot the label at the top of `destroy_entity` / `disconnect_entity`, next to the identity snapshot.
- **Departed-entity ring.** Entity IDs are recycled runtime slots (Rule 5), so the static NameBook can't name them. Each space keeps a bounded ring of recently destroyed entities: `(entity_id, label, template_id, created_at, destroyed_at)`. Keep about 10 minutes or 4,096 rows, whichever is smaller. `SpaceManager::entity_label_at(space_id, entity_id, at)` answers from the live entity when `at` falls in its lifetime, from the ring when it falls in a departed one's, and `None` otherwise. A recycled slot is never named after the wrong occupant. NT-40 uses this for delayed client rows.
- **Tests:** a recycled ID resolves to the old occupant for a timestamp before the recycle and to the new one after; a timestamp older than the ring returns `None`.
- Convert the Rule 5 guard `identity_propagation.rs` to assert the names as well.

### NT-03 Unpaired-ID scan and baseline

**Depends:** NT-00 (D-NT4: blocking). **Effort:** M. **Reviewer:** testing-validation-engineer.

- A source-scan test in the style of `crates/server/src/logging/target_scan_tests.rs`. It parses every `trace!/debug!/info!/warn!/error!/event!/info_span!`… call in `IN_PROCESS_CRATES`, and classifies **every** ID-shaped key (NT-00's default rule: any `*_id`, plus the bare entity keys), not just the keys the table lists. Each one must have its paired name key (default `<prefix>_name`, or the table's exception) in the same call, or an exemption. A table-only scan would let `witness_id` or `method_index` stay unpaired.
- `crates/server/src/logging/unpaired_id_baseline.txt`: `path count` per file. The test fails if any file's count rises or a new file appears, and prints the offending call. It also fails if a count falls without the baseline being lowered, so the ratchet stays tight.
- An inline `// nt:id-only <reason>` marker exempts one field: a pure slot counter, a test-only log, a hot path proven unreadable.
- The first run's totals go into the ledger as the campaign baseline.

---

## Phase 1: Discord

### NT-10 Discord typed events carry name and ID

**Depends:** NT-01. **Effort:** M. **Agent:** rust-gameserver-dev.

**Problem.** The typed events are half-named (README § Where things stand).

**Scope.**

- Every object field on `Event` becomes a pair, using one `Named { id, name: Option<String> }` type in `crates/discord/src/event/payload.rs`:
  - `MissionAccepted` / `Completed` / `Failed`: fill `mission_name` at all five call sites (`cell-content/.../executor/mission.rs` ×3, `cell-console/.../mission.rs`).
  - `ItemUsed`: item type ID + name, and the target as a `Named` entity.
  - `Dialog`: `dialog_id` + name, and the button's text when a choice is made.
  - `CharacterCreated`: archetype ID + name.
  - `PlayerDeath`, `PlayerRespawn`, `PlayerLevelUp`, `PlayerWorldEntry` / `Exit`: `player_id` alongside the character name. World ID + name.
  - `NpcDeath`: `entity_id`, `template_id` + `template_name`, NPC display name, killer as `Named`.
  - `GmCommand`: the target player/entity as `Named` when the command has one.
  - `MinigameResult`: the minigame's ID and the object/mission it was for.
  - `MercuryTimeout`: character name when known.
- One renderer, generalised from `account_value`: `Name (#id)`, `#id` when the name is missing, `?` when both are.
- D-NT2: the account field keeps the login name, `steve (#6)`, as today.

**Tests.** A table-driven test that builds every `Event` variant with sentinel names and IDs and asserts both appear in the rendered body. Reverting any one variant's pairing fails it. The existing whisper and IP privacy guards stay green.

### NT-11 Discord tracing layer: fold pairs, no internal links

**Depends:** NT-00. **Effort:** S.

- In the `TracingEvent` formatter (`crates/discord/src/embed/format.rs`), fold each `<p>_id`/`<p>_name` pair, and the `target`/`target_name` style pairs from the NT-00 table, into one embed field `<p>: Name (#id)`. Each pair then costs one of the 25 slots, not two. Order: identity first, then the event's object fields, then the rest.
- Fold `account_id`/`player_id`/`player_name` into one "Who" field.
- **No internal links:** add a guard over the rendered embed JSON that rejects any `http(s)://` URL. Allowlist only public, team-reachable hosts (none today). Strip `trace_id`/`span_id` *URLs*. Put the trace ID in the footer as plain text (D-NT3).
- Update [discord-notifications.md](../../architecture/discord-notifications.md): the naming section and a "no internal links" section next to the privacy sections.

**Tests.**

- Fold: an event with three pairs renders three fields.
- Budget: an event with 30 fields keeps every pair and drops only the unpaired tail.
- No links: an event whose field value contains a SigNoz URL renders without it.
- Each fails when its change is reverted.

---

## Phase 2: Server sweeps

Each sweep converts every unpaired ID field in its crates, shrinks the NT-03 baseline, and adds the capture test the dispatch rules require. The sweeps don't depend on each other and run in parallel. Sizes are relative; the NT-03 baseline gives exact counts.

| Packet | Crates | Most-read events to guard | Size | Advisor to consult |
|---|---|---|---|---|
| NT-20 Combat and effects | `cell-combat`, `cell-effect-scripts`, the effects half of `cell-world` | ability use/reject, damage, death, effect apply/expire | L | combat-systems-advisor |
| NT-21 Missions, content, dialog | `cell-content`, `content-engine`, `cell-interactions` | mission accept/advance/complete, chain fire, dialog display/choice | L | mission-systems-advisor |
| NT-22 Inventory, loot, vendor, crafting | `base-methods` (inventory, vendor, bank), `base-crafting`, loot in `cell-catalog` | move/equip/use, loot grant, vendor buy/sell, craft result | XL; may split in two | items-systems-advisor |
| NT-23 World, movement, AoI, travel | `cell-world`, `cell` (movement, witness), gate travel in `base-world-entry` | movement reject, teleport, AoI enter/leave, gate travel | L | aoi-witness-broadcast, movement-teleport-advisor |
| NT-24 Session, auth, world entry | `base`, `base-session`, `auth`, the rest of `base-world-entry` | login, play character, disconnect, timeout | M | network-security-auth |
| NT-25 NPC AI, spawner, pets, cover | `npc_ai`, `cell-catalog` spawner, `cell-pets`, `cell-cover` | aggro, leash, spawn/respawn, pet summon | M | npc-ai-spawn-advisor |
| NT-26 Social | `cell-org`, `cell-duel`, social parts of `base-methods` / `base-session` (mail, trade, BM, contacts, channels) | org change, mail send, trade execute, BM list/buy | M | social-systems-engineer |
| NT-27 GM console and minigames | `cell-console`, `minigame` | every GM command (caller and subject named), minigame result | S | — |

Sweep notes:

- **The `"world_name"` metric label (NT-25, coordinate with NT-23).** The NPC respawn counter at `cell/service/ticks/npc_respawn/mod.rs:413` is the one metric labelled `world_name`; every other world label is `world` (Rule 4). Rename it to `world`. It is a label rename, so check the SigNoz dashboards and saved views (`docs/operations/signoz/`) for queries on the old label first, and update them in the same PR.
- **NPC names that break Rule 6 (NT-25).** Four sites log `npc_name = ….as_deref().unwrap_or("")`: `npc_ai/dispatch.rs` (two), `space_manager/npc_population.rs` and `playtest_friction_watch.rs`. Pass the `Option` through instead. `npc_population.rs` also logs `name = %record.template_name`; make it `template_name`.
- **GM commands** name both the caller and the subject (Rule 5 § "Naming when an actor acts on someone else"), e.g. `player_name` + `subject_player_name`.
- **Loops over many objects** (loot tables, witness lists) log a count plus at most the first few `Name (#id)` pairs. A per-row line is a volume regression.
- **Wire crates** (`wire`, `mercury`, `wireclient`, `wire-log`) are left for NT-30. They have no content names to resolve.

---

## Phase 3: Codes and flags

### NT-30 Opcode and method names

**Depends:** none. **Effort:** M. **Agent:** rust-gameserver-dev. Check `docs/protocol/*-dispatch-table.md` first.

- One generated table, `wire::names`: Mercury message ID → name, and base/cell/client entity-method index → name, per entity type (clientIndex keyed, per the typeID rule). Reuse the existing per-module `method_name()` fns as its source where they already exist (`wire/src/base/organization.rs`, `cell_methods/organization/decode.rs`, `crafting/request.rs`, `cell-world/.../plugin/registry.rs`).
- Every log field `opcode`, `msg_id`, `method_id`, `method_index` gets a `method_name` pair (about 48 sites today). `wire-log` and `mercury.tx_hole` output include the name.
- A test that every index in the dispatch tables resolves, and that the table agrees with the dispatch-table docs (a disagreement fails and names the row).

### NT-31 Flag, enum and error-code names

**Depends:** NT-01. **Effort:** M.

- Bitflag fields (BSF state flags, effect flags, item flags; about 15 `flags = {:#x}` sites) log a `*_names` pair rendered `A|B|C`, from one `bitflags`-style formatter per flag set. Unknown bits render as `0x…`.
- Numeric enum codes logged as integers (reason codes, `aiState`, movement type, dialog UI state, error codes) gain a name. Prefer logging the Rust enum with `?` when one exists. `error_code` pairs with `error_texts.moniker_name` from NT-01.
- Positions: player-activity events that carry a position also carry `world`, and the enclosing spawn region's name when one contains the point (`regions.rs` already has region names). This is optional per site; the sweep owner decides.

---

## Phase 4: Client side and lab

### NT-40 Client telemetry resolved at ingest

**Depends:** NT-01, NT-02, NT-30. **Effort:** M. **No client patch.**

- In the replay ingest (`crates/admin-api/src/routes/telemetry/replay.rs`), resolve before re-emitting:
  - item and ability IDs → names, from the NameBook
  - entity IDs → labels, from NT-02's `entity_label_at(space_id, entity_id, row_timestamp)`, never from the NameBook. A row whose timestamp falls outside every known lifetime stays unnamed. The worker confirms the ingest route can reach the cell's `SpaceManager` (in process, through the existing admin-api state), or adds a narrow query channel for it.
  - native addresses → symbol names, from a committed symbol table (`docs/protocol/client-symbols.tsv` or similar), seeded from the addresses already documented in `docs/` and project memory (`Channel::send`, `curl_easy_setopt`, …)
  - Mercury method indexes → NT-30 names
- The DLL keeps sending raw values. Resolving on the server means the launcher's installed DLLs don't need updating.
- Client replay targets still never post to Discord (`CLIENT_REPLAY_TARGETS` is unchanged).

**Tests.** Ingest a fixture row with a known entity ID, item ID and address; assert the re-emitted event carries all three names. Assert an unknown address passes through unnamed, and that a row timestamped after its entity was destroyed and its slot reused names the original occupant.

### NT-41 Lab tools return names

**Depends:** NT-01. **Effort:** S.

- `server_entity_get`, `server_entity_query`, `server_witnesses`, `server_sessions`: every ID in the response has a name next to it. `client_entity_table` / `client_inventory` responses do the same where the server can resolve them.
- Response-shape unit tests per tool. Update [live-research-lab.md](../../guides/live-research-lab.md).

---

## Phase 5: Close-out

### NT-50 Close-out

**Depends:** all. **Effort:** S.

- SigNoz saved views (dev-only), committed as JSON under `tools/signoz/` if that convention exists, otherwise documented in [observability.md](../../architecture/observability.md):
  - "Logs with names", a column preset showing the paired fields
  - "Missing names": rows with an ID key and no name key, grouped by key, to catch seed holes
- Re-run NT-03 and record the final unpaired count; the target is the exempted residue only.
- Update `docs/gap-analysis.md`, `docs/project-status.md`, [docs/readme.md](../../readme.md) and project memory per the campaign rule.
- A short Discord before/after note for the restoration team, written by a human or the coordinator.
