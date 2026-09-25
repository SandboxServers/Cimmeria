//! Harset H13 live-DB regression guards for the hub spawn data.
//!
//! These tests guard the seed edit made by Harset rebuild packet **H13**
//! (`db/resources/Worlds/Seed/spawnlist.sql`) against defect **H-B7**:
//! before H13, `respawn_secs` was NULL on every one of the 167 spawn rows
//! and all 153 templates, so [`crate::cell::combat::mark_npc_dead`] never
//! stamped `respawn_at` and the [`super::super::npc_respawn_tick`] had
//! nothing to promote. Killing the eight Praxis guards emptied the Harset
//! gate plaza until a server restart.
//!
//! They live here rather than in `spawner/tests/` because the whole point
//! is the *chain*, not the loader: seed row → `load_spawns_from_db`'s
//! `COALESCE(spawnlist.respawn_secs, entity_templates.respawn_secs)` →
//! `spawn_npc_from_record` → `mark_npc_dead` → this tick. A loader-only
//! assertion would pass while the tick stayed broken.
//!
//! Revert sensitivity (TESTING.md type 3 — "a regression guard must fail
//! when the fix is reverted"):
//!
//! - Drop the `respawn_secs` column values from the seed and
//!   [`harset_hub_mob_spawns_carry_the_documented_respawn_delay`] and
//!   [`killed_harset_guard_is_stamped_and_revived_by_the_respawn_tick`]
//!   both fail (`Some(30)` → `None`, then `respawn_at` → `None`).
//! - Drop `is_stationary` and
//!   [`harset_gate_and_door_sentries_are_stationary`] fails.
//! - Restore the two debug INSERTs and
//!   [`harset_debug_spawns_are_absent_from_the_production_world`] fails.
//!
//! Two more guard the *blast radius* rather than the revert, so do not try
//! to falsify them by reverting — they hold either way, by design:
//!
//! - [`harset_prop_spawns_stay_one_shot`] catches the over-correction
//!   (`UPDATE spawnlist SET respawn_secs = 30 WHERE world_id = 57`), which
//!   would put a respawn timer on a DHD and five ring switches.
//! - [`harset_respawn_delay_is_written_on_the_spawn_row_and_spares_castle`]
//!   is the one that keeps this whole file honest. Every other test reads
//!   the delay through the loader's COALESCE, which a *template*-level
//!   value also satisfies — and packet H11 has put `respawn_secs = 300` on
//!   templates 159/160. That test reads `spawnlist.respawn_secs` raw, so
//!   no template value can stand in for the seed edit, and it checks that
//!   Castle's four world-8 spawn rows on those same templates were not
//!   touched.
//!
//! Every test asserts the rows *exist* before asserting their contents, so
//! an unseeded or half-loaded database fails loudly instead of passing
//! vacuously on an empty result set. The absence assertions additionally
//! prove the world-57 slice loaded, since "no debug row here" is worthless
//! if world 57 never arrived.
//!
//! # These guards pin the data, not the reachability
//!
//! [`killed_harset_guard_is_stamped_and_revived_by_the_respawn_tick`]
//! calls [`mark_npc_dead`] directly, the way the damage path does at its
//! kill site. It deliberately does **not** go through `use_ability`,
//! because no Harset NPC is damageable today: a harmful ability is
//! refused unless the target's faction is
//! `crate::cell::combat::HOSTILE_FACTION` (10), and templates 43 / 159 /
//! 160 all ship `faction = 1` while 163 (Petbe) is NULL. Flipping the
//! sentry templates to faction 10 belongs to packet H11.
//!
//! The ordering that matters: **H11's faction change must not land before
//! this packet's seed edit.** Hostile-but-one-shot re-creates H-B7 on its
//! own — the first player to clear the plaza empties it until restart.

use super::super::*;
use crate::cell::combat::mark_npc_dead;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{load_spawns_from_db, SpawnRecord};
use crate::test_support::require_db_or_skip;
use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::stats::HEALTH;

