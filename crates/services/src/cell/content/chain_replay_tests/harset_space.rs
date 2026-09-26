//! Harset space chains — ring switches and the Command Center doors
//! (`harset_space_chains.sql` chains 6001-6007, packet H10).
//!
//! These are not mission chains; they are the permanent furniture of
//! worlds 57 (`Harset`) and 68 (`Harset_CmdCenter`), ported line-for-line
//! from the two surviving 2009 space scripts. The ring chains carry no
//! conditions and the two door chains carry only a `world` gate (H07),
//! so the whole regression surface is *which key resolves which action*:
//! a transposed `regionId`, a miscopied spawn tag, or a `params` key
//! renamed out of camelCase are the realistic authoring errors, and each
//! one is silent at load time.
//!
//! Five guard shapes live here:
//!
//! 1. **Ring resolution.** Each of the five switch tags resolves exactly
//!    its own `TriggerTransporter` and nothing else.
//! 2. **Ring identity, derived from data this packet did not author.**
//!    The tag↔region pairing is re-derived through
//!    `ring_transport_regions` → `point_sets`, because a table of
//!    expected pairs written by the same hand that wrote the seed cannot
//!    catch a transposition made at authoring time.
//! 3. **Door isolation (the T3 trap).** Each door's region key resolves
//!    exactly one `CrossWorldTeleport` to its own destination world, and
//!    zero to the other. `OnRegionEnter` does not filter by world, so
//!    each door carries a `world eq <id>` condition (H07) and the test
//!    proves the same key resolves nothing from the other world or from
//!    a context with no world at all (the fail-closed case).
//! 4. **The off-mesh return coordinate.** Chain 6007 is enabled with an
//!    arrival the mesh does not cover (placement PL-A-06 measured a real
//!    floor 0.04 m under it with `obj_slab`), so the guard pins the two
//!    conditions that make that safe instead: world 57 stays `advisory`
//!    and keeps an authored respawner row.
//! 5. **Ping-pong and template-bit invariants** that the seed comments
//!    assert in prose: neither arrival lands in the opposing door's
//!    trigger box, and template 3 still carries `INT_RingNetwork` (the
//!    bit the linter allowlist entries assume).
//!
//! Every resolve assertion counts `resolved.actions.len()` in total, not
//! just the filtered matches. `build_engine` loads the entire DB into one
//! engine and `resolve_event` appends every matching chain's actions
//! into one flat vec with no per-file partitioning, so a
//! variant-filtered count of 1 still passes when some other seed file's
//! chain matched the same key and appended unrelated actions. Chain ids
//! are pinned alongside the action, following the `mission_640` shape
//! rather than the variant-only filter in `region_transition_accepts`.
//!
//! Deliberately **not** here: an executor-level assertion (TESTING.md's
//! caveat that resolve-only replays don't pin executor arms). Both arms
//! these chains use — `trigger_transporter` and `cross_world_teleport` —
//! are pre-existing and already exercised by the Castle Cellblock chains
//! (1043, 1109); H10 adds no arm, so there is nothing new to pin.

use cimmeria_common::Vector3;
use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use cimmeria_entity::navigation::NavMesh;

use super::super::engine_loader::build_engine;
use crate::test_support::require_db_or_skip;

/// `(chain_id, spawn tag, ring_transport_regions.region_id)` for the five
/// Harset ring switches.
///
/// Verified three ways when authored: `Harset.py:19,26,33,40,47`
/// (`transporters.get(N)`), `spawnlist.sql` rows 4/127/128/129/130 (the
/// tags), and `ring_transport_regions.sql:29-53` (the regions). Region 8's
/// `tag` column carries the shipped 2009 typo `HarsetinRingRightRegion`,
/// which is deliberately not matched on anywhere — the chain keys on the
/// *spawn* tag below.
const RINGS: [(i64, &str, i32); 5] = [
    (6001, "HarsetRingLeftBottom", 4),
    (6002, "HarsetRingRightBottom", 5),
    (6003, "HarsetRingLeft", 6),
    (6004, "HarsetRingLeftTop", 7),
    (6005, "HarsetRingRight", 8),
];

