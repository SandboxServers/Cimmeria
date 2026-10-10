# ADR: Persistent database profile and migration ownership

> **Last updated**: 2026-10-10
> **Audience**: The project owner deciding issue [#1290](https://github.com/SandboxServers/Cimmeria/issues/1290), and the engineers and agents who would build it
> **Type**: Architecture decision record
> **Status**: Proposed. Nothing here is built. The owner decisions in [§10](#10-owner-decisions) gate every phase.
> **Owner**: Database and persistence
> **Companion docs**: [container.md](../operations/container.md) (the image as it ships today), [colo-deploy.md](../operations/colo-deploy.md) (the colo runbook), [write-a-database-migration.md](../guides/write-a-database-migration.md) (today's manual-migration guide), [rules-and-gotchas.md](../agents/rules-and-gotchas.md) (the "seeds are the source of truth" rule), [server-infrastructure-proposals.md §3](server-infrastructure-proposals.md#3-world-state-persistence) (future durable tables), [integration-test-infra.md](integration-test-infra.md) (live-DB tests)

## TL;DR

The database holds two kinds of data that change at different rates and for different reasons, and today both are reset together on every container start:

- **Content** (the `resources` schema: 86 tables, 131 enum types and one composite type, loaded from `db/resources/`). It is a client-derived reconstruction that the project edits all the time. Since 2026-07-01, 129 non-merge commits touched `db/resources/`. Apart from the content editor and a cache-invalidation trigger, nothing writes it at runtime. The repo seed is the truth.
- **Durable state** (the `public` schema: 25 tables loaded from `db/sgw/`): accounts, characters, inventories, mission progress, mail, organizations, auctions, the login audit. Players create these rows. Its DDL changes in bursts: all 23 non-merge commits since 2026-07-01 that changed `db/sgw/` tables, types or support files fall between 2026-09-26 and 2026-10-07, about two a day during the September and October system campaigns. In the same twelve days, 84 non-merge commits touched `db/resources/`.

The recommendation is to **split ownership along that line** (Option B):

1. Content stays seed-owned and is **reseeded from the image** on every start of the persistent profile, exactly as it is today.
2. Durable schema gets **forward-only migrations** in a new `db/migrations/` directory. A PR that changes `db/sgw/` DDL ships the matching migration. CI proves that an upgraded database matches a fresh one.
3. Rollback is a **pre-upgrade `pg_dump`** of the durable schema plus the previous image. There are no down migrations.
4. A new **persistent profile** (`CIMMERIA_DB_MODE=persistent`, an external Postgres through `DB_URL`, watchtower off) sits beside the **disposable profile**, which stays the default and keeps reseeding everything.

Thirteen owner decisions are listed in [§10](#10-owner-decisions). The largest are D-2 (narrowing the "seeds are the source of truth" rule to content), D-12 (content fixes that no longer reach existing characters) and D-9 (whether and when the colo becomes persistent).

## 1. Problem

[`docker/entrypoint.sh`](../../docker/entrypoint.sh) clears `PGDATA` and copies the image's baked cluster over it on every container start (lines 58-76 at `87912c644`). An image update, a `docker restart`, a Docker restart and a host reboot all start from a clean database. That is deliberate and documented ([container.md → Volume / persistence](../operations/container.md#volume--persistence), [colo-deploy.md → The database resets on every start](../operations/colo-deploy.md#the-database-resets-on-every-start)). It also means characters, inventory and mission progress never survive a release.

The documented escape is "run Postgres outside the image and point `DB_URL` at it, and then you own schema drift between releases." Nothing supports that today:

- **The image cannot create its own schema elsewhere.** The `db-init` build stage loads `db/database.sql` into the baked cluster and deletes `/tmp/db`. The runtime image has no SQL files. An operator with an empty external Postgres has to load `database.sql` from a repo checkout of the right commit by hand.
- **Nothing tells the operator what changed.** There is no schema version in the database and no list of DDL changes per release. The only migration files are the twelve historical, hand-applied scripts in `db/scripts/`, which the project rule now forbids adding to without asking.
- **The image assumes its bundled Postgres in three places**, whatever `DB_URL` says. The entrypoint reseeds `PGDATA`. The s6-rc service `cimmeria-server` depends on the `postgres` longrun through `dependencies.d/postgres`, an edge compiled at image build time, and its [`run`](../../docker/s6/cimmeria-server/run) script waits on `pg_isready -h 127.0.0.1`. The Dockerfile `HEALTHCHECK` runs `pg_isready -h 127.0.0.1` too. With an external database the bundled cluster still starts and reseeds, and both checks look at the wrong database.
- **The server never touches the schema.** No non-test crate code runs DDL or sqlx migrations. [`orchestrator_postgres.rs`](../../crates/services/src/orchestrator_postgres.rs) only starts a local Postgres, and skips that for non-local hosts. Schema is loaded by `setup.ps1 -ForceDatabase`, by `db.bat`, or by the Docker build, all from `db/database.sql`.

Issue #1290 asks for a documented persistent profile with migration and rollback steps, proof that a restart and an image update preserve a test character and other mutable state, the disposable profile kept as it is, and a smoke procedure that catches data loss.

## 2. Constraints

- **Seeds are the source of truth** ([CLAUDE.md](../../CLAUDE.md), [rules-and-gotchas.md](../agents/rules-and-gotchas.md), [db/README.md](../../db/README.md)). Content changes edit `db/resources/` directly; no new `db/scripts/*.sql` without asking. Any design that adds migrations changes this rule, and the change needs the owner (D-2).
- **`db/resources/` is a reconstruction, not the 2009 database.** It is rebuilt from client evidence and corrected constantly. Content rows must stay cheap to change. Requiring a migration for every content edit would end that.
- **Content keys are mostly client-derived.** Item, world, mission, ability and dialog ids come from the client's cooked data, so they are stable across releases. Author-assigned keys (content chains, spawn rows, stargate rows, sentinel ranges) are not guaranteed stable today.
- **Single host, stop-the-world upgrades.** The colo is one Docker host with one server process. No design here needs old and new binaries to share a database at the same moment.
- **No client impact.** This is deployment and database behaviour only.
- **Telemetry is first-class.** Every migration, reseed and integrity check must log what it did under a stable target, so an operator can read an upgrade from SigNoz alone.
- **PostgreSQL 17 client tools.** The image ships 17.9 and its `psql`, `pg_dump` and `pg_restore`. The baked content SQL restores into 17 or newer, but `db-prepare` also takes backups, and `pg_dump` 17 refuses to dump a server of a newer major version. So the external server must be **17.x until the image's client tools move to a newer major**. That is also why a major upgrade of the external server is out of scope here ([§11](#11-consequences-and-risks)): it needs an image built with the newer tools first.

## 3. Table classification

The schema already draws the line: `db/database.sql` loads the `resources` schema from `db/resources/` and then the `public` schema from `db/sgw/`. The classification follows the schema, with the exceptions called out below.

### 3.1 Content: reseedable (`resources` schema)

All 86 tables, 131 enum types, the composite type `active_interaction_map`, 34 sequences, and their functions and triggers. Grouped by `db/resources/` directory:

| Area | Tables |
|---|---|
| AI | `cover_sets`, `cover_nodes` |
| Abilities | `abilities`, `ability_moniker_groups`, `ability_set_abilities`, `ability_sets`, `applied_science`, `trainer_abilities`, `trainer_ability_lists`, `ammo_modifiers` |
| Archetypes | `archetype_ability_tree`, `archetypes`, `char_creation`, `char_creation_abilities`, `char_creation_choices`, `char_creation_debug_kit_abilities`, `char_creation_debug_kit_items`, `char_creation_items`, `char_creation_visgroups`, `disciplines`, `racial_paradigm` |
| Content engine | `content_chains`, `content_triggers`, `content_conditions`, `content_actions`, `content_counters` |
| Dialogs | `ambient_chatter_groups`, `ambient_chatter_lines`, `dialog_screen_buttons`, `dialog_screens`, `dialog_set_maps`, `dialog_sets`, `dialogs`, `speakers`, `special_words` |
| Effects | `effect_nvps`, `effects` |
| Entities | `blueprints`, `blueprints_components`, `deployables`, `entity_templates`, `monikers`, `pet_summons`, `resource_types`, `resource_versions` |
| Events | `event_sets`, `event_sets_sequences`, `paths`, `point_set_points`, `point_sets`, `sequences`, `sequences_nvp` |
| Items | `ammo_item_types`, `bank_expansion_price`, `containers`, `crafting_item_effects`, `item_list_items`, `item_list_prices`, `item_lists`, `items`, `items_event_sets` |
| Loot | `loot`, `loot_tables` |
| Missions | `mission_objectives`, `mission_reward_groups`, `mission_rewards`, `mission_steps`, `mission_tasks`, `missions` |
| Texts | `error_texts`, `texts` |
| Visuals | `body_component_visuals`, `body_components`, `body_sets`, `skeletal_meshes`, `static_meshes` |
| Worlds | `entity_interactions`, `generic_regions`, `interactions`, `respawners`, `ring_transport_regions`, `spawn_points`, `spawn_sets`, `spawnlist`, `stargates`, `worlds` |

Two code paths write content at runtime, and a reseed discards what they wrote:

- **The admin API content editor** writes `content_chains` and its four child tables through `save_content` and `delete_content` in [`routes/editor.rs`](../../crates/admin-api/src/routes/editor.rs). (`save_draft` is a stub.) Editor changes that matter are meant to land in the seed.
- **`resource_update_trigger`** appends to `resource_versions`, the client-cache invalidation ledger, on every content change. A reseed restores the baked ledger, which is what every colo deploy does today.

Nothing else writes `resources` outside tests: every other `INSERT INTO resources.*` in `crates/` is under `#[cfg(test)]`. The GM `.`-console authoring commands only format seed SQL for a human to commit and never run it ([dev-console-channel.md](dev-console-channel.md)).

### 3.2 Durable: migrated, never reseeded (`public` schema)

| Class | Tables | Seeded rows today |
|---|---|---|
| Accounts | `account` | 14 dev and lab accounts, GM level, password `test` (unsalted SHA-1) |
| Characters | `sgw_player`, `sgw_player_ability_grants`, `sgw_player_content_cooldown`, `sgw_player_discipline_expertise`, `sgw_player_tutorials` | 9 playtest characters (ids 62-70) |
| Inventory | `sgw_inventory_base` (parent), `sgw_inventory` (inherits it) | the seeded characters' starting kit |
| Missions | `sgw_mission` | none |
| Mail | `sgw_gate_mail`, `sgw_gate_mail_item` | none (the seed file only sets the sequence) |
| Social | `sgw_contact_list`, `sgw_contact_list_member` | contact lists for the QA characters |
| Organizations | `sgw_organizations`, `sgw_organization_ranks`, `sgw_organization_members`, `sgw_organization_vault_items`, `sgw_organization_cash_log`, `sgw_organization_events`, `sgw_organization_vault_log` | none |
| Black Market | `sgw_auction`, `sgw_auction_bid` | none |
| Audit | `login_audit` | none |
| Delivery queue | `cell_event_outbox` | none. Undelivered rows are retried at startup, so they must survive a restart too. |
| Operator config | `shards` | 1 shard, `Test` |

The `public` schema also owns the composite type `player_interaction_map`, 12 sequences, and the organization and Black Market trigger functions in `db/sgw/_functions.sql`.

### 3.3 References from durable state into content

This is the part that makes a content reseed dangerous, and the part the design has to handle.

**Hard dependencies** (Postgres enforces them, so a careless reseed either fails or destroys data):

| Durable column | Content target | Today |
|---|---|---|
| `sgw_inventory.type_id` | `resources.items(item_id)` | FK, `ON UPDATE CASCADE ON DELETE RESTRICT` |
| `sgw_gate_mail_item.type_id` | `resources.items(item_id)` | FK, `ON UPDATE CASCADE ON DELETE RESTRICT` |
| `sgw_organization_vault_items.type_id` | `resources.items(item_id)` | FK, `ON UPDATE CASCADE ON DELETE RESTRICT` |
| `sgw_player.world_id` | `resources.worlds(world_id)` | FK, `ON UPDATE RESTRICT ON DELETE RESTRICT` |
| `sgw_player.world_location` | `resources.worlds(world)` | FK, `ON UPDATE RESTRICT ON DELETE RESTRICT` |
| `ammo_type`, `ammo_types` on `sgw_inventory_base`, `sgw_gate_mail_item`, `sgw_organization_vault_items` | enum type `resources."EAmmoType"` | column type |

Two of these need care beyond a reseed:

- **`EAmmoType`.** `DROP SCHEMA resources CASCADE` would drop those columns from durable tables, and the ammo state of every item with them. Any reseed must either never drop `resources."EAmmoType"` or move the type out of `resources` first (D-6, costed in [§6.6](#66-moving-eammotype-out-of-resources-d-6)). Four content tables use the same type (`items`, `ammo_item_types`, `ammo_modifiers`, `entity_templates`). Content may depend on a durable-owned type, but durable state must not depend on a content-owned one.
- **`ON UPDATE CASCADE` on the three `items` FKs.** A drop-and-reload reseed never fires it. A content data migration that renumbers an `item_id` with `UPDATE` would silently rewrite every inventory, mail and vault row holding that item. Renumbering items must be a deliberate durable data migration, not a side effect.

**Soft references** (plain integers or strings, no FK). A reseed that drops or renumbers the target leaves them dangling, with no error:

| Durable column | Content it names |
|---|---|
| `sgw_player.abilities`, `trained_abilities`, `sgw_player_ability_grants.ability_id` | `abilities` |
| `sgw_player.known_stargates` | `stargates` |
| `sgw_player.known_respawners` | `respawners` |
| `sgw_player.blueprint_ids` | `blueprints` |
| `sgw_player.discipline_ids`, `sgw_player_discipline_expertise.discipline_id` | `disciplines` |
| `sgw_player.title` | title ids (texts) |
| `sgw_player.interaction_maps` (`template_id`, `interaction_set_map_id`, `mission_id`) | `entity_templates`, `dialog_set_maps`, `missions` |
| `sgw_player.looted_containers` | content chain `container_key` values and spawn tags |
| `sgw_player.components`, `bodyset` | body component and body set names |
| `sgw_mission.mission_id`, `current_step_id`, the four objective-id arrays | `missions`, `mission_steps`, `mission_objectives` |
| `sgw_player_tutorials.tutorial_id` | `dialogs` |
| `sgw_player_content_cooldown.cooldown_key` | content chain cooldown keys |
| `sgw_auction.item_def_id` | `items` |

**Ranges hard-coded in durable CHECK constraints.** `sgw_player` checks `alignment` (0-5), `archetype` (0-8), `gender` (1-3), `skin_color_id` (0-15), `level` (0-50), `bandolier_slot` (0-3) and `bank_slots` (40-100, multiples of 10). Those ranges mirror content enums and tables. A content change that widens one (a new archetype, a higher level cap) is a durable DDL change and needs a migration.

Once a persistent database exists, **every key in the soft-reference table becomes a contract.** Renumbering or deleting one needs a durable data migration in the same PR. That is the price of persistence, and it lands on content authors, which is why D-5 asks how strictly to enforce it.

### 3.4 Content copied into durable rows

Durable rows also hold **copies** of content values, not just keys. Character creation copies the start profile: the starting kit into `sgw_inventory`, starter and racial abilities into `sgw_player.abilities` and `sgw_player_ability_grants` (#1264, #1273), the `racial_paradigm_levels` defaults, `components` and `bodyset`. Inventory rows copy `ammo_types` from `resources.items` when they are created (see the comment in `db/sgw/Inventory/Seed/sgw_inventory.sql`).

Today every release starts with fresh characters, so a seed fix to a start kit, a granted ability or an item's ammo list reaches every character. With persistence it reaches **only characters created after the fix**. The key-contract rule in §3.3 does not catch this, because no key changes. It changes how playtest feedback gets fixed: either the content PR carries a durable backfill migration for existing characters, or the fix is accepted as new-characters-only, or a GM resets the affected characters. D-12 asks which.

## 4. Options for migration ownership

### Option A: Status quo, plus documentation

Write down what the operator does by hand: load `database.sql` from the release's commit into an external Postgres, and at each upgrade work out the DDL difference between the two release tags and apply it.

- **Who writes migrations:** the operator, per deployment, every release.
- **CI verification:** none.
- **Rollback:** whatever backup the operator took.
- **"Seeds are truth":** unchanged.
- **Assessment:** cheapest for the project, and it does not meet #1290's acceptance criteria (no migration steps, no smoke). It moves the whole cost to whoever runs a persistent server, and the colo would be first in line. Content and durable changes are mixed in one diff, so every upgrade starts with sorting them.

### Option B: Split ownership. Forward-only durable migrations, content reseeded (recommended)

- Durable DDL keeps its canonical home in `db/sgw/`; fresh installs still load it through `database.sql`.
- Each PR that changes durable DDL also adds a migration in `db/migrations/`: forward-only, written against the latest release plus the migrations merged since, with any data backfill in the same file (numbering in [§6.5](#65-what-a-durable-change-looks-like-after-d-2)).
- A schema-version table in `public` records which migrations a database has. A fresh `database.sql` load stamps every migration in the directory as applied, so a fresh install and an upgraded install end up in the same state.
- Content is reloaded from the image on every start in the persistent profile, through a guarded swap ([§6.4](#64-what-db-prepare-does-in-persistent-mode)).
- **Who writes migrations:** the author of the PR that changes `db/sgw/` (agent or human). The database-persistence agent reviews.
- **CI verification:** load the previous release's `db/` from its git tag, add a durable fixture, apply HEAD's migrations and content, then compare the result with a fresh HEAD load using `pg_dump --schema-only` ([§8.2](#82-ci)).
- **Rollback:** automatic `pg_dump` of `public` before any pending migration runs. Rolling back means restoring that dump and running the previous image. Play since the upgrade is lost.
- **"Seeds are truth":** still true for content, which is most of the `db/` churn (129 content commits against 23 durable-DDL commits since 2026-07-01, both counted without merges). Durable DDL gains one extra file per change.
- **Assessment:** matches the real split in how the data changes, keeps content edits as cheap as they are now, and the CI check means a forgotten or wrong migration fails a PR, not a server.

### Option C: Declarative schema diff

Keep `db/sgw/` as the only truth and generate the upgrade at release time with a schema-diff tool (`pg-schema-diff`, Atlas, `migra` and similar) run between the previous release's schema and the new one.

- **Who writes migrations:** nobody by hand; the release pipeline generates them, and a human reviews destructive steps.
- **CI verification:** generate the diff in CI and fail on destructive statements nobody approved.
- **Rollback:** backup and restore, as in B.
- **"Seeds are truth":** unchanged for authors; `db/sgw/` stays the only file to edit.
- **Assessment:** attractive for authors, weak on data. Diff tools see a rename as a drop plus an add, cannot write backfills (for example `racial_paradigm_levels` defaults for existing characters), and handle enum value changes, inheritance (`sgw_inventory` inherits `sgw_inventory_base`) and composite types unevenly. It adds a third-party tool to the release path. The useful part, comparing schemas, is folded into B's CI check without the tool.

### Option D: Dump and restore across releases

At each upgrade, export durable rows from the old database (`pg_dump --data-only --schema=public`), start the new image on a fresh database, and import the rows.

- **Who writes migrations:** the project still writes a per-release transform whenever a durable table gains a `NOT NULL` column without a default, renames a column or splits a table. Otherwise the import fails.
- **CI verification:** export from the previous release, import into the new one.
- **Rollback:** keep the export; very simple.
- **"Seeds are truth":** unchanged.
- **Assessment:** simple for small databases and for releases with no durable DDL change, but it only works without a transform when nothing changed, which is when it is least needed. Import time grows with the player base. It is the right **backup and rollback** mechanism, and B uses it for that.

### Comparison

| | A: status quo | B: split (rec.) | C: schema diff | D: dump/restore |
|---|---|---|---|---|
| Meets #1290 | No | Yes | Yes | Partly |
| Content edits stay seed-only | Yes | Yes | Yes | Yes |
| Extra work per durable DDL change | Operator, every deploy | One migration file per PR | Review the generated diff | Transform when columns change |
| Data backfills | By hand | In the migration | Not supported | In the transform |
| CI can prove the upgrade | No | Yes, schema equality | Yes, diff review | Yes, import |
| Rollback | Ad hoc | Pre-upgrade dump | Pre-upgrade dump | The export |
| New dependencies | None | None | Diff tool | None |

## 5. Recommendation

Adopt **Option B**, with Option C's schema comparison as B's CI check (plain `pg_dump --schema-only`, normalized) and Option D's dump as B's automatic pre-upgrade backup.

Why B and not the others:

- Content and durable state already live in different schemas and change for different reasons. B makes that existing line the ownership line, so a content author's workflow does not change at all.
- Durable DDL changes come in bursts of about two a day during system campaigns (23 between 2026-09-26 and 2026-10-07). A manual process (A) or a transform per release (D) would be paid at exactly the busiest time. Each change is small, so one hand-written migration per PR is reasonable.
- Backfills are common in this schema's history: `state_field`, the `racial_paradigm_levels` defaults, `cur_ammo_type`. Only B carries them as a first-class part of the change.

Phases, each gated on the decisions it needs:

| Phase | Delivers | Needs |
|---|---|---|
| P0: prerequisites | Move `EAmmoType` out of `resources`, with the reordering in [§6.6](#66-moving-eammotype-out-of-resources-d-6). Add the schema-version table and the stamp to `database.sql`. Add `db/migrations/** text eol=lf` and the stamp file to `.gitattributes`. Bake the SQL that persistent mode needs into the image. No behaviour change for the disposable profile. | D-1, D-6, D-13 |
| P1: persistent profile | `CIMMERIA_DB_MODE`; the entrypoint, s6 and healthcheck changes in [§6.3](#63-image-changes-for-persistent-mode); the compose overlay; the `db-prepare` step (bootstrap, migrate, reseed, integrity check, version guard); the migration runner; telemetry; operator docs. The first release with P1 is the migration baseline: `db/migrations/` starts empty. | D-2, D-3, D-4, D-5, D-7, D-8, D-11, D-12 |
| P2: CI | The PR-level SQL upgrade job and the release-level container smoke. | D-10 |
| P3: colo | Cut-over, backups, announcement. | D-9 |

## 6. Profile shape

### 6.1 Two profiles, chosen explicitly

The profile is chosen by an environment variable, not inferred from `DB_URL`. An operator who points `DB_URL` somewhere for another reason must not get a migrating server by surprise.

| | Disposable (default) | Persistent |
|---|---|---|
| `CIMMERIA_DB_MODE` | unset or `disposable` | `persistent` |
| Database | Bundled Postgres inside the image | An external Postgres 17 (a sibling container or a managed server) through `DB_URL` |
| On every start | Reseed `PGDATA` from the image (today's entrypoint, unchanged) | Leave the bundled cluster alone and keep it idle. Run `db-prepare`, then the server. |
| Image updates | Watchtower, as today | Pinned image tag, upgraded by hand (D-8) |
| Characters survive | Nothing survives a start | Restarts and upgrades |

Persistent mode refuses to start when `DB_URL` names the bundled cluster (`127.0.0.1:5432` inside the image), because that cluster is not on a volume the design manages. A bundled Postgres on a named volume was considered and rejected: it ties Postgres major upgrades to image releases, and the watchtower volume behaviour described in the entrypoint header already caused one silent carry-over.

### 6.2 Compose layout

A new overlay, `docker/compose.persistent.yml`, chosen with `COMPOSE_FILE` like the existing overlays:

- adds a `postgres` service (official `postgres:17` image, pinned minor) with a named volume `cimmeria-pgdata`, no published port, on the compose network only;
- sets `CIMMERIA_DB_MODE=persistent` and `DB_URL=host=postgres port=5432 user=cimmeria dbname=sgw`, with the password from `.env` or a Docker secret, not the baked `w-testing` credentials;
- sets the label `com.centurylinklabs.watchtower.enable=false` on `cimmeria` (D-8). Compose merges labels across files and cannot delete the base file's `true`, so the overlay overrides the value instead;
- mounts a host directory for pre-upgrade backups (`CIMMERIA_DB_BACKUP_DIR`).

`DB_URL` stays in libpq key-value form, for the reason [container.md](../operations/container.md#environment) gives.

### 6.3 Image changes for persistent mode

The image hard-wires its bundled Postgres in three places (§1). Each needs a mode switch. s6-overlay keeps the container environment (`S6_KEEP_ENV=1`), so every script below can read `CIMMERIA_DB_MODE`.

| Place | Today | Persistent mode |
|---|---|---|
| `docker/entrypoint.sh` | `rm -rf` of `PGDATA`, then copy the baked cluster | Skip the clear and the copy. Validate `CIMMERIA_DB_MODE` (unknown value: exit) and refuse a `DB_URL` that names the bundled cluster. |
| `docker/s6/postgres/run` | `exec postgres -D ...` | `exec sleep infinity`. The s6-rc graph is compiled at build time, so `cimmeria-server`'s `dependencies.d/postgres` edge cannot be removed per start. An idle longrun keeps the edge satisfied without starting a cluster. |
| `docker/s6/cimmeria-server/` | `dependencies.d/postgres`; `run` waits on `pg_isready -h 127.0.0.1` | A new oneshot `db-prepare` sits between them: `cimmeria-server` depends on `db-prepare`, and `db-prepare` depends on `postgres`. In disposable mode `db-prepare` waits on the local cluster, which is today's wait moved out of `run`. In persistent mode it runs §6.4. |
| Dockerfile `HEALTHCHECK` | `pg_isready -h 127.0.0.1 ...` and a TCP probe of `LOGON_PORT` | Call a small script that runs `pg_isready -d "$DB_URL"` (`-d` accepts a libpq connection string, so one command covers both modes) and then the same `LOGON_PORT` probe. |

### 6.4 What `db-prepare` does in persistent mode

`db-prepare` runs once per container start, before the server. Every step logs under one target (for example `db.prepare`), with the image version, the schema version before and after, and the counts.

1. **Wait** for the database in `DB_URL`, bounded by `POSTGRES_WAIT_TIMEOUT`.
2. **Take an advisory lock**, so two containers pointed at one database cannot prepare it at once.
3. **Classify the database:**
   - empty: **bootstrap**, in the order `database.sql` uses, because the durable DDL needs content to exist first. (a) After D-6: the shared types in `public`, `EAmmoType` among them. (b) Content: the `resources` schema. (c) The durable tables, sequences, functions and triggers, without the cross-schema FKs. (d) The five cross-schema FKs. (e) The stamp. (f) The bootstrap rows D-7 allows. Then start the server; no reseed is needed;
   - schema version **equal** to the image's: go to step 5;
   - **older**: go to step 4;
   - **newer** than the image (a rollback without a restore), or an unknown migration recorded: **refuse to start**, and log both versions and the restore instructions.
4. **Migrate.** `pg_dump --schema=public` to the backup directory first. Stop if the dump fails. Then apply each pending migration in version order, each in its own transaction unless the file is marked non-transactional (see §6.6), and record it in the version table.
5. **Reseed content** in one transaction. The image carries content as **plain SQL** (`pg_dump -n resources` in plain format, taken in the `db-init` stage, or produced from a custom-format dump with `pg_restore -f -`), because `pg_restore --single-transaction` opens its own session and cannot share a transaction with the FK statements. `db-prepare` runs one `psql -1 -v ON_ERROR_STOP=1` over three parts: drop the five cross-schema FKs and `DROP SCHEMA resources CASCADE` (safe only after D-6), the content SQL, then re-add the FKs. If any durable row now points at a missing item or world, the FK re-add fails, the whole transaction rolls back, the old content stays, and the server does not start. The transaction holds `ACCESS EXCLUSIVE` locks on all of `resources`, which is fine because the server is not running yet. D-11 decides whether to skip this step when the image's content fingerprint matches the database's.
6. **Check soft references** (§3.3): count durable rows whose ability, mission, stargate, respawner, discipline, dialog, blueprint or title id no longer exists. Log every one with its name pair. D-5 decides whether a non-zero count stops the start or only warns.
7. **Release the lock** and start the server.

The image must therefore carry, beside today's baked cluster: the content SQL above, the durable baseline split into the parts (a), (c), (d) and (e), and `db/migrations/`. All of it comes from the same build as the baked cluster, so the two profiles cannot drift apart.

### 6.5 What a durable change looks like after D-2

Schema-version table, one in `public`, owned by the durable schema:

| Column | Meaning |
|---|---|
| `version` | The migration's version (primary key; see numbering below) |
| `name` | The file name |
| `sha256` | The file's hash; a mismatch on a recorded migration stops the start |
| `applied_at` | When it ran |
| `image_version` | The release tag that applied it |

**Line endings.** `core.autocrlf=true` is in use here, so `db/**` checks out with CRLF on Windows while the image and CI see LF. A hash over raw bytes would differ between a stamp generated on Windows and the image, and a correct database would refuse to boot. P0 does both of these: `.gitattributes` gets `db/migrations/** text eol=lf` and an entry for the stamp file, so every checkout has LF; and the runner hashes the file with CRLF normalized to LF, so a file that a Windows editor rewrote still matches.

**Stamp.** The fresh-load stamp (`db/sgw/_migrations_stamp.sql`, loaded at the end of `database.sql`) inserts one row per file in `db/migrations/`. A CI check fails when the stamp and the directory disagree. Live-DB tests and `setup.ps1` load `database.sql` as now, so they get the stamp without changes.

**Numbering.** Releases are frequent (three on 2026-10-05), and PRs that change durable DDL often run in parallel. Two PRs that each take the next sequential number would both pass CI alone and collide on merge. D-13 picks the scheme; the recommendation is a UTC timestamp prefix (`20261010T1430_player_<name>.sql`) as the version, applied in version order, plus two checks: a check on `main` after each merge that versions are unique, and a release-time check that refuses a release in which a migration's version is older than the newest migration already in the previous release (rename it before releasing).

**Runner.** D-3 decides whether the runner is sqlx's embedded migrator (`_sqlx_migrations`, already a dependency, checksums built in, but its checksum rows are awkward to stamp from plain SQL and it hashes raw bytes) or a small in-house runner over the table above. This ADR leans to the in-house runner because the stamp has to be writable by `psql` and the hash has to be line-ending-neutral.

A PR that adds a column to `sgw_player`:

1. edits `db/sgw/Players/Tables/sgw_player.sql`, as today;
2. adds `db/migrations/<version>_player_<name>.sql`: the `ALTER TABLE`, plus any backfill. No `IF NOT EXISTS`, because the runner tracks what was applied;
3. regenerates the stamp;
4. passes the CI upgrade job, which fails if (1) and (2) disagree.

A content PR changes nothing in its workflow, with three exceptions: it deletes or renumbers a key listed in §3.3 (it then needs a durable data migration, and the reference check in CI says so, D-5); it widens a range in a durable CHECK constraint (§3.3); or it changes content that durable rows copied (§3.4, D-12).

`db/scripts/` stays frozen as history and is never run by the runner.

### 6.6 Moving `EAmmoType` out of `resources` (D-6)

Moving the type is what makes `DROP SCHEMA resources CASCADE` safe, and it is not free:

- **`database.sql` reorders.** Today it loads all of `resources`, then all of `public`. After the move, `public."EAmmoType"` (and any later shared type) must exist before the four content tables that use it. The shared types get their own file, loaded first, before `resources/_schema.sql`.
- **The content SQL depends on `public`.** The baked `resources` SQL references `public."EAmmoType"` but does not create it, so it only loads into a database that already has the type. Bootstrap order (§6.4 step 3) and the reseed (step 5) both satisfy that, and the CI job has to as well.
- **New ammo types become durable migrations.** Adding an `EAmmoType` value is driven by client data, so it is a content change in spirit, but the type is now durable: the PR needs `ALTER TYPE public."EAmmoType" ADD VALUE ...` as a migration. Postgres allows `ADD VALUE` inside a transaction block but forbids using the new value until that transaction commits. A migration that adds a value and backfills rows with it cannot be one transaction. The runner therefore supports either a non-transactional marker on a file or the rule that the value is added in one migration and used in the next. The content seed using the value loads after migrations (step 5), so it is unaffected.
- **The alternative** is to keep the type in `resources` and never drop it: the reseed then truncates and reloads the content tables and leaves types alone, and `EAmmoType` changes still need `ALTER TYPE`. That avoids the reordering but makes the reseed depend on every content table's DDL staying compatible between releases, which defeats the point of reseeding content.

## 7. Rollback

Forward-only, by restore:

1. Stop the container.
2. Restore the pre-upgrade dump that `db-prepare` wrote (`pg_restore --clean --schema=public`), or restore the whole database from the operator's regular backup.
3. Start the previous image tag. Its `db-prepare` finds a matching schema version, reseeds that release's content, and starts.

Play between the upgrade and the restore is lost, and the runbook says so. Down migrations are not offered: they are rarely tested, they cannot restore data a forward migration dropped, and in a single-host stop-the-world setup a restore is faster to trust. The version guard in `db-prepare` step 3 stops the other failure: an old image starting on a newer schema and writing rows the new code would misread.

## 8. Detecting data loss

### 8.1 Operator smoke procedure

A script (proposed name `tools/persistent-db-smoke.sh`) for the persistent profile. It fingerprints the durable tables with a column list pinned to the release the fixture was written on, so columns a migration adds do not change the fingerprint:

```sql
-- one row per stable table; see the table classes below
SELECT 'sgw_player' AS tbl, count(*) AS n,
       md5(string_agg(row(player_id, account_id, player_name, level, exp,
                          naquadah, world_id, pos_x, pos_y, pos_z, abilities)::text,
                      '|' ORDER BY player_id)) AS digest
FROM sgw_player;
```

The server changes some durable rows on its own, so an exact digest of every table would fail on a healthy database. The script treats three classes differently:

| Class | Tables | How it is compared |
|---|---|---|
| Stable | every durable table not listed below | Exact count and digest. The fixture's rows must be inert: mail with `expires_at` far in the future and not quarantined, an auction whose `expires_at` is far in the future. |
| Server-mutated columns | `cell_event_outbox` (the startup drainer sets `delivered_at`, `attempts`, `last_error`); `sgw_auction.status` (the Black Market sweep); `sgw_gate_mail` expiry and quarantine columns | The fixture outbox row is already delivered, so the drainer skips it. Those columns are left out of the digest; the rest of the row is compared exactly. |
| Append-only | `login_audit` (every login inserts a row) | The rows present at the baseline must still be there, unchanged, and the count may only grow. |

Logging in also updates the character (position, `first_login` and similar). So the procedure **fingerprints before any login**, and logs in only afterwards, as a functional check:

1. **Install.** Bring up the persistent profile with release N-1 against an empty Postgres. Confirm that bootstrap ran and the schema version matches N-1.
2. **Create state.** Log in with a client or with [`cimmeria-wireclient`](wireclient.md), create a character named `Persist<date>`, move it, loot or buy an item, equip something, accept a mission and advance one objective, send a gate mail with an attachment, add a contact, then log out. For CI, load a SQL fixture that writes at least one inert row into every durable table in §3.2.
3. **Baseline.** Fingerprint every durable table.
4. **Restart.** `docker compose restart cimmeria`. Wait for the healthcheck, then fingerprint again, before anyone logs in. Every comparison must pass. Then log in: the character list must show `Persist<date>` at its saved position. Log out and take a new baseline.
5. **Upgrade.** Change the image tag to release N and run `docker compose up -d`. Confirm `db-prepare` logged a backup, the migrations it applied, a content reseed and zero dangling references. Fingerprint with the N-1 column lists before anyone logs in. Every comparison must pass. Then log in and check the character, its item, its mission step and its mail.
6. **Content check.** The `resources` fingerprint must equal the fingerprint baked into image N, which proves the reseed happened.
7. **Guard check.** Delete one fixture row by hand and run the comparison again; it must fail. A comparison that cannot fail proves nothing, and a comparison that always fails proves nothing either, which is what the table classes above prevent.

### 8.2 CI

Two jobs (D-10):

- **PR job, SQL only, no Docker.** Runs when a PR touches `db/`. Loads the latest release tag's `db/` (`git archive <tag> db`) into a scratch Postgres, loads the durable fixture, applies HEAD's `db/migrations/`, reseeds HEAD's `resources`, runs the soft-reference check, then compares `pg_dump --schema-only --schema=public` with a fresh HEAD load after normalizing ordering and comments. It also checks the stamp against the directory, that migration versions are unique, and that no `db/sgw/` file refers to a `resources` type. It fits beside the existing `test-live-db` job.
- **Release job, container.** In `release-container.yml`, before `latest-prerelease` is promoted: run §8.1 against the previous dated image tag and the candidate, with a Postgres service container and the SQL fixture in place of a client, and the wireclient for the login checks. It also runs the migration-version ordering check from §6.5. Release images are already tagged by version, so N-1 is available.

## 9. Colo rollout

The colo is the only standing deployment, and its players are used to a reset on every release. Making it persistent changes more than the database:

- **Cut-over is a wipe.** The first persistent release starts from the seed. Characters made before it are not carried over, because there is nothing to carry. Announce the date.
- **Accounts stop coming from the seed.** Today a new colo tester is a new row in `db/sgw/Accounts/Seed/account.sql`, shipped with the next release ([colo DB refresh behaviour](../operations/colo-deploy.md#the-database-resets-on-every-start)). In persistent mode the `account` table is durable and the seed loads once, at bootstrap. The colo needs an account-provisioning path (an admin API route, a GM console command or a one-off durable data migration) before cut-over (D-7).
- **Dev and lab GM accounts with password `test`** (14 of them) are fine on a server that resets every release. On a persistent public server they are a standing credential. D-7 decides whether bootstrap loads them.
- **Content fixes stop reaching existing characters** (§3.4). A start-kit, granted-ability or ammo-list fix that playtesters ask for reaches only new characters unless the PR carries a backfill (D-12). Testers who expect "fixed in the next release" need to hear this.
- **Seed-only content fixes still ship with the next release** and need nothing on the client, as today. Durable fixes (a character stuck by a bad mission state) now need a data migration or a GM action.
- **Watchtower.** With D-8 at "off", each release becomes a manual `docker compose pull && docker compose up -d` on the colo. With D-8 at "on", `db-prepare` migrates unattended, and the backup directory must have room.
- **Reboots stop resetting the database.** colo-deploy.md ([The database resets on every start](../operations/colo-deploy.md#the-database-resets-on-every-start)) says the colo reboots at 04:30 when an automatic security update needs it, and that those reboots reset the database too. In persistent mode they preserve it.
- **Backups.** A nightly `pg_dump` of the external database to host disk, with a retention policy, separate from the per-upgrade dumps. Without it, a lost volume loses every character.
- **Compose edits need `docker compose up -d`.** Watchtower recreates from the old container's settings, so adding the overlay is a one-time manual step on the host.
- **Resources.** A second Postgres container on the colo host, and Postgres minor upgrades become an operator task. Major upgrades wait for an image with matching client tools (§2).

## 10. Owner decisions

| ID | Decision | Recommendation |
|---|---|---|
| D-1 | Adopt split ownership (Option B): content reseeded from the image, durable schema migrated forward. | Yes |
| D-2 | Narrow "seeds are the source of truth" to content (`db/resources/`). Durable DDL changes in `db/sgw/` must ship a migration in a new `db/migrations/`, written by the PR author. `db/scripts/` stays frozen. Update CLAUDE.md, rules-and-gotchas.md, db/README.md, copilot-instructions.md, the migration guide and the database-persistence agent definition, which today says to put migrations in `db/scripts/`. | Yes, from the P1 release on |
| D-3 | Migration runner: sqlx's embedded migrator, or a small in-house runner over a `schema_migrations` table that `database.sql` can stamp, with line-ending-neutral hashes and a non-transactional marker. Runs from `db-prepare` at start, or only from an explicit `migrate` command the operator runs. | In-house runner, run automatically by `db-prepare` with a backup first |
| D-4 | Rollback: forward-only migrations, automatic pre-upgrade `pg_dump`, restore plus previous image; refuse to start when the database is newer than the image. No down migrations. | Yes |
| D-5 | Content keys referenced by durable state (§3.3) become a contract. On reseed, hard-FK breakage aborts the start. Do dangling soft references abort or warn? Should CI fail a content PR that deletes or renumbers a referenced key, or widens a CHECK-guarded range, without a durable migration? | Abort on hard FKs; warn on soft references in P1, fail in P2; CI fails the PR |
| D-6 | Move enum types used by durable tables (today `EAmmoType`) out of `resources`, into `public` or a separate `shared` schema owned by migrations, accepting the costs in §6.6. Lint against new `resources` type references in `db/sgw/`. | `public`, plus the lint |
| D-7 | Bootstrap rows in the persistent profile: load the dev GM accounts, seeded characters, inventory, mail and contacts, or only `shards`? How do accounts get created on a persistent server? And the password risk: the seeded accounts are unsalted SHA-1 (`password_algo` 1), and an account moves to argon2id only when it logs in over TLS, so a persistent database can keep SHA-1 hashes indefinitely. | Only `shards` plus one operator-named admin account from `.env`, created directly as argon2id; add an account-provisioning path that writes argon2id before colo cut-over |
| D-8 | Watchtower in the persistent profile: off, with pinned tags and manual upgrades, or on, with unattended migration. | Off |
| D-9 | Colo: does it become persistent, from which release (the cut-over wipe), with what backup retention and where, and who announces it? | Yes, after P2 is green for one release cycle |
| D-10 | CI: is the PR-level SQL upgrade job blocking for PRs that touch `db/`, and does the release-level container smoke block promotion to `latest-prerelease`? | Both blocking |
| D-11 | Content reseed on every persistent start, or only when the image's content fingerprint differs from the database's? | Every start in P1 (simplest, same as today); add the fingerprint skip if start time hurts |
| D-12 | Content copied into durable rows (§3.4): does a content PR that changes a start kit, a granted ability, an item's `ammo_types` or similar owe a backfill migration for existing characters, or are such fixes new-characters-only unless someone asks? | New-characters-only by default; a backfill migration when the fix is a correctness bug players hit, decided in the PR |
| D-13 | Migration numbering: sequential numbers, or timestamp versions with the uniqueness and release-ordering checks in §6.5. | Timestamp versions plus both checks |

## 11. Consequences and risks

- **For content authors:** nothing changes until a persistent database exists. After that, deleting or renumbering a key in §3.3, widening a CHECK-guarded range, or changing copied content (§3.4) may need a durable migration, and CI flags the first two.
- **For durable-schema authors:** one migration file and a regenerated stamp per DDL change, checked by CI.
- **For operators:** a supported persistent profile with documented install, upgrade, rollback and smoke steps. The disposable profile and its reseed-on-every-start stay the default.
- **For tests:** live-DB tests keep loading `database.sql`; the stamp comes along. The PR upgrade job adds one Postgres load of the previous release.
- **Risk: content fixes miss existing characters** (§3.4, D-12). Playtest feedback about start kits and grants gets slower to fix for testers who keep their characters.
- **Risk: legacy password hashes persist** (D-7). Any account that never logs in over TLS keeps an unsalted SHA-1 hash in a durable table. Whether the argon2id upgrade also clears the legacy `password` column should be checked before colo cut-over.
- **Risk: item renumbering through `UPDATE` cascades silently** into durable rows (§3.3).
- **Docs that change when this is built:** [container.md](../operations/container.md) and [colo-deploy.md](../operations/colo-deploy.md) (both named by #1290), [write-a-database-migration.md](../guides/write-a-database-migration.md), [db/README.md](../../db/README.md), and the rule sources D-2 lists.
- **Not covered:** multi-host or concurrent-writer deployments; Postgres major-version upgrades of the external server, which need an image with matching client tools first (§2); and the world-state persistence proposal in [server-infrastructure-proposals.md §3](server-infrastructure-proposals.md#3-world-state-persistence). That proposal's new tables would be durable tables under this ADR, with migrations from day one.
