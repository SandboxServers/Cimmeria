//! Unit tests for the entity-lifecycle action handlers (Harset H03).
//!
//! These call `spawn_entity` / `despawn_by_tag` directly rather than via
//! `execute_actions`, so each guard is pinned at the level that produces
//! the signal. The dispatch wiring itself is covered by the chain-replay
//! fixture in `content/chain_replay_tests/harset_spawn_entity.rs`, which
//! goes through `execute_actions` — a resolve-only or handler-only test
//! cannot tell a wired match arm from the `other =>` catch-all.
//!
//! Bug shapes guarded here, in order of how expensive they'd be in
//! production:
//!
//! - **Wrong space.** A spawn that resolves the space from anything other
//!   than the acting player lands mission NPCs in someone else's instance,
//!   or in the shared hub. Two tests: the positive (player's own instanced
//!   space id) and the shared-world refusal.
//! - **Duplicate spawns.** Relog-restore chains re-fire by design; without
//!   the tag guard every relog stacks another copy of the mission NPC.
//! - **Silent no-op despawn (H-B6 / #582).** `destroy_entity` alone leaves
//!   the id in every witness set. The `LeftAoI` count and the scrubbed
//!   witness sets are both asserted — count alone passes if the sets are
//!   left dirty, and sets alone pass if nothing was ever sent.
//! - **Silent no-op `set_visible` (H-B5).** The pre-fix code built a
//!   correctly-shaped `EntityMethodCall` that base dropped on the floor for
//!   any NPC. Asserting message *construction* is what let that ship, so
//!   these assert the message *variant* and the per-witness cardinality.

use super::*;
use cimmeria_common::EntityId;

use crate::cell::spawner::SpawnRecord;

pub(in crate::cell::content::executor) const TEMPLATE_ID: i32 = 4242;
const SHARED_WORLD: &str = "Agnos";
const INSTANCED_WORLD: &str = "Castle_CellBlock";
const TAG: &str = "Harset_H03_Mala_c";

/// A `SpaceManager` with one shared startup world and one instanced world,
/// carrying a single cached template. Mirrors the real shape: `Agnos` is a
/// startup space, `Castle_CellBlock` allocates a fresh space per player.
fn make_space_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces>
        <Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" />
        <Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
    </Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.spawn_templates.insert(TEMPLATE_ID, template(10));
    mgr
}

/// A prototype `SpawnRecord` shaped like `load_spawn_templates` returns
/// one: spawn-instance fields are placeholders, template fields are set.
/// `respawn_secs` is deliberately non-`None` so the executor's
/// unconditional override has something to override.
pub(in crate::cell::content::executor) fn template(faction: i32) -> SpawnRecord {
    SpawnRecord {
        spawn_id: -1,
        world_name: String::new(),
        x: 0.0,
        y: 0.0,
        z: 0.0,
        heading: 0.0,
        tag: None,
        template_id: TEMPLATE_ID,
        template_name: "H03 Test Template".to_string(),
        class: "mob".to_string(),
        static_mesh: None,
        body_set: "BS_JaffaMale".to_string(),
        components: None,
        flags: 0,
        interaction_type: 0,
        event_set_id: None,
        level: Some(20),
        alignment: Some(1),
        faction: Some(faction),
        name_id: None,
        speaker_id: None,
        static_interaction_sets: Vec::new(),
        has_dynamic_properties: false,
        loot_table_id: None,
        is_stationary: false,
        ability_ids: vec![594],
        // The template opts into respawn; the executor must still spawn
        // one-shot. Inheriting this silently is the failure mode.
        respawn_secs: Some(30),
        patrol_path: Vec::new(),
        patrol_point_delay_secs: 2.0,
        wander_radius: 0.0,
        wander_min_dwell_secs: 3.0,
        wander_max_dwell_secs: 8.0,
        follow_min_distance: 2.0,
        follow_max_distance: 5.0,
        move_speed: 0.6,
        leash_distance: None,
        aggro_radius: None,
        aggression_override: None,
    }
}

/// Put a connected player into `world`. Returns the space id they landed
/// in (for an instanced world that is a freshly allocated instance).
fn stage_player(mgr: &mut SpaceManager, entity_id: u32, world: &str) -> u32 {
    mgr.create_entity(entity_id, world, [1.0, 2.0, 3.0], [0.0; 3])
        .expect("player entity must be creatable");
    let p = mgr
        .get_entity_mut(entity_id)
        .expect("player entity must exist immediately after create_entity");
    p.is_player = true;
    p.player_id = Some(42);
    mgr.connect_entity(entity_id);
    mgr.get_entity_space_id(entity_id)
        .expect("player must have a space")
}