/// Fire an `interact_tag` event for `tag` through the full seeded engine.
fn resolve_interact_tag(
    engine: &ChainEngine,
    tag: &str,
) -> cimmeria_content_engine::chain::ResolvedActions {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("entity_tag".to_string(), serde_json::json!(tag));
    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, &ctx)
}

/// Fire an `enter_region` event for `region_key` through the full seeded
/// engine, as a player standing in `world_id` (`None` models a dispatch
/// site that never populated the world context — the H07 fail-closed
/// case).
fn resolve_enter_region(
    engine: &ChainEngine,
    region_key: &str,
    world_id: Option<i32>,
) -> cimmeria_content_engine::chain::ResolvedActions {
    let mut ctx = ExecutionContext::new();
    ctx.world_id = world_id;
    ctx.set_param("region_key".to_string(), serde_json::json!(region_key));
    let event = TriggerEvent {
        trigger_type: TriggerType::RegionEnter,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, &ctx)
}

/// Each ring switch resolves exactly one action: its own chain's
/// `TriggerTransporter` with its own region id.
///
/// Fails if any of the five seed chains is missing, if a `regionId`
/// param was transposed between two switches, or if the param key drifted
/// out of camelCase — the loader reads `regionId` and falls back to
/// `unwrap_or(0)` rather than dropping the action, so a snake_case typo
/// produces a live `TriggerTransporter { region_id: 0 }` that reaches the
/// ring runtime, finds no transporter, and logs. Asserting the exact
/// region id is what turns that silent dead switch into a test failure.
#[tokio::test]
async fn each_harset_ring_tag_resolves_exactly_its_own_transporter() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    for (chain_id, tag, region_id) in RINGS {
        let resolved = resolve_interact_tag(&engine, tag);

        assert_eq!(
            resolved.actions.len(),
            1,
            "interact_tag '{tag}' must resolve exactly one action (chain \
             {chain_id}); got {} — either the seed row is missing or another \
             chain elsewhere in the DB also matched this tag. Actions: {:?}",
            resolved.actions.len(),
            resolved.actions,
        );
        assert!(
            matches!(
                &resolved.actions[0],
                (id, Action::TriggerTransporter { region_id: r })
                    if *id == chain_id && *r == region_id
            ),
            "interact_tag '{tag}' must resolve chain {chain_id}'s \
             TriggerTransporter for region {region_id}. A different region id \
             here means two switches' `regionId` params were transposed, \
             which sends the player to the wrong pad with no runtime error. \
             Got: {:?}",
            resolved.actions,
        );
    }
}

/// Every ring chain's tag agrees with its region id *according to the
/// seed*, independently of the [`RINGS`] table above.
///
/// [`RINGS`] is a second copy of data I authored, so a test driven only
/// by it cannot catch a transposition made at authoring time — the
/// mistake would be copied into both places. This one derives the
/// expected pairing from rows nobody in this packet wrote:
/// `content_actions.params->>'regionId'` picks a
/// `ring_transport_regions` row, that row's `point_set_id` picks a
/// `point_sets` row, and that point set's name must be the chain's own
/// `interact_tag` key plus the `PS` suffix the shipped data uses
/// (`HarsetRingLeftBottom` ↔ set 2052 `HarsetRingLeftBottomPS`).
///
/// A transposed `regionId` breaks the correspondence immediately, with
/// no dependence on anything H10 authored.
#[tokio::test]
async fn harset_ring_chain_tags_agree_with_their_regions_point_set() {
    let pool = require_db_or_skip!();

    let rows: Vec<(i32, String, i32, String)> = sqlx::query_as(
        "SELECT c.chain_id, t.event_key, (a.params->>'regionId')::int, ps.name \
         FROM resources.content_chains c \
         JOIN resources.content_triggers t ON t.chain_id = c.chain_id \
         JOIN resources.content_actions a ON a.chain_id = c.chain_id \
         JOIN resources.ring_transport_regions r \
           ON r.region_id = (a.params->>'regionId')::int \
         JOIN resources.point_sets ps ON ps.set_id = r.point_set_id \
         WHERE c.chain_id BETWEEN 6001 AND 6005 \
           AND t.event_type = 'interact_tag' \
           AND a.action_type = 'trigger_transporter' \
         ORDER BY c.chain_id",
    )
    .fetch_all(&pool)
    .await
    .expect("query ring chain / region / point-set correspondence");

    assert_eq!(
        rows.len(),
        5,
        "expected five ring chains (6001-6005) each joining cleanly to a \
         ring_transport_regions row and its point set; got {}. A short \
         count means a chain is missing, its `regionId` param names a \
         region that does not exist, or the param key drifted out of \
         camelCase (the loader's `unwrap_or(0)` hides that at runtime, so \
         this join is where it surfaces). Rows: {rows:?}",
        rows.len(),
    );

    for (chain_id, event_key, region_id, point_set_name) in &rows {
        assert_eq!(
            point_set_name,
            &format!("{event_key}PS"),
            "chain {chain_id} right-clicks '{event_key}' but its \
             regionId {region_id} belongs to point set '{point_set_name}'. \
             The shipped data names each ring's pad '<tag>PS', so a \
             mismatch means two chains' region ids are transposed and the \
             player is sent to the wrong pad with no runtime error.",
        );
    }
}