/// `resources.worlds.world` for world_id 57 — the shared Harset hub.
const HARSET: &str = "Harset";
/// `resources.worlds.world` for world_id 68 — the Command Center.
const HARSET_CMD_CENTER: &str = "Harset_CmdCenter";

/// The single documented respawn default H13 applies to every Harset mob
/// row. Sourced from the `respawn_secs` column comment in
/// `db/resources/Entities/Tables/entity_templates.sql`: "Recommended floor
/// for typical mobs is 30 seconds; bosses 300+". Not an invented number —
/// if that recommendation changes, the seed and this constant move
/// together.
const HARSET_MOB_RESPAWN_SECS: u32 = 30;

/// The eight Praxis Jaffa Guards (template 160) flanking the stargate
/// plaza, and the four Praxis Jaffa Lieutenants (template 159) posted at
/// the two door pairs (z ≈ -188 and z ≈ -231). All twelve are gate/door
/// sentries: they hold a post, they do not roam.
const HARSET_SENTRY_SPAWN_IDS: [i32; 12] = [
    // Template 160 — plaza guards.
    225, 226, 227, 228, 229, 230, 231, 234, //
    // Template 159 — door lieutenants.
    232, 233, 235, 236,
];

/// Non-combat scenery on the Harset rows: the DHD (template 1), the five
/// ring transporter switches (template 3) and the "FirstBug" merchant
/// basket (template 164). Props are never killed, so a `respawn_secs`
/// here would be dead data that a future reader would mistake for intent.
const HARSET_PROP_SPAWN_IDS: [i32; 7] = [37, 4, 127, 128, 129, 130, 224];

/// The two debug rows H13 removes from the production seed. Spawn 1 is
/// template 23 "Loot debug item"; spawn 42 is template 25 "Interaction
/// Debug NPC - DO NOT USE". Both stood on the gate plaza, in the player's
/// face on arrival. `spawnlist` has no `enabled`/`dev` column, so removal
/// was the only available gate.
const HARSET_DEBUG_SPAWN_IDS: [i32; 2] = [1, 42];
/// Template ids behind [`HARSET_DEBUG_SPAWN_IDS`]. Asserted separately so
/// a re-added debug row under a *different* `spawn_id` is still caught.
const HARSET_DEBUG_TEMPLATE_IDS: [i32; 2] = [23, 25];

/// Load every spawn record and keep the ones in the two shared Harset
/// worlds. Fails loudly when the join returns nothing for Harset — an
/// empty vector must never be mistaken for "the assertion held".
async fn harset_records(pool: &sqlx::PgPool) -> Vec<SpawnRecord> {
    let records = load_spawns_from_db(pool)
        .await
        .expect("load_spawns_from_db must succeed against the seeded DB");
    let harset: Vec<SpawnRecord> = records
        .into_iter()
        .filter(|r| r.world_name == HARSET || r.world_name == HARSET_CMD_CENTER)
        .collect();
    assert!(
        !harset.is_empty(),
        "no spawn rows loaded for worlds {HARSET} / {HARSET_CMD_CENTER} — the DB is \
         not seeded from db/database.sql, so every assertion below would pass \
         vacuously. Load the seed before trusting this run.",
    );
    harset
}

fn record_for(records: &[SpawnRecord], spawn_id: i32) -> &SpawnRecord {
    records
        .iter()
        .find(|r| r.spawn_id == spawn_id)
        .unwrap_or_else(|| {
            panic!(
                "Harset spawn_id {spawn_id} is missing from the loaded records — \
                 the seed row was deleted or moved out of world 57/68"
            )
        })
}

/// A minimal SpaceManager carrying the real Harset AABB from
/// `entities/spaces.xml`, so a record spawned into it lands in a space
/// whose bounds match production.
fn harset_space_manager() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Harset" Instanced="false" MinX="-1000" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .expect("Harset space XML must parse");
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Harset" /></Spaces>"#,
    )
    .expect("Harset startup space must be created");
    mgr
}