/// Invoke the action with the mandatory fields and no overrides.
async fn spawn_default(mgr: &mut SpaceManager, actor: u32) {
    spawn_entity(
        TEMPLATE_ID,
        [10.0, 11.0, 12.0],
        1.5,
        TAG.to_string(),
        None,
        None,
        None,
        actor,
        6301,
        mgr,
    )
    .await;
}

fn npc_with_tag(mgr: &SpaceManager, actor: u32) -> Option<u32> {
    mgr.find_entity_by_tag(actor, TAG)
}

// ── spawn_entity ─────────────────────────────────────────────────────────

/// The load-bearing positive: the NPC must land in the **acting player's**
/// space and nowhere else.
///
/// Two players are staged in the same instanced world, which
/// `find_or_create_space` gives *separate* instances. That is what makes
/// "the acting player's space" distinguishable from "the world's space" —
/// with one player staged the two are the same id and a world-derived
/// regression would pass. The bystander's instance is asserted empty.
#[tokio::test]
async fn spawn_lands_in_the_acting_players_space_and_not_a_bystanders() {
    let mut mgr = make_space_mgr();
    let actor = 7001;
    let bystander = 7099;
    let space_id = stage_player(&mut mgr, actor, INSTANCED_WORLD);
    let bystander_space = stage_player(&mut mgr, bystander, INSTANCED_WORLD);
    assert_ne!(
        space_id, bystander_space,
        "fixture precondition: an instanced world must give each player its \
         own space, or this test cannot tell the two resolutions apart"
    );

    spawn_default(&mut mgr, actor).await;

    let npc_id = npc_with_tag(&mgr, actor).expect(
        "spawn_entity must place a tagged entity in the player's space -- \
         None here means either nothing spawned or the tag was dropped",
    );
    assert_eq!(
        mgr.get_entity_space_id(npc_id),
        Some(space_id),
        "the NPC must be in the acting player's own instance, not a \
         world-derived or newly-created space"
    );
    let npc = mgr
        .get_entity(npc_id)
        .expect("spawned NPC must be readable");
    assert_eq!(npc.tag.as_deref(), Some(TAG));
    assert_eq!(npc.template_id, Some(TEMPLATE_ID));
    assert_eq!(
        [npc.position.x, npc.position.y, npc.position.z],
        [10.0, 11.0, 12.0],
        "position comes from the action, not the template prototype"
    );
    assert_eq!(
        npc.direction.y, 1.5,
        "heading must reach direction.y (yaw); a dropped heading faces \
         every mission spawn the same way"
    );
    assert!(!npc.is_player, "a spawned template is never a player");

    assert!(
        mgr.find_entity_by_tag(bystander, TAG).is_none(),
        "the bystander's own instance must be untouched -- a world-derived \
         space resolution would populate whichever instance it found first"
    );

    // The spawn deliberately emits no cell→base traffic: client visibility
    // is the 100ms AoI tick's job, which recomputes each player's AoI from
    // the spatial grid and diffs it against their witness set.
    //
    // That contract only holds if the NPC actually reached the **grid**.
    // `find_entity_by_tag` reads `space.entities` alone, so every assertion
    // above passes even if `spawn_npc_from_record_into`'s
    // `space.space.add_entity(...)` call were dropped — and the result
    // would be an NPC no AoI tick can ever see. That is the #582
    // invisible-entity shape on the spawn side, in the packet that closes
    // it on the despawn side, so pin it through the real AoI path.
    let entered: Vec<u32> = mgr
        .compute_aoi_changes_for_player(actor)
        .into_iter()
        .filter_map(|msg| match msg {
            CellToBaseMsg::EnteredAoI { entity_id, .. } => Some(entity_id),
            _ => None,
        })
        .collect();
    assert!(
        entered.contains(&npc_id),
        "the AoI tick must introduce the spawned NPC to the acting player; \
         got {entered:?}. A miss here means the entity went into \
         space.entities but not into the spatial grid"
    );
}