/// The `interact_tag` bit on the ring switches comes from entity template
/// 3, not from any chain — and that is the justification for this
/// packet's five `interact_tag_linter` allowlist entries.
///
/// Without this test the claim is unpinned in a dangerous direction: zero
/// out `entity_templates.interaction_type` for template 3 and all five
/// switches go un-clickable, the linter stays **silent because of the
/// allowlist**, and every other test in this file still passes — the
/// chains resolve fine, the player just can never fire them. An allowlist
/// entry should earn a pin.
#[tokio::test]
async fn ring_switch_template_carries_the_ring_network_bit() {
    let pool = require_db_or_skip!();

    // INT_RingNetwork, bit 5. See docs/content/interaction-flags.md.
    const INT_RING_NETWORK: i64 = 32;

    let (interaction_type,): (Option<i64>,) = sqlx::query_as(
        "SELECT interaction_type FROM resources.entity_templates WHERE template_id = 3",
    )
    .fetch_one(&pool)
    .await
    .expect("entity template 3 (Ring Transporter Switch) must exist");

    let flags = interaction_type.unwrap_or(0);
    assert_eq!(
        flags & INT_RING_NETWORK,
        INT_RING_NETWORK,
        "entity template 3 must keep INT_RingNetwork (32) in \
         `interaction_type` — it is the only thing that makes the five \
         Harset ring switches right-clickable, and the five allowlist \
         entries in crates/content-engine/tests/it/interact_tag_linter.rs \
         suppress the linter's complaint on exactly that basis. Got \
         {flags}. If this bit is deliberately moving into chain SQL, seed \
         `set_interaction_type` actions and delete those allowlist entries.",
    );
}

/// Chain 6006: entering `Harset.CommandCenterTransition` resolves exactly
/// one `CrossWorldTeleport` to `Harset_CmdCenter` at the coordinate
/// recovered from `Harset.py:15`, and zero teleports to `Harset`.
///
/// The exact position is pinned, not just the variant, for two reasons.
/// `convert_action` *drops* a `cross_world_teleport` whose params are
/// missing or non-finite rather than defaulting to the origin, so a
/// malformed row yields zero actions — which a "resolves a teleport"
/// assertion catches but a "resolves at most one" assertion does not.
/// And pinning the coordinate means a future M0 re-pin has to come
/// through this test rather than silently landing the player somewhere
/// else.
#[tokio::test]
async fn harset_command_center_door_teleports_only_to_the_command_center() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    // Chain 6006 carries `world eq 57` (H07). A player in Harset trips it;
    // the same region key hinted from the Command Center (68) or from a
    // dispatch site that never populated the world must resolve nothing —
    // that is the real closure of the T3 trap, not the byte-distinct keys.
    for (world, label) in [(Some(68), "world 68"), (None, "no world context")] {
        let wrong = resolve_enter_region(&engine, "Harset.CommandCenterTransition", world);
        assert!(
            wrong.actions.is_empty(),
            "chain 6006 must not resolve for a player with {label}; the `world eq 57` \
             condition row is missing or evaluated open. Actions: {:?}",
            wrong.actions,
        );
    }

    let resolved = resolve_enter_region(&engine, "Harset.CommandCenterTransition", Some(57));

    assert_eq!(
        resolved.actions.len(),
        1,
        "enter_region 'Harset.CommandCenterTransition' must resolve exactly \
         one action (chain 6006) for a player in world 57; got {}. Actions: {:?}",
        resolved.actions.len(),
        resolved.actions,
    );
    assert!(
        matches!(
            &resolved.actions[0],
            (6006, Action::CrossWorldTeleport { world_name, position })
                if world_name == "Harset_CmdCenter"
                    && *position == [0.0f32, 0.355, -20.0]
        ),
        "chain 6006 must teleport to Harset_CmdCenter at the coordinate \
         recovered from Harset.py:15 (`str2vec('0,0.355,-20')`). Got: {:?}",
        resolved.actions,
    );

    let back_to_harset = resolved
        .actions
        .iter()
        .filter(|(_, action)| {
            matches!(action, Action::CrossWorldTeleport { world_name, .. } if world_name == "Harset")
        })
        .count();
    assert_eq!(
        back_to_harset, 0,
        "the outbound door must not also resolve the return chain — \
         OnRegionEnter ignores world, so byte-distinct region keys are the \
         only separation between 6006 and 6007. Actions: {:?}",
        resolved.actions,
    );
}