/// Every Harset mob row resolves a respawn delay through the loader's
/// COALESCE. This is the H-B7 guard proper: with the seed reverted the
/// COALESCE collapses to `None` and the mob is one-shot forever.
///
/// Note the per-spawn (not per-template) placement matters here —
/// templates 159 and 160 are *shared* with the Castle hub (world 8 spawns
/// 119/121/123/124). Putting the value on the template would silently
/// change Castle's guards too; the per-spawn override keeps the blast
/// radius inside Harset and wins the COALESCE over whatever H11 later
/// puts on the template.
#[tokio::test]
async fn harset_hub_mob_spawns_carry_the_documented_respawn_delay() {
    let pool = require_db_or_skip!();
    let records = harset_records(&pool).await;

    for spawn_id in HARSET_SENTRY_SPAWN_IDS {
        let r = record_for(&records, spawn_id);
        assert_eq!(
            r.respawn_secs,
            Some(HARSET_MOB_RESPAWN_SECS),
            "Harset sentry spawn {spawn_id} ({}) must resolve respawn_secs = {HARSET_MOB_RESPAWN_SECS}; \
             got {:?}. NULL here is defect H-B7 — mark_npc_dead never stamps respawn_at and the \
             plaza depopulates permanently on first clear.",
            r.template_name,
            r.respawn_secs,
        );
    }
}

/// The two named story NPCs on the shared Harset rows — Petbe (spawn 223,
/// template 163) in the hub and Anat (spawn 222, template 43) in the
/// Command Center — must also respawn. They are talk-only in normal play,
/// but a griefer who kills Anat would otherwise break missions 1200, 1324
/// and 742 for every other player on the shard until a restart. Separate
/// from the sentry test so a regression names the right culprit.
#[tokio::test]
async fn harset_named_story_npcs_respawn_rather_than_staying_dead() {
    let pool = require_db_or_skip!();
    let records = harset_records(&pool).await;

    for (spawn_id, who) in [(223_i32, "Petbe"), (222_i32, "Anat")] {
        let r = record_for(&records, spawn_id);
        assert_eq!(
            r.respawn_secs,
            Some(HARSET_MOB_RESPAWN_SECS),
            "{who} (spawn {spawn_id}, template {}) must resolve the documented \
             {HARSET_MOB_RESPAWN_SECS}s delay — a permanently-dead shared story NPC \
             blocks every player's mission chain, and a fat-fingered 3000 would be \
             indistinguishable from that at the table",
            r.template_id,
        );
    }
}

/// Props carry no respawn delay. Guards the narrow edit against the
/// obvious over-correction: a blanket `respawn_secs` across all 23 Harset
/// rows would schedule respawn timers for a DHD and five ring switches
/// that can never die, which reads as intent to a future author.
#[tokio::test]
async fn harset_prop_spawns_stay_one_shot() {
    let pool = require_db_or_skip!();
    let records = harset_records(&pool).await;

    for spawn_id in HARSET_PROP_SPAWN_IDS {
        let r = record_for(&records, spawn_id);
        assert_eq!(
            r.respawn_secs, None,
            "Harset prop spawn {spawn_id} ({}) must leave respawn_secs NULL — props \
             are never killed, so a delay here is dead data",
            r.template_name,
        );
    }
}