/// A content-scoped spawn must be one-shot even when the template row opts
/// into respawn. A revived mission NPC re-fires its `entity_dead_tag`
/// chain, so a kill objective can complete twice — and the template value
/// would opt every mission spawn in *silently*.
#[tokio::test]
async fn spawn_never_inherits_template_respawn() {
    let mut mgr = make_space_mgr();
    let actor = 7002;
    stage_player(&mut mgr, actor, INSTANCED_WORLD);
    assert_eq!(
        mgr.spawn_templates[&TEMPLATE_ID].respawn_secs,
        Some(30),
        "fixture precondition: the template must opt into respawn, or this \
         test cannot observe the override"
    );

    spawn_default(&mut mgr, actor).await;

    let npc_id = npc_with_tag(&mgr, actor).expect("spawn must succeed");
    assert_eq!(
        mgr.get_entity(npc_id).unwrap().respawn_secs,
        None,
        "content spawns must be one-shot -- npc_respawn_tick keys purely on \
         (ai_state == Dead && respawn_at <= now), so any Some(_) here revives \
         the mission NPC inside the player's still-open instance"
    );
}

/// A content spawn is one-shot even when the *template* opts into respawn
/// — `template()` above carries `respawn_secs: Some(30)` precisely so this
/// has something to override. Inheriting it would hand a `spawn_id = -1`
/// instance to the respawner, and a revived mission NPC would re-fire its
/// `entity_dead_tag` chain.
///
/// The action-row half of this moved to the loader in the PR #662 review
/// (`loader::tests::spawn_entity_with_respawn_secs_still_converts_and_drops_it`):
/// `Action::SpawnEntity` no longer carries the field at all, so the
/// executor has nothing left to ignore and only the template path can leak.
#[tokio::test]
async fn spawn_never_inherits_the_templates_respawn_secs() {
    let mut mgr = make_space_mgr();
    let actor = 7003;
    stage_player(&mut mgr, actor, INSTANCED_WORLD);

    spawn_entity(
        TEMPLATE_ID,
        [0.0; 3],
        0.0,
        TAG.to_string(),
        None,
        None,
        None,
        actor,
        6301,
        &mut mgr,
    )
    .await;

    let npc_id = npc_with_tag(&mgr, actor).expect("the spawn itself must still land");
    assert_eq!(
        mgr.get_entity(npc_id).unwrap().respawn_secs,
        None,
        "a content-scoped spawn must be one-shot even when the template \
         asks for respawn",
    );
}

/// `aggression` and `is_stationary` are per-spawn values with no template
/// column; they must reach the entity from the action.
#[tokio::test]
async fn spawn_applies_aggression_and_stationary_overrides() {
    let mut mgr = make_space_mgr();
    let actor = 7004;
    stage_player(&mut mgr, actor, INSTANCED_WORLD);

    spawn_entity(
        TEMPLATE_ID,
        [0.0; 3],
        0.0,
        TAG.to_string(),
        Some(true),
        Some(1),
        None,
        actor,
        6301,
        &mut mgr,
    )
    .await;

    let npc = mgr
        .get_entity(npc_with_tag(&mgr, actor).expect("spawn must succeed"))
        .unwrap();
    assert_eq!(
        npc.aggro.override_level,
        Some(cimmeria_entity::cell_entity::MobAggression::Hostile),
        "the action's aggression is not on the template prototype, so a \
         regression that only clones the prototype would leave no override"
    );
    assert!(npc.is_stationary);
}

/// Spawning into a non-instanced world is refused: the NPC would be visible
/// to (and, hostile, attack) every player in the shared hub.
#[tokio::test]
async fn spawn_into_shared_world_is_refused() {
    let mut mgr = make_space_mgr();
    let actor = 7005;
    stage_player(&mut mgr, actor, SHARED_WORLD);
    let before = mgr.all_npc_entity_ids().len();

    spawn_default(&mut mgr, actor).await;

    assert!(
        npc_with_tag(&mgr, actor).is_none(),
        "a mission spawn must not populate a non-instanced world without \
         allow_shared"
    );
    assert_eq!(
        mgr.all_npc_entity_ids().len(),
        before,
        "the refusal must spawn nothing at all, not spawn-then-hide"
    );
}

/// `allow_shared: true` is the author's explicit opt-in and must work —
/// otherwise the refusal is a hard block rather than a guardrail.
#[tokio::test]
async fn spawn_into_shared_world_with_allow_shared_succeeds() {
    let mut mgr = make_space_mgr();
    let actor = 7006;
    let space_id = stage_player(&mut mgr, actor, SHARED_WORLD);

    spawn_entity(
        TEMPLATE_ID,
        [0.0; 3],
        0.0,
        TAG.to_string(),
        None,
        None,
        Some(true),
        actor,
        6301,
        &mut mgr,
    )
    .await;

    let npc_id = npc_with_tag(&mgr, actor).expect("allow_shared must lift the refusal");
    assert_eq!(mgr.get_entity_space_id(npc_id), Some(space_id));
}

