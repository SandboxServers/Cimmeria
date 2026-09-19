//! Live-DB regression guards for the Command Center population seeded by
//! packet H12 / placement cluster PL-C
//! (`docs/analysis/harset-rebuild/placements/C-interiors-68-69-70.md`).
//!
//! Seed-data guards, not fixture tests: the rows under test are the production
//! seed in `db/resources/Worlds/Seed/spawnlist.sql`. Every test here fails if
//! the PL-C rows are removed.
//!
//! What they pin, and why each one is not already covered:
//!
//! - **The roster is exact.** The packet's acceptance is "exactly these rows
//!   plus Anat spawn for world 68". An extra row is as much a defect as a
//!   missing one: world 68 is a *shared* space, so a stray spawn is visible to
//!   every player forever.
//! - **Nothing in 68 is hostile** (D-H03/D-H04). Faction cannot be changed at
//!   runtime, and right-click on an alive faction-10 NPC is rerouted from
//!   dialog to auto-attack, so a hostile row here would make the council room's
//!   quest-givers unspeakable rather than merely dangerous.
//! - **Every row is `is_stationary`** (D-H06). World 68 has no navmesh, so an
//!   NPC that tried to path would take the `no_path` branch in `npc_ai/fight`
//!   and freeze mid-pursuit.
//! - **The tags match the registry byte for byte.** `interact_tag` chains match
//!   on exact string equality; six chains already shipped in
//!   `harset_jaffa_chains.sql` and `harset_opcore_chains.sql` key on these
//!   strings and were inert until these rows landed.
//! - **No heading is 0.** The Castle reconstruction shipped a full packet of
//!   heading-0 rows that faced walls; this is the guard that stops PL-C
//!   repeating it.
//! - **The loader actually returns them.** Counting DB rows proves the seed;
//!   only a `load_spawns_from_db` round-trip proves the cell would spawn them.

use super::*;

/// World 68 = Harset_CmdCenter, confirmed against `resources.worlds`.
const CMDCENTER_WORLD_ID: i32 = 68;

/// `resources.worlds.world` for world 68 — the string `load_spawns_from_db`
/// returns in `SpawnRecord::world_name`.
const CMDCENTER_WORLD_NAME: &str = "Harset_CmdCenter";

/// The complete world-68 roster after PL-C, as `(spawn_id, template_id, tag)`.
///
/// Spawn 222 (Anat) predates the packet and keeps its coordinates; PL-C only
/// fills its `tag`, which the registry requires be `CmdCenter_Anat` on the
/// existing row rather than on a second spawn.
const CMDCENTER_ROSTER: [(i32, i32, &str); 11] = [
    (222, 43, "CmdCenter_Anat"),
    (300, 42, "CmdCenter_Baal"),
    (301, 209, "CmdCenter_RoyalGuard"),
    (302, 245, "CmdCenter_SymbioteTank"),
    (303, 54, "CmdCenter_Mohkatan"),
    (304, 10, "CmdCenter_Marsh"),
    (305, 48, "CmdCenter_Copplemann"),
    (306, 214, "CmdCenter_Blackstock"),
    (307, 53, "CmdCenter_Nerus"),
    (308, 215, "CmdCenter_Opheltes"),
    (309, 44, "CmdCenter_Athena"),
];

/// Tags that a shipped `content_chain_triggers` row already keys on. These are
/// the ones whose absence makes an authored mission beat unreachable rather
/// than merely making the room emptier.
const TAGS_WITH_LIVE_CHAINS: [&str; 4] = [
    "CmdCenter_Baal",
    "CmdCenter_Mohkatan",
    "CmdCenter_Marsh",
    "CmdCenter_Copplemann",
];