/// The delay is written on the **spawn rows themselves**, and H13 reaches
/// no row outside Harset.
///
/// This is the guard that keeps the rest of this file honest, and it is the
/// only one that reads `spawnlist.respawn_secs` raw instead of through the
/// loader. Everything else in this file sees
/// `COALESCE(spawnlist.respawn_secs, entity_templates.respawn_secs)`
/// (`cell/spawner/npcs.rs:149`), so a template-level value alone can satisfy
/// them — and packet **H11 has in fact put `respawn_secs = 300` on templates
/// 159 and 160**. Without this test, reverting all fourteen spawn-row values
/// would leave the loader resolving 300, and only the exact-value assertions
/// would notice. Reading the column directly makes the seed edit itself the
/// thing under test: no template value can ever produce a 30 here.
///
/// The second half is the blast-radius check on the placement rationale.
/// Templates 159 and 160 are shared with the Castle hub (world 8, spawns
/// 119 / 121 / 123 / 124), which is exactly why H13 wrote per-spawn rather
/// than per-template. This asserts those four Castle **spawn rows** stay
/// untouched. It deliberately says nothing about what they resolve through
/// the COALESCE — that is downstream of H11's template value and a decision
/// for the Castle campaign, not something H13 should pin.
#[tokio::test]
async fn harset_respawn_delay_is_written_on_the_spawn_row_and_spares_castle() {
    use sqlx::Row;

    let pool = require_db_or_skip!();
    let sentry_ids: Vec<i32> = HARSET_SENTRY_SPAWN_IDS.to_vec();
    let castle_ids: Vec<i32> = vec![119, 121, 123, 124];

    let rows = sqlx::query(
        "SELECT spawn_id, world_id, respawn_secs, is_stationary \
         FROM resources.spawnlist WHERE spawn_id = ANY($1) ORDER BY spawn_id",
    )
    .bind(&sentry_ids)
    .fetch_all(&pool)
    .await
    .expect("spawnlist lookup must succeed against the seeded DB");

    assert_eq!(
        rows.len(),
        HARSET_SENTRY_SPAWN_IDS.len(),
        "expected all {} Harset sentry rows in resources.spawnlist, found {} — the DB \
         is not seeded, so the assertions below would pass vacuously",
        HARSET_SENTRY_SPAWN_IDS.len(),
        rows.len(),
    );
    for row in &rows {
        let spawn_id: i32 = row.get("spawn_id");
        let raw: Option<i32> = row.get("respawn_secs");
        assert_eq!(
            raw,
            Some(HARSET_MOB_RESPAWN_SECS as i32),
            "spawn {spawn_id} must carry respawn_secs on its own spawnlist row, not \
             inherit one from its template — a template-level value would satisfy the \
             loader's COALESCE with this seed edit reverted",
        );
    }

    // Castle's four Praxis spawn rows on the same two templates: H13 must
    // not have reached them.
    let castle = sqlx::query(
        "SELECT spawn_id, respawn_secs, is_stationary FROM resources.spawnlist \
         WHERE spawn_id = ANY($1) ORDER BY spawn_id",
    )
    .bind(&castle_ids)
    .fetch_all(&pool)
    .await
    .expect("Castle spawnlist lookup must succeed");

    assert_eq!(
        castle.len(),
        castle_ids.len(),
        "expected Castle spawns 119/121/123/124 (templates 159/160) to exist"
    );
    for row in &castle {
        let spawn_id: i32 = row.get("spawn_id");
        let raw: Option<i32> = row.get("respawn_secs");
        let stationary: bool = row.get("is_stationary");
        assert_eq!(
            raw, None,
            "Castle spawn {spawn_id} picked up a per-spawn respawn_secs — H13 owns \
             Harset rows only and must never write a world-8 row",
        );
        assert!(
            !stationary,
            "Castle spawn {spawn_id} picked up is_stationary — H13 owns Harset rows only",
        );
    }
}

/// Policy closure, so this suite grows with the zone instead of going
/// stale one packet from now.
///
/// The assertions above walk two hand-written constant arrays that happen
/// to cover today's 14 mob rows. H12 will add ~20 rows to world 68 and H14
/// more to world 57; none of them would be guarded by those arrays, and
/// nothing would say so. This test keys on `class` instead, so every mob
/// row any future packet seeds into Harset falls under D-H17 ("every
/// Harset spawn row sets `respawn_secs`") and D-H06 (stationary until GH1)
/// automatically.
///
/// A new mob row that legitimately wants different treatment fails here
/// and has to argue for it, which is the intended friction.
#[tokio::test]
async fn every_harset_mob_row_carries_respawn_and_stationary_data() {
    let pool = require_db_or_skip!();
    let records = harset_records(&pool).await;

    let mob_rows: Vec<&SpawnRecord> = records.iter().filter(|r| r.class == "mob").collect();
    for r in &mob_rows {
        assert!(
            r.respawn_secs.is_some(),
            "Harset mob spawn {} ({}, template {}) has no respawn delay — D-H17 says \
             every Harset spawn row sets one, or the zone depopulates permanently \
             (defect H-B7)",
            r.spawn_id,
            r.template_name,
            r.template_id,
        );
        assert!(
            r.is_stationary,
            "Harset mob spawn {} ({}, template {}) is not stationary — D-H06 holds \
             every Harset NPC in place until GH1 rebuilds harset.nav, because a chase \
             across a mesh component boundary freezes the NPC",
            r.spawn_id, r.template_name, r.template_id,
        );
    }

    // Floor, not an equality: H12 and H14 will add rows and this test is
    // meant to absorb them. The floor only has to be tight enough that an
    // empty or class-renamed result set can't pass.
    let floor = HARSET_SENTRY_SPAWN_IDS.len() + 2; // 12 sentries + Petbe + Anat
    assert!(
        mob_rows.len() >= floor,
        "only {} Harset rows have class = 'mob', expected at least {floor} — either \
         the seed is incomplete or `entity_templates.class` changed spelling, and this \
         test silently stopped covering anything",
        mob_rows.len(),
    );
}