/// Relog-restore chains re-fire their step's actions by design. A second
/// spawn with a tag already live in this space must be a no-op, not a
/// second NPC.
#[tokio::test]
async fn second_spawn_with_the_same_tag_is_a_no_op() {
    let mut mgr = make_space_mgr();
    let actor = 7007;
    stage_player(&mut mgr, actor, INSTANCED_WORLD);

    spawn_default(&mut mgr, actor).await;
    let first = npc_with_tag(&mgr, actor).expect("first spawn must succeed");
    let after_first = mgr.all_npc_entity_ids().len();

    spawn_default(&mut mgr, actor).await;

    assert_eq!(
        mgr.all_npc_entity_ids().len(),
        after_first,
        "a repeat spawn with a live tag must add no entity -- without this \
         guard every relog stacks another copy of the mission NPC"
    );
    assert_eq!(
        npc_with_tag(&mgr, actor),
        Some(first),
        "the original entity must survive the repeat, not be replaced"
    );
}

/// A dead entity still holds its tag. Re-firing the spawn must not
/// resurrect a mission NPC the player already killed.
#[tokio::test]
async fn spawn_does_not_resurrect_a_dead_tagged_entity() {
    use cimmeria_entity::cell_entity::AiState;
    let mut mgr = make_space_mgr();
    let actor = 7008;
    stage_player(&mut mgr, actor, INSTANCED_WORLD);

    spawn_default(&mut mgr, actor).await;
    let corpse = npc_with_tag(&mgr, actor).expect("first spawn must succeed");
    crate::cell::service::npc_ai::force_ai_state(
        mgr.get_entity_mut(corpse).unwrap(),
        AiState::Dead,
    );
    let after_first = mgr.all_npc_entity_ids().len();

    spawn_default(&mut mgr, actor).await;

    assert_eq!(
        mgr.all_npc_entity_ids().len(),
        after_first,
        "the idempotence lookup must match corpses too -- otherwise a relog \
         after the kill re-opens completed content"
    );
}

/// Cover-node and NPC-death chains fire with an NPC as the acting entity.
/// That NPC resolves to a *valid* space, so an unguarded spawn would
/// succeed in whichever instance it happens to be in — which is not "the
/// acting player's space".
#[tokio::test]
async fn spawn_from_a_non_player_actor_is_refused() {
    let mut mgr = make_space_mgr();
    let player = 7009;
    stage_player(&mut mgr, player, INSTANCED_WORLD);
    // An NPC in the same space, standing in as the chain's source entity.
    let space_id = mgr.get_entity_space_id(player).unwrap();
    let fixture_npc = mgr
        .spawn_npc_from_template(
            TEMPLATE_ID,
            space_id,
            INSTANCED_WORLD,
            [5.0; 3],
            0.0,
            "SomeOtherTag",
            false,
            None,
        )
        .expect("fixture NPC must spawn");
    let before = mgr.all_npc_entity_ids().len();

    spawn_default(&mut mgr, fixture_npc).await;

    assert_eq!(
        mgr.all_npc_entity_ids().len(),
        before,
        "an NPC-sourced chain must not spawn -- it resolves a valid space, \
         so the failure is silent-and-wrong rather than loud"
    );
}

/// A template id with no cached row spawns nothing.
#[tokio::test]
async fn spawn_with_an_unknown_template_is_refused() {
    let mut mgr = make_space_mgr();
    let actor = 7010;
    stage_player(&mut mgr, actor, INSTANCED_WORLD);
    let before = mgr.all_npc_entity_ids().len();

    spawn_entity(
        TEMPLATE_ID + 1,
        [0.0; 3],
        0.0,
        TAG.to_string(),
        None,
        None,
        None,
        actor,
        6301,
        &mut mgr,
    )
    .await;

    assert_eq!(mgr.all_npc_entity_ids().len(), before);
}

// ── despawn ──────────────────────────────────────────────────────────────