/// Chain 6007 is seeded ENABLED (placement PL-A-06), carries the recovered
/// 2009 coordinate unchanged, and entering
/// `Harset_CmdCenter.HarsetTransition` resolves exactly its own teleport
/// back to Harset.
///
/// This is the return half of the T3 trap *and* the enabled-flag guard, and
/// the structural pins below are the reason it is worth keeping now that the
/// flag is `true` rather than `false`: H10's version of this test noted that
/// "disabled" could hide "structurally broken" — a chain whose trigger or
/// action rows were missing or typo'd resolved nothing for the wrong reason
/// and every test stayed green. Enabling the chain converts that latent hole
/// into a live one, so the `event_key`, destination world and coordinate are
/// all pinned here. The survivability half of the story (off-mesh arrival,
/// advisory world, respawner row) is in
/// [`harset_return_arrival_is_offmesh_but_survivable`].
#[tokio::test]
async fn harset_return_door_is_enabled_and_structurally_intact() {
    let pool = require_db_or_skip!();

    let (enabled,): (bool,) =
        sqlx::query_as("SELECT enabled FROM resources.content_chains WHERE chain_id = 6007")
            .fetch_one(&pool)
            .await
            .expect(
                "chain 6007 must exist in content_chains — harset_space_chains.sql is \
                 either unseeded or not `\\ir`'d from db/database.sql",
            );
    assert!(
        enabled,
        "chain 6007 must be `enabled = true`. Placement PL-A-06 opened it \
         after `obj_slab` confirmed a real floor 0.04 m under the recovered \
         arrival (0, -67.6, -231), 7.41 m clear of point set 2078. If it has \
         been closed again, chains 6511-6513 and 6528 must close with it — \
         see `praxis_acceptance_is_enabled_iff_the_return_door_is`",
    );

    // Pin the rows the door depends on. A chain row that exists and is
    // enabled, whose trigger or action rows are missing or typo'd, is a door
    // that silently does nothing — and the resolve assertion at the end
    // cannot distinguish "wrong key" from "no chain".
    let (event_key,): (String,) = sqlx::query_as(
        "SELECT event_key FROM resources.content_triggers \
         WHERE chain_id = 6007 AND event_type = 'enter_region'",
    )
    .fetch_one(&pool)
    .await
    .expect("chain 6007 must have exactly one enter_region trigger row");
    assert_eq!(
        event_key, "Harset_CmdCenter.HarsetTransition",
        "chain 6007's region key must byte-match point_sets row 2079; the \
         resolver compares with case-sensitive string equality, so a typo \
         here is a door that never fires",
    );

    let (world, x, y, z): (String, f64, f64, f64) = sqlx::query_as(
        "SELECT target_key, (params->>'x')::float8, (params->>'y')::float8, \
                (params->>'z')::float8 \
         FROM resources.content_actions \
         WHERE chain_id = 6007 AND action_type = 'cross_world_teleport'",
    )
    .fetch_one(&pool)
    .await
    .expect("chain 6007 must have exactly one cross_world_teleport action row");
    assert_eq!(
        world, "Harset",
        "chain 6007 must return the player to Harset; the world name is \
         matched byte-exactly against entities/spaces.xml",
    );
    assert_eq!(
        (x, y, z),
        (0.0, -67.6, -231.0),
        "chain 6007 must still carry the coordinate recovered from \
         Harset_CmdCenter.py:15. Placement PL-A-06 deliberately did NOT move \
         it — the measured floor and the 7.41 m clearance from point set 2078 \
         are both properties of this exact point. If it is re-pinned, redo \
         the obj_slab floor check and the clearance check and update the \
         ledger row PL-A-06, `harset_return_arrival_is_offmesh_but_survivable` \
         and this expectation together.",
    );

    let engine = build_engine(Some(&pool)).await;
    // World 68 is the one world the chain's `world eq 68` row admits.
    let resolved = resolve_enter_region(&engine, "Harset_CmdCenter.HarsetTransition", Some(68));
    // Total count, not a filtered count: `build_engine` loads the whole
    // database into one engine and `resolve_event` appends every matching
    // chain's actions into one flat vec, so a filtered count of 1 still
    // passes when another seed file's chain matched the same key.
    assert_eq!(
        resolved.actions.len(),
        1,
        "enter_region 'Harset_CmdCenter.HarsetTransition' must resolve \
         exactly one action (chain 6007) for a player in world 68; got {}. \
         Actions: {:?}",
        resolved.actions.len(),
        resolved.actions,
    );
    assert!(
        matches!(
            &resolved.actions[0],
            (6007, Action::CrossWorldTeleport { world_name, position, .. })
                if world_name == "Harset" && *position == [0.0f32, -67.6, -231.0]
        ),
        "chain 6007 must teleport back to Harset at the coordinate recovered \
         from Harset_CmdCenter.py:15. Got: {:?}",
        resolved.actions,
    );

    // The mirror of 6006's world gate: the return key must resolve nothing
    // from world 57 or from a dispatch site that never populated the world.
    for (world, label) in [(Some(57), "world 57"), (None, "no world context")] {
        let wrong = resolve_enter_region(&engine, "Harset_CmdCenter.HarsetTransition", world);
        assert!(
            wrong.actions.is_empty(),
            "chain 6007 must not resolve for a player with {label}; the \
             `world eq 68` condition row is missing or evaluated open. \
             Actions: {:?}",
            wrong.actions,
        );
    }
}