/// The twelve gate and door sentries are pinned to their posts.
///
/// This is an interim measure, not a design preference: per decision
/// D-H06, `harset.nav` currently has 1,939 disconnected components and an
/// NPC that aggros across a component boundary hits the `no_path` outcome
/// in `npc_ai/fight.rs` and freezes mid-chase. `is_stationary` routes the
/// AI down the hold-position-and-fire branch instead, which is correct
/// behavior for a sentry and survives GH1's navmesh rebuild.
#[tokio::test]
async fn harset_gate_and_door_sentries_are_stationary() {
    let pool = require_db_or_skip!();
    let records = harset_records(&pool).await;

    for spawn_id in HARSET_SENTRY_SPAWN_IDS {
        let r = record_for(&records, spawn_id);
        assert!(
            r.is_stationary,
            "Harset sentry spawn {spawn_id} ({}) must be stationary until GH1 rebuilds \
             harset.nav — a chase across a mesh component boundary freezes the NPC",
            r.template_name,
        );
    }
}

/// The two named story NPCs are stationary too. Asserted separately from
/// the twelve sentries because the rationale differs: Anat's world 68 has
/// no navmesh at all, and Petbe stands off-mesh on the fragmented world-57
/// mesh. Both reduce to "a pathfind from here returns None", which without
/// the flag means a silent freeze instead of the diagnosable
/// `stationary_holds` branch. H12 will extend the same rule to the ~20
/// rows it adds to world 68.
#[tokio::test]
async fn harset_named_story_npcs_are_stationary() {
    let pool = require_db_or_skip!();
    let records = harset_records(&pool).await;

    let anat = record_for(&records, 222);
    assert_eq!(
        anat.world_name, HARSET_CMD_CENTER,
        "spawn 222 (Anat) must still be in {HARSET_CMD_CENTER}"
    );
    assert!(
        anat.is_stationary,
        "Anat (spawn 222) must be stationary — world 68 has no navmesh, so any \
         pathfind attempt fails closed"
    );

    let petbe = record_for(&records, 223);
    assert_eq!(
        petbe.world_name, HARSET,
        "spawn 223 (Petbe) must still be in {HARSET}"
    );
    assert!(
        petbe.is_stationary,
        "Petbe (spawn 223) must be stationary — he stands off-mesh on the \
         fragmented harset.nav"
    );
}