/// Stage a tagged NPC witnessed by `witness_count` connected players in the
/// shared startup space. Returns `(npc_id, witness_ids)`.
fn stage_npc_with_witnesses(mgr: &mut SpaceManager, witness_count: u32) -> (u32, Vec<u32>) {
    let npc_id = 9001;
    mgr.create_entity(npc_id, SHARED_WORLD, [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(npc_id).unwrap().tag = Some(TAG.to_string());

    let mut witnesses = Vec::new();
    for i in 0..witness_count {
        let pid = 8000 + i;
        mgr.create_entity(pid, SHARED_WORLD, [0.0; 3], [0.0; 3])
            .unwrap();
        let p = mgr.get_entity_mut(pid).unwrap();
        p.is_player = true;
        p.player_id = Some(100 + i as i32);
        p.witnesses.insert(EntityId(npc_id as i32));
        mgr.connect_entity(pid);
        witnesses.push(pid);
    }
    (npc_id, witnesses)
}

fn left_aoi_pairs(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::LeftAoI {
            witness_id,
            entity_id,
        } = msg
        {
            out.push((witness_id, entity_id));
        }
    }
    out.sort_unstable();
    out
}

/// The H-B6 / #582 shape: exactly one `LeftAoI` per current witness, and
/// the witness sets scrubbed. Reverting `despawn_by_tag` to the old bare
/// `SpaceManager::destroy_entity` produces zero `LeftAoI` and leaves the
/// dead id in all three witness sets.
#[tokio::test]
async fn despawn_fans_one_left_aoi_per_witness_and_scrubs_the_sets() {
    let mut mgr = make_space_mgr();
    let (npc_id, witnesses) = stage_npc_with_witnesses(&mut mgr, 3);
    let (tx, mut rx) = mpsc::channel(32);

    despawn_by_tag(
        TAG.to_string(),
        witnesses[0],
        6302,
        "despawn_entity",
        &tx,
        &mut mgr,
    )
    .await;

    let mut expected: Vec<(u32, u32)> = witnesses.iter().map(|w| (*w, npc_id)).collect();
    expected.sort_unstable();
    assert_eq!(
        left_aoi_pairs(&mut rx),
        expected,
        "every witness must get exactly one LeftAoI naming the despawned \
         entity -- zero here means the content path still calls bare \
         destroy_entity and the client keeps rendering a ghost"
    );
    for w in &witnesses {
        assert!(
            !mgr.get_entity(*w)
                .expect("witness must still exist")
                .witnesses
                .contains(&EntityId(npc_id as i32)),
            "witness {w}'s set must be scrubbed, or the next AoI tick emits \
             a duplicate LeftAoI"
        );
    }
    assert!(
        mgr.get_entity(npc_id).is_none(),
        "the entity itself must be gone"
    );
}

/// `destroy_entity` (the older seed spelling) must route identically —
/// closing U5 means both verbs share one implementation.
#[tokio::test]
async fn destroy_entity_routes_through_the_same_despawn() {
    let mut mgr = make_space_mgr();
    let (npc_id, witnesses) = stage_npc_with_witnesses(&mut mgr, 2);
    let (tx, mut rx) = mpsc::channel(32);

    // Through the dispatch arm, not a handler: the `destroy_entity` verb
    // has no handler of its own any more (the pass-through wrapper was
    // deleted in the PR #662 review), so the arm *is* the routing decision
    // this test guards.
    super::super::execute_one(
        6303,
        cimmeria_content_engine::actions::Action::DestroyTaggedEntity {
            entity_tag: TAG.to_string(),
        },
        witnesses[0],
        0,
        &std::collections::HashMap::new(),
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    // Exact pairs, not a count: a routing bug that fanned `LeftAoI` naming
    // the *source* entity instead of the target would still emit two
    // messages and satisfy a length assertion.
    let mut expected: Vec<(u32, u32)> = witnesses.iter().map(|w| (*w, npc_id)).collect();
    expected.sort_unstable();
    assert_eq!(
        left_aoi_pairs(&mut rx),
        expected,
        "destroy_entity must fan LeftAoI like despawn_entity does, naming the \
         target entity"
    );
    assert!(mgr.get_entity(npc_id).is_none());
}

/// An unresolvable tag emits nothing and destroys nothing.
#[tokio::test]
async fn despawn_with_an_unknown_tag_emits_nothing() {
    let mut mgr = make_space_mgr();
    let (npc_id, witnesses) = stage_npc_with_witnesses(&mut mgr, 1);
    let (tx, mut rx) = mpsc::channel(32);

    despawn_by_tag(
        "NoSuchTag".to_string(),
        witnesses[0],
        6304,
        "despawn_entity",
        &tx,
        &mut mgr,
    )
    .await;

    assert!(left_aoi_pairs(&mut rx).is_empty());
    assert!(
        mgr.get_entity(npc_id).is_some(),
        "the real tagged NPC must be untouched"
    );
}