/// **The roster is exactly these eleven rows.**
///
/// Fails if a PL-C row is deleted (missing id), if its template is swapped, or
/// if anyone adds a twelfth spawn to the shared council room.
#[tokio::test]
async fn world_68_holds_exactly_the_pl_c_roster_plus_anat() {
    let pool = require_db_or_skip!();

    let rows: Vec<(i32, i32, Option<String>)> = sqlx::query_as(
        "SELECT spawn_id, template_id, tag FROM resources.spawnlist \
         WHERE world_id = $1 ORDER BY spawn_id",
    )
    .bind(CMDCENTER_WORLD_ID)
    .fetch_all(&pool)
    .await
    .expect("query world 68 spawns");

    let actual: Vec<(i32, i32, String)> = rows
        .into_iter()
        .map(|(id, tid, tag)| (id, tid, tag.unwrap_or_default()))
        .collect();
    let expected: Vec<(i32, i32, String)> = CMDCENTER_ROSTER
        .iter()
        .map(|(id, tid, tag)| (*id, *tid, (*tag).to_string()))
        .collect();

    assert_eq!(
        actual, expected,
        "world 68 roster drifted. Every tag here is matched byte-exactly by an \
         `interact_tag` chain or by `docs/analysis/harset-rebuild/worknotes/\
         harset-tags.md`; a missing row makes a shipped mission beat \
         unreachable and an extra one is permanently visible to every player, \
         because 68 is a shared space and not an instance.",
    );
}

/// **Nothing in world 68 is hostile, and nothing there can be a kill target.**
///
/// Decision D-H03/D-H04. The check is on the *template*, because
/// `spawnlist` has no faction column and faction is not runtime-mutable: a
/// faction-10 row in the council room would reroute right-click from dialog to
/// auto-attack and silence the quest-giver it is attached to.
#[tokio::test]
async fn no_world_68_spawn_uses_a_hostile_template() {
    let pool = require_db_or_skip!();

    let offenders: Vec<(i32, i32, Option<String>)> = sqlx::query_as(
        "SELECT s.spawn_id, s.template_id, t.template_name \
         FROM resources.spawnlist s \
         JOIN resources.entity_templates t ON t.template_id = s.template_id \
         WHERE s.world_id = $1 AND t.faction = $2 \
         ORDER BY s.spawn_id",
    )
    .bind(CMDCENTER_WORLD_ID)
    .bind(i32::from(HOSTILE_FACTION))
    .fetch_all(&pool)
    .await
    .expect("query world 68 factions");

    assert!(
        offenders.is_empty(),
        "world 68 is the shared council room and D-H03 forbids a hostile NPC \
         in it; these rows use faction {}: {offenders:?}",
        HOSTILE_FACTION,
    );
}

/// **Every world-68 row is stationary, carries a respawn delay, and faces
/// somewhere.**
///
/// Three properties in one sweep because they share a failure shape — a row
/// added later that copies the two-column `INSERT` form used elsewhere in the
/// seed gets `is_stationary = false`, `respawn_secs = NULL` and, if the author
/// does not derive one, `heading = 0`.
///
/// The heading clause is the Castle lesson (`METHOD.md`): the Castle
/// reconstruction shipped rows that all had heading 0 and faced walls. Exact
/// equality with 0.0 is the right test — a *derived* heading of due-+Z would
/// also be 0.0, so PL-C deliberately never derives one, and any future 0.0 in
/// world 68 is an author who skipped the step rather than one who measured it.
#[tokio::test]
async fn world_68_rows_are_stationary_respawning_and_not_heading_zero() {
    let pool = require_db_or_skip!();

    let rows: Vec<(i32, bool, Option<i32>, f32)> = sqlx::query_as(
        "SELECT spawn_id, is_stationary, respawn_secs, heading \
         FROM resources.spawnlist WHERE world_id = $1 ORDER BY spawn_id",
    )
    .bind(CMDCENTER_WORLD_ID)
    .fetch_all(&pool)
    .await
    .expect("query world 68 flags");

    assert_eq!(
        rows.len(),
        CMDCENTER_ROSTER.len(),
        "precondition: world 68 must hold the full roster before these \
         per-row properties mean anything",
    );

    for (spawn_id, stationary, respawn, heading) in rows {
        assert!(
            stationary,
            "spawn {spawn_id}: world 68 has no navmesh, so a non-stationary \
             NPC takes the `no_path` branch and freezes (D-H06)",
        );
        assert_eq!(
            respawn,
            Some(30),
            "spawn {spawn_id}: every Harset row sets `respawn_secs` (D-H17); \
             NULL here would make it one-shot until a server restart",
        );
        assert_ne!(
            heading, 0.0,
            "spawn {spawn_id}: heading 0 is the Castle reconstruction defect \
             -- derive it from the doorway or approach direction, never leave \
             the column at its default",
        );
    }
}