/// The debug rows are gone from the production seed.
///
/// `resources.spawnlist` has no `enabled` / `dev` column, so there was no
/// flag to gate them behind — deletion was the only lever. Both templates
/// remain in `entity_templates.sql` (a template with no spawn row is
/// inert), so a GM can still `.spawn` them on demand.
///
/// Scope note: this scans the two Harset worlds only. Re-adding template
/// 23 or 25 to some *other* zone would not be caught here — that would
/// need a workspace-wide assertion, which is out of H13's scope.
#[tokio::test]
async fn harset_debug_spawns_are_absent_from_the_production_world() {
    let pool = require_db_or_skip!();
    let records = harset_records(&pool).await;

    // Both debug rows were world-57 rows, and `harset_records` only proves
    // that *a* Harset world loaded — one surviving world-68 row satisfies
    // it. Prove the world-57 slice arrived before reading anything into an
    // absence. Spawn 37 is the DHD: a world-57 row H13 does not touch, so
    // it won't drift out from under this check.
    record_for(&records, 37);

    for spawn_id in HARSET_DEBUG_SPAWN_IDS {
        assert!(
            !records.iter().any(|r| r.spawn_id == spawn_id),
            "debug spawn {spawn_id} is back in a Harset world — it stands on the gate \
             plaza where every arriving player sees it"
        );
    }
    // Catch a re-add under a different spawn_id too.
    for template_id in HARSET_DEBUG_TEMPLATE_IDS {
        let offenders: Vec<i32> = records
            .iter()
            .filter(|r| r.template_id == template_id)
            .map(|r| r.spawn_id)
            .collect();
        assert!(
            offenders.is_empty(),
            "debug template {template_id} is spawned in a Harset world by spawn(s) \
             {offenders:?} — templates 23 (\"Loot debug item\") and 25 (\"Interaction \
             Debug NPC - DO NOT USE\") must not appear in production zones"
        );
    }
}

/// End-to-end: a real Harset guard record, spawned through the production
/// path, killed, then promoted by the respawn tick.
///
/// This is the test that actually proves the chain works rather than that
/// a column is populated. The revert-sensitive step is the
/// `.expect("respawn_at must be stamped")` — with `respawn_secs` back to
/// NULL, `mark_npc_dead` leaves `respawn_at` as `None` and the test fails
/// there, before the deadline rewind that drives the tick.
#[tokio::test]
async fn killed_harset_guard_is_stamped_and_revived_by_the_respawn_tick() {
    let pool = require_db_or_skip!();
    let records = harset_records(&pool).await;
    // Spawn 225 — a plaza guard flanking the stargate, template 160.
    let guard = record_for(&records, 225).clone();

    let mut mgr = harset_space_manager();
    let npc_id = 9001u32;
    mgr.spawn_npc_from_record(npc_id, &guard)
        .expect("Harset guard record must spawn into the Harset space");

    // Kill it the way the damage path does.
    let respawn_secs = {
        let npc = mgr.get_entity_mut(npc_id).expect("guard must exist");
        if let Some(hp) = npc.stats.get_mut(HEALTH) {
            hp.set_current(0);
        }
        mark_npc_dead(npc, "Harset");
        assert_eq!(
            npc.ai_state(),
            AiState::Dead,
            "kill must set ai_state = Dead"
        );
        // H-B7 guard: this is `None` whenever the seed's respawn_secs is
        // reverted, and the expect below is where that regression lands.
        let stamped = npc
            .respawn_at
            .expect("respawn_at must be stamped on death — spawnlist.respawn_secs is NULL");
        let secs = npc
            .respawn_secs
            .expect("respawn_secs must survive onto the entity");
        // Rewind the *stamped* deadline rather than writing a fresh one,
        // so the assertion above stays the only thing standing between a
        // reverted seed and a green test.
        npc.respawn_at = Some(stamped - std::time::Duration::from_secs(secs as u64 + 1));
        secs
    };
    assert_eq!(
        respawn_secs, HARSET_MOB_RESPAWN_SECS,
        "the guard's resolved delay must be the documented Harset default"
    );

    let (tx, _rx) = mpsc::channel(64);
    npc_respawn_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(npc_id).expect("guard must still exist");
    assert_eq!(
        npc.ai_state(),
        AiState::Idle,
        "the respawn tick must promote the guard back to Idle"
    );
    let hp = npc
        .stats
        .get(HEALTH)
        .expect("guard must have a HEALTH stat");
    assert_eq!(hp.cur, hp.max, "respawn must restore the guard to full HP");
    assert!(
        npc.respawn_at.is_none(),
        "respawn_at must be consumed by the promotion"
    );
    assert_eq!(
        npc.respawn_secs,
        Some(HARSET_MOB_RESPAWN_SECS),
        "respawn_secs must persist so the next death re-schedules"
    );
}