/// Chain 6007's arrival is off the `harset.nav` mesh, the chain is enabled
/// anyway, and the two conditions that make that safe must both hold.
///
/// **What changed, and why the old biconditional had to go.** H10 shipped
/// 6007 disabled and this guard asserted `enabled ⟹ on-mesh`, on the
/// reasoning that off-mesh is the repo's only automated proxy for
/// "unverified" — the coordinate came out of a prop transform and might be
/// inside geometry or above a drop. Placement PL-A-06 answered that question
/// with the cooked map instead of a playtest: `obj_slab` reports an up-facing
/// floor at y -67.64 in the columns at (0, -231), (±1, -231) and (0, -230),
/// with 6.5 m of headroom, and the seeded y of -67.600 sits 0.04 m above it.
/// The point is on a floor. Keeping the old implication would have required
/// waiting for a *navmesh rebuild*, not a pin: nothing within ~20 m of that
/// door is on-mesh at the real floor height (the nearest polygon to the
/// arrival is 28.6 m above it, and 51.7 m above the door threshold), so
/// there was no on-mesh coordinate to move to.
///
/// **The replacement contract**, which is still a biconditional and still
/// fails on a half-done change: 6007 may be enabled with an off-mesh arrival
/// **only while** world 57 is `navmesh_mode = 'advisory'` *and* world 57 has
/// an authored respawner row. Those are the two independent reasons the
/// off-mesh arrival cannot ghost the player — advisory makes the movement
/// validator fail open, and the respawner makes even an enforcing world's
/// recovery land somewhere real instead of returning
/// `UnrecoverableOffMesh`. Flip world 57 back to `enforce`, or delete
/// respawner 20, and this test fails before a player finds out.
///
/// An on-mesh coordinate still satisfies it unconditionally, so a future
/// navmesh rebuild needs no edit here — only the `!on_mesh` branch is
/// conditional.
///
/// The ring-4 pad is checked as a live control: if the mesh failed to load,
/// or every query returned false, the off-mesh verdict below would be
/// vacuous.
///
/// Everything is read from the DB rather than hardcoded, because a hardcoded
/// literal silently stops describing the seed the moment the seed changes.
/// No navmesh is asserted for chain 6006's destination: world 68 had no
/// `.nav` file at all, which is precisely why that door always shipped
/// enabled. NA26 gave it one (`harset_cmdcenter.nav`) and seeded it
/// advisory, so that door still cannot be refused on mesh grounds.
///
/// Since NA26 the rebuilt `harset.nav` covers the Command Center door, so
/// today this test takes the `on_mesh` early return.
#[tokio::test]
async fn harset_return_arrival_is_offmesh_but_survivable() {
    let pool = require_db_or_skip!();

    let (enabled, x, y, z): (bool, f64, f64, f64) = sqlx::query_as(
        "SELECT c.enabled, (a.params->>'x')::float8, (a.params->>'y')::float8, \
                (a.params->>'z')::float8 \
         FROM resources.content_chains c \
         JOIN resources.content_actions a ON a.chain_id = c.chain_id \
         WHERE c.chain_id = 6007 AND a.action_type = 'cross_world_teleport'",
    )
    .fetch_one(&pool)
    .await
    .expect("chain 6007 and its cross_world_teleport action must exist");

    let path = std::path::Path::new("../../data/spaces/harset.nav");
    let mesh = NavMesh::load(path).expect("load data/spaces/harset.nav");

    // Control: the ring-4 pad, a coordinate the shipped seed already
    // places a player-reachable entity on. `ring_transport_regions.sql:29`
    // spells it -25.6410007 / -67.8280029 / 15.2489996 — Postgres's text
    // form for a `real`, which carries more digits than f32 can hold. The
    // shortened literals below are the same f32 bit patterns (clippy's
    // `excessive_precision` says so); do not read the difference as a
    // rounded-off coordinate. Without this control, a mesh that failed to
    // load would make the off-mesh branch below vacuously true.
    assert!(
        mesh.is_point_valid(&Vector3::new(-25.641, -67.828, 15.249)),
        "ring region 4's pad must be on-mesh — if this fails the mesh did \
         not load correctly and the verdict below is meaningless",
    );

    let arrival = Vector3::new(x as f32, y as f32, z as f32);
    let on_mesh = mesh.is_point_valid(&arrival);

    assert!(
        enabled,
        "chain 6007 is disabled. Placement PL-A-06 enabled it after \
         confirming a real floor 0.04 m under the recovered arrival \
         {arrival:?}; if it has been turned off again, mission 1361's \
         acceptance trio (6511-6513) and its abandon twin 6528 must be \
         turned off in the same change or every player who accepts 1361 \
         soft-sticks at step 4041 with no `fail_objective` arm to recover.",
    );

    if on_mesh {
        // A navmesh rebuild reached this corner of the map. Nothing to
        // check — the arrival needs no special dispensation any more. Worth
        // re-reading the seed comment at 6007 and simplifying it.
        return;
    }

    // Off-mesh: both survivability conditions must hold.
    let (navmesh_mode,): (String,) =
        sqlx::query_as("SELECT navmesh_mode FROM resources.worlds WHERE world_id = 57")
            .fetch_one(&pool)
            .await
            .expect("world 57 (Harset) must exist in resources.worlds");
    assert_eq!(
        navmesh_mode, "advisory",
        "chain 6007's arrival {arrival:?} is off harset.nav and world 57 is \
         now `{navmesh_mode}`. Under `enforce` the movement validator stops \
         failing open, so every position update a returning player sends is \
         suppressed — the silent `CorrectionSuppressed` freeze H10 disabled \
         this door to avoid. Either rebuild harset.nav around the Command \
         Center door (its nearest polygon there is ~29 m above the real \
         floor), or disable 6007 together with chains 6511-6513 and 6528.",
    );

    let authored_world_57_respawners: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM resources.respawners \
         WHERE world_id = 57 AND NOT (pos_x = 0 AND pos_y = 0 AND pos_z = 0)",
    )
    .fetch_one(&pool)
    .await
    .expect("count world-57 respawners");
    assert!(
        authored_world_57_respawners > 0,
        "chain 6007's arrival {arrival:?} is off harset.nav and world 57 has \
         no authored respawner row. That is the second half of why the \
         off-mesh arrival is survivable: `nearest_valid_respawner` skips \
         `(0,0,0)` placeholders, so with none left an off-mesh arrival in an \
         enforcing world resolves to `UnrecoverableOffMesh` and the caller \
         refuses the transfer. Restore respawners row 20 (placement PL-A-02) \
         rather than deleting it.",
    );
}