/// **The four tags a shipped chain keys on exist, and in world 68.**
///
/// `interact_tag` matches on exact string equality with no prefix matching, so
/// a misspelled or absent tag is a mission beat that silently does nothing.
/// H30 records `CmdCenter_Marsh` in particular as a hard constraint on the
/// placement pass, because 1361 reaches Marsh twice.
///
/// Uniqueness is asserted too, but it is the schema that guarantees it —
/// `spawnlist` carries a `UNIQUE` constraint on `tag`, so a second
/// `CmdCenter_Marsh` cannot be seeded at all. The `len() == 1` below is a cheap
/// pin on that constraint still existing, not the guard's reason for being: if
/// the constraint were ever dropped, `find_entity_by_tag` returns the first
/// match and which Marsh answers a mission would depend on spawn order.
#[tokio::test]
async fn chain_referenced_cmdcenter_tags_are_unique_and_present() {
    let pool = require_db_or_skip!();

    for tag in TAGS_WITH_LIVE_CHAINS {
        let ids: Vec<(i32, i32)> = sqlx::query_as(
            "SELECT spawn_id, world_id FROM resources.spawnlist WHERE tag = $1 \
             ORDER BY spawn_id",
        )
        .bind(tag)
        .fetch_all(&pool)
        .await
        .expect("query tag");

        assert_eq!(
            ids.len(),
            1,
            "tag `{tag}` must resolve to exactly one spawn -- a shipped \
             `content_chain_triggers` row keys on it, so zero rows means an \
             inert mission beat, and more than one means the `UNIQUE` \
             constraint on `spawnlist.tag` has been dropped and \
             `find_entity_by_tag` now picks a winner by spawn order; \
             found {ids:?}",
        );
        assert_eq!(
            ids[0].1, CMDCENTER_WORLD_ID,
            "tag `{tag}` must live in world {CMDCENTER_WORLD_ID}",
        );
    }
}

/// **`load_spawns_from_db` actually returns the roster.**
///
/// The row-count guards above prove the seed; this proves the cell would spawn
/// it. The loader joins `entity_templates` and drops any row whose template is
/// missing, so a template-id typo in the seed would pass every DB-only
/// assertion and still leave the council room empty at runtime.
#[tokio::test]
async fn the_spawn_loader_returns_the_full_cmdcenter_roster() {
    let pool = require_db_or_skip!();

    let spawns = load_spawns_from_db(&pool)
        .await
        .expect("load_spawns_from_db");

    let mut loaded: Vec<(i32, i32, String)> = spawns
        .iter()
        .filter(|s| s.world_name == CMDCENTER_WORLD_NAME)
        .map(|s| (s.spawn_id, s.template_id, s.tag.clone().unwrap_or_default()))
        .collect();
    loaded.sort();

    let expected: Vec<(i32, i32, String)> = CMDCENTER_ROSTER
        .iter()
        .map(|(id, tid, tag)| (*id, *tid, (*tag).to_string()))
        .collect();

    assert_eq!(
        loaded, expected,
        "the loader dropped or altered a world-68 row. It inner-joins \
         `entity_templates`, so the usual cause is a template id in the seed \
         that no template row matches -- which every DB-only count in this \
         file would happily accept.",
    );

    for record in spawns
        .iter()
        .filter(|s| s.world_name == CMDCENTER_WORLD_NAME)
    {
        assert!(
            record.is_stationary,
            "spawn {}: the loader must carry `is_stationary` through to the \
             spawn path, or D-H06 is documented but not enforced",
            record.spawn_id,
        );
    }
}