/// Neither door may drop the player inside the *opposing* door's trigger
/// box, and world 68's respawner may not sit inside world 68's door.
///
/// This is the ping-pong invariant. The seed comments assert the
/// clearance exists ("several units", per the audit) but nothing
/// exercised it, and its failure mode is the worst in the packet: a
/// player who arrives inside the return trigger is bounced straight back,
/// arrives inside the outbound trigger, and loops — a soft-lock that no
/// amount of client-side input escapes. Respawning into a door does the
/// same thing on every death.
///
/// Deliberately data-driven on both sides (arrival coordinates from
/// `content_actions`/`respawners`, boxes from `point_set_points`) so an
/// M0 re-pin is checked rather than assumed.
#[tokio::test]
async fn door_arrivals_and_respawner_sit_outside_the_opposing_trigger_box() {
    let pool = require_db_or_skip!();

    // AABB of a BoundingBox point set, from its corner points.
    async fn aabb(pool: &sqlx::PgPool, set_id: i32) -> (f64, f64, f64, f64, f64, f64) {
        sqlx::query_as(
            "SELECT MIN(x)::float8, MAX(x)::float8, MIN(y)::float8, MAX(y)::float8, \
                    MIN(z)::float8, MAX(z)::float8 \
             FROM resources.point_set_points WHERE set_id = $1",
        )
        .bind(set_id)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|e| panic!("point_set_points for set {set_id}: {e}"))
    }

    fn inside(p: (f64, f64, f64), b: (f64, f64, f64, f64, f64, f64)) -> bool {
        p.0 >= b.0 && p.0 <= b.1 && p.1 >= b.2 && p.1 <= b.3 && p.2 >= b.4 && p.2 <= b.5
    }

    // 2078 = Harset.CommandCenterTransition (world 57, the outbound door).
    // 2079 = Harset_CmdCenter.HarsetTransition (world 68, the return door).
    let box_2078 = aabb(&pool, 2078).await;
    let box_2079 = aabb(&pool, 2079).await;

    for (chain_id, opposing_set, opposing_box) in [(6006, 2079, box_2079), (6007, 2078, box_2078)] {
        let (x, y, z): (f64, f64, f64) = sqlx::query_as(
            "SELECT (params->>'x')::float8, (params->>'y')::float8, (params->>'z')::float8 \
             FROM resources.content_actions \
             WHERE chain_id = $1 AND action_type = 'cross_world_teleport'",
        )
        .bind(chain_id)
        .fetch_one(&pool)
        .await
        .unwrap_or_else(|e| panic!("cross_world_teleport params for chain {chain_id}: {e}"));

        assert!(
            !inside((x, y, z), opposing_box),
            "chain {chain_id} drops the player at ({x}, {y}, {z}), which is \
             INSIDE point set {opposing_set}'s trigger box {opposing_box:?}. \
             The player is teleported straight back and loops forever. Move \
             the arrival clear of the box.",
        );
    }

    let (rx, ry, rz): (f64, f64, f64) = sqlx::query_as(
        "SELECT pos_x::float8, pos_y::float8, pos_z::float8 \
         FROM resources.respawners WHERE respawner_id = 21",
    )
    .fetch_one(&pool)
    .await
    .expect("respawner row 21 (world 68 Command Center) must exist");

    assert!(
        !inside((rx, ry, rz), box_2079),
        "respawner 21 is at ({rx}, {ry}, {rz}), INSIDE point set 2079's \
         trigger box {box_2079:?} — every death in the Command Center would \
         respawn the player standing in the return door and immediately \
         teleport them to Harset.",
    );
}
