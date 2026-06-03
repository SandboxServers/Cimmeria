use super::*;
use cimmeria_entity::inventory::{
    INV_ARTIFACT1, INV_ARTIFACT2, INV_BACK, INV_BANDOLIER, INV_BUYBACK, INV_CHEST, INV_CRAFTING,
    INV_FACE, INV_FEET, INV_HANDS, INV_HEAD, INV_LEGS, INV_MAIN, INV_MISSION, INV_NECK, INV_WAIST,
};

/// Sentinel-slot invariant: the move handler's three-step swap parks the
/// source row at `slot_id = -1` mid-transaction. For that to be safe,
/// `bag_min_slot` MUST never return a value `<= -1` for any container —
/// otherwise a concurrent grant could legitimately reserve `slot_id = -1`
/// and collide with the parked sentinel.
///
/// This test pins that invariant across `container_id` 0..=16. The
/// game-defined range is 1..=16 (main, mission, bandolier, equipment
/// slots 4..=14, crafting, vendor buyback); 0 is included as a sentinel
/// for "no container" so the symmetry with `bag_max_slots` (which
/// returns 0 for 0) is also exercised. Any out-of-range container_id
/// returning 0 from `bag_min_slot` is fine — `bag_max_slots` returns 0
/// there too, so no slot is ever reservable.
///
/// Documented as the regression guard in `move_/mod.rs`'s swap-path
/// comment (`bag_max_slots() never reserves negative slots, so
/// grant/purchase paths cannot land there`).
#[test]
fn bag_min_slot_is_non_negative_for_every_container() {
    for container_id in 0..=16 {
        let min = bag_min_slot(container_id);
        assert!(
            min >= 0,
            "bag_min_slot({container_id}) returned {min}; must be >= 0"
        );
    }
}

#[test]
fn bag_max_slots_known_containers_match_constants() {
    assert_eq!(bag_max_slots(INV_MAIN), 40);
    assert_eq!(bag_max_slots(INV_MISSION), 100);
    assert_eq!(bag_max_slots(INV_BANDOLIER), 4);
    for container_id in [
        INV_HEAD,
        INV_FACE,
        INV_NECK,
        INV_CHEST,
        INV_HANDS,
        INV_WAIST,
        INV_BACK,
        INV_LEGS,
        INV_FEET,
        INV_ARTIFACT1,
        INV_ARTIFACT2,
    ] {
        assert_eq!(bag_max_slots(container_id), 1);
    }
    assert_eq!(bag_max_slots(INV_CRAFTING), 100);
    assert_eq!(bag_max_slots(INV_BUYBACK), 12);
}

#[test]
fn bag_max_slots_out_of_range_returns_zero() {
    assert_eq!(bag_max_slots(0), 0);
    assert_eq!(bag_max_slots(17), 0);
    assert_eq!(bag_max_slots(100), 0);
    assert_eq!(bag_max_slots(-1), 0);
}

#[test]
fn bag_min_slot_bandolier_is_zero() {
    assert_eq!(
        bag_min_slot(INV_BANDOLIER),
        0,
        "bandolier slots are zero-based weapon slots"
    );
}

#[test]
fn bag_min_slot_other_containers_is_zero() {
    for container_id in [1, 2, 4, 15, 16] {
        assert_eq!(
            bag_min_slot(container_id),
            0,
            "container {container_id} must start at slot 0"
        );
    }
}

// ── compute_metadata_bump invariants ─────────────────────────────
// The bump is what makes the cooked-data version-info handshake see a
// mismatch and reship overridden entries. Same content → same bump
// (no churn). Any field change (mission_id, insert_after_step_id, or
// injected XML) → different bump (client refetches).

use super::super::mission_overrides::MissionOverride;

fn ov(mission_id: u32, after: u32, xml: &'static str) -> MissionOverride {
    MissionOverride {
        mission_id,
        insert_after_step_id: after,
        injected_steps_xml: xml,
    }
}

#[test]
fn compute_metadata_bump_is_deterministic_across_calls() {
    let overrides = [
        ov(622, 2113, "<Steps>x</Steps>"),
        ov(641, 2121, "<Steps>y</Steps>"),
    ];
    assert_eq!(
        compute_metadata_bump(&overrides, &[]),
        compute_metadata_bump(&overrides, &[]),
        "same overrides must hash to the same bump on every call",
    );
}

#[test]
fn compute_metadata_bump_low_bit_is_always_set() {
    // Even an empty-overrides bump must be non-zero so a future caller
    // that hits this path doesn't leave metadata unchanged.
    let bump_empty = compute_metadata_bump(&[], &[]);
    assert_eq!(bump_empty & 0x1, 0x1, "low bit must be set");

    let bump_one = compute_metadata_bump(&[ov(1, 2, "x")], &[]);
    assert_eq!(bump_one & 0x1, 0x1, "low bit must be set");
}

#[test]
fn compute_metadata_bump_changes_when_xml_changes() {
    let a = [ov(622, 2113, "<Steps>aaa</Steps>")];
    let b = [ov(622, 2113, "<Steps>bbb</Steps>")];
    assert_ne!(
        compute_metadata_bump(&a, &[]),
        compute_metadata_bump(&b, &[]),
        "edits to injected XML must produce a different bump so the \
         client refetches the patched entry",
    );
}

#[test]
fn compute_metadata_bump_changes_when_insert_after_changes() {
    // The load-bearing case: a maintainer pivots only the insertion
    // anchor (e.g., 2113 → some other anchor in the same mission)
    // while keeping mission_id and XML identical. Without
    // insert_after_step_id in the hash this would silently re-use
    // the previous bump and the client would never refetch even
    // though the patched XML's structure changed.
    let a = [ov(622, 2113, "<Steps>x</Steps>")];
    let b = [ov(622, 9999, "<Steps>x</Steps>")];
    assert_ne!(
        compute_metadata_bump(&a, &[]),
        compute_metadata_bump(&b, &[]),
        "insert_after_step_id must participate in the hash",
    );
}

#[test]
fn compute_metadata_bump_changes_when_mission_id_changes() {
    let a = [ov(622, 2113, "<Steps>x</Steps>")];
    let b = [ov(641, 2113, "<Steps>x</Steps>")];
    assert_ne!(
        compute_metadata_bump(&a, &[]),
        compute_metadata_bump(&b, &[])
    );
}

/// A change to a step-text override's `new_step_display_log_text` must
/// produce a different bump too — otherwise a client cached against
/// the previous text would never see the corrected version.
#[test]
fn compute_metadata_bump_changes_when_step_text_override_changes() {
    use super::super::mission_overrides::StepTextOverride;
    let a = [StepTextOverride {
        mission_id: 639,
        step_id: 2343,
        new_step_display_log_text: "press 'a'",
    }];
    let b = [StepTextOverride {
        mission_id: 639,
        step_id: 2343,
        new_step_display_log_text: "press 'b'",
    }];
    assert_ne!(
        compute_metadata_bump(&[], &a),
        compute_metadata_bump(&[], &b),
        "edits to step-text override caption must change the bump so \
         the client refetches the patched mission entry",
    );
}

// ── apply_mission_overrides on hand-built CategoryData ───────────
// The PAK-load path is integration-tested by running the real server,
// but the in-memory mutation logic is covered here without going
// through ZIP IO. Build the same `HashMap<u32, CategoryData>` shape
// that `load_pak` would have produced, call the private helper, and
// assert the post-state.

/// Minimal but realistic mission XML: the QA-build root + one `<Steps>`
/// child per known step id. `apply_override` looks for `StepID="<id>"`
/// and the next `</Steps>`, both present here.
fn fake_mission_xml(mission_id: u32, step_ids: &[u32]) -> Vec<u8> {
    let mut s = String::new();
    s.push_str(&format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
         <COOKED_MISSION MissionID=\"{mission_id}\">"
    ));
    for sid in step_ids {
        s.push_str(&format!(
            "<Steps StepEnabled=\"false\" StepID=\"{sid}\" \
             AwardXP=\"false\" Difficulty=\"1\">\
             <StepDisplayLogText>step {sid}</StepDisplayLogText>\
             </Steps>"
        ));
    }
    s.push_str("</COOKED_MISSION>");
    s.into_bytes()
}

fn missions_category_with(entries: &[(u32, Vec<u32>)], metadata: u32) -> CategoryData {
    let mut elements = HashMap::new();
    for (mid, steps) in entries {
        elements.insert(*mid, fake_mission_xml(*mid, steps));
    }
    CategoryData { metadata, elements }
}

/// Pin the post-condition of `apply_mission_overrides`: every
/// registered override mutates the corresponding entry, the metadata
/// is bumped to a new value, and the returned overridden-elements
/// map names the patched ids.
#[test]
fn apply_mission_overrides_patches_entries_and_bumps_metadata() {
    let starting_metadata = 7538;
    let mut categories: HashMap<u32, CategoryData> = HashMap::new();
    // Match the live PAK shape: mission 622 has step 2113; mission
    // 641 has steps 2121, 3563, 3564.
    categories.insert(
        CATEGORY_MISSIONS,
        missions_category_with(
            &[(622, vec![2113]), (641, vec![2121, 3563, 3564])],
            starting_metadata,
        ),
    );

    let overridden = ResourceCache::apply_mission_overrides(&mut categories);

    let missions = categories
        .get(&CATEGORY_MISSIONS)
        .expect("missions category must remain after apply");

    // Metadata bumped to a content-derived non-zero offset.
    assert_ne!(
        missions.metadata, starting_metadata,
        "apply must bump the category metadata so the client refetches",
    );
    let bump = missions.metadata.wrapping_sub(starting_metadata);
    assert_eq!(bump & 0x1, 0x1, "low bit of bump must be set");

    // Each override's mission entry now contains the new step.
    let m622 = std::str::from_utf8(missions.elements.get(&622).unwrap()).unwrap();
    assert!(
        m622.contains("StepID=\"80622\""),
        "_622 must carry the equip-pistol step after override; got: {m622}",
    );
    let m641 = std::str::from_utf8(missions.elements.get(&641).unwrap()).unwrap();
    assert!(
        m641.contains("StepID=\"80641\""),
        "_641 must carry the equip-P90 step after override; got: {m641}",
    );

    // Returned map lists exactly the overridden ids, sorted ascending,
    // for the missions category.
    let ids = overridden
        .get(&CATEGORY_MISSIONS)
        .expect("missions must appear in returned map");
    assert_eq!(
        ids.as_slice(),
        &[622u32, 641u32],
        "overridden_elements must name both patched ids in ascending order",
    );
}

/// Idempotency at the bump level: running apply twice on the same
/// fresh input yields the same total metadata advancement (because
/// the bump is content-derived, not random). Two server starts in a
/// row see the same value and the client doesn't churn.
#[test]
fn apply_mission_overrides_bump_is_deterministic_per_content() {
    let first_meta = {
        let mut categories = HashMap::new();
        categories.insert(
            CATEGORY_MISSIONS,
            missions_category_with(&[(622, vec![2113]), (641, vec![2121, 3563, 3564])], 7538),
        );
        ResourceCache::apply_mission_overrides(&mut categories);
        categories.get(&CATEGORY_MISSIONS).unwrap().metadata
    };
    let second_meta = {
        let mut categories = HashMap::new();
        categories.insert(
            CATEGORY_MISSIONS,
            missions_category_with(&[(622, vec![2113]), (641, vec![2121, 3563, 3564])], 7538),
        );
        ResourceCache::apply_mission_overrides(&mut categories);
        categories.get(&CATEGORY_MISSIONS).unwrap().metadata
    };
    assert_eq!(first_meta, second_meta);
}

/// Defensive path: when the missions category itself is absent (PAK
/// missing on disk, e.g. for a partial bootstrap), apply_mission_
/// overrides must return an empty map and not panic. The warn log
/// is enough — server startup keeps going.
#[test]
fn apply_mission_overrides_no_op_when_category_missing() {
    let mut categories: HashMap<u32, CategoryData> = HashMap::new();
    let overridden = ResourceCache::apply_mission_overrides(&mut categories);
    assert!(
        overridden.is_empty(),
        "missing category must not produce override entries",
    );
}

/// Defensive path: when an override's target mission isn't present
/// in the loaded PAK, that single override is skipped (logged as a
/// warn) without aborting the rest. This protects a partial cooker
/// run or a Cimmeria override pointing at a mission that hasn't
/// shipped yet.
#[test]
fn apply_mission_overrides_skips_missing_target_mission() {
    let mut categories = HashMap::new();
    // Only mission 622 is present; mission 641's override should
    // skip with a warn but mission 622's still applies.
    categories.insert(
        CATEGORY_MISSIONS,
        missions_category_with(&[(622, vec![2113])], 7538),
    );

    let overridden = ResourceCache::apply_mission_overrides(&mut categories);

    let ids = overridden.get(&CATEGORY_MISSIONS).unwrap();
    assert!(ids.contains(&622), "mission 622 override must still apply",);
    assert!(
        !ids.contains(&641),
        "mission 641 must be skipped when entry is absent",
    );
}

/// `overridden_elements` accessor returns sorted slice for known
/// categories and an empty slice for unknown ones (the latter is
/// what handle_version_info_request needs to fall back to the
/// legacy invalidate-all path).
#[test]
fn overridden_elements_returns_empty_slice_for_unknown_category() {
    let cache = ResourceCache {
        categories: Arc::new(HashMap::new()),
        overridden_elements: Arc::new(HashMap::new()),
    };
    assert!(
        cache.overridden_elements(999).is_empty(),
        "unknown category must yield empty slice",
    );
}

// ── apply_item_overrides on hand-built CategoryData ──────────────
// Mirrors the apply_mission_overrides tests above. The PAK-load path
// is integration-tested by running the real server; this exercises
// the in-memory mutation logic without ZIP IO.

/// Minimal Server-Build COOKED_ITEM that satisfies the
/// `apply_override` patcher's anchors (`IconLocation="..."` and
/// `<InventorySet ... MaxStackSize="..." .../>`). Each item gets a
/// distinct id so the test can pin per-id overrides.
fn fake_item_xml(item_id: u32, icon: &str, max_stack: u32) -> Vec<u8> {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
         <COOKED_ITEM ID=\"{item_id}\" \
         IconLocation=\"{icon}\" \
         Name=\"Item {item_id}\" Tier=\"1\">\
         <InventorySet IsDeletable=\"true\" MaxStackSize=\"{max_stack}\" />\
         </COOKED_ITEM>"
    )
    .into_bytes()
}

fn items_category_with(entries: &[(u32, &str, u32)], metadata: u32) -> CategoryData {
    let mut elements = HashMap::new();
    for (item_id, icon, max_stack) in entries {
        elements.insert(*item_id, fake_item_xml(*item_id, icon, *max_stack));
    }
    CategoryData { metadata, elements }
}

/// Post-condition pin: every registered item override mutates its
/// entry, the category metadata is bumped, and the overridden-ids
/// map names the patched items.
#[test]
fn apply_item_overrides_patches_entries_and_bumps_metadata() {
    let starting_metadata = 7542;
    let mut categories: HashMap<u32, CategoryData> = HashMap::new();
    // Slappack pre-fix shape (IconMissing + MaxStackSize=1) so the
    // patcher has something to rewrite. Other entries are present
    // to verify the patcher doesn't touch them.
    categories.insert(
        CATEGORY_ITEMS,
        items_category_with(
            &[
                (2893, "set:CoreWidgets image:IconMissing", 1),
                (4735, "set:CoreWidgets image:IconMissing", 1),
                (9999, "set:ItemIcon001 image:SomeOther", 5),
            ],
            starting_metadata,
        ),
    );

    let overridden = ResourceCache::apply_item_overrides(&mut categories);

    let items = categories
        .get(&CATEGORY_ITEMS)
        .expect("items category must remain after apply");

    assert_ne!(
        items.metadata, starting_metadata,
        "apply must bump the items metadata so the client invalidates and refetches",
    );
    let bump = items.metadata.wrapping_sub(starting_metadata);
    assert_eq!(bump & 0x1, 0x1, "low bit of bump must be set");

    // Slappack entries patched.
    for &id in &[2893u32, 4735u32] {
        let patched = std::str::from_utf8(
            items
                .elements
                .get(&id)
                .unwrap_or_else(|| panic!("item {id} missing")),
        )
        .unwrap();
        assert!(
            patched.contains("IconLocation=\"set:ItemIcon001 image:Medkit\""),
            "item {id} must carry the Medkit icon after override; got: {patched}",
        );
        assert!(
            patched.contains("MaxStackSize=\"10\""),
            "item {id} must carry MaxStackSize=10 after override; got: {patched}",
        );
    }

    // Untouched entry retains its original values byte-for-byte.
    let other = std::str::from_utf8(items.elements.get(&9999).unwrap()).unwrap();
    assert!(
        other.contains("IconLocation=\"set:ItemIcon001 image:SomeOther\""),
        "unrelated item 9999 must not be touched by item overrides"
    );
    assert!(
        other.contains("MaxStackSize=\"5\""),
        "unrelated item 9999 must keep its MaxStackSize=5"
    );

    let ids = overridden
        .get(&CATEGORY_ITEMS)
        .expect("items must appear in returned map");
    assert_eq!(
        ids.as_slice(),
        &[2893u32, 4735u32],
        "overridden_elements must name both slappack ids in ascending order",
    );
}

/// Defensive path: items category absent (PAK missing) → no-op
/// return, server startup keeps going. Mirrors the analogous
/// missions test.
#[test]
fn apply_item_overrides_no_op_when_category_missing() {
    let mut categories: HashMap<u32, CategoryData> = HashMap::new();
    let overridden = ResourceCache::apply_item_overrides(&mut categories);
    assert!(
        overridden.is_empty(),
        "missing items category must not produce override entries"
    );
}

/// `compute_item_metadata_bump` is deterministic across calls and
/// always sets the low bit. Companion of the mission-side pin
/// `compute_metadata_bump_low_bit_is_always_set`.
#[test]
fn compute_item_metadata_bump_is_deterministic_and_low_bit_set() {
    use super::super::item_overrides::ItemOverride;
    let overrides = [ItemOverride {
        item_id: 2893,
        new_icon_location: Some("set:ItemIcon001 image:Medkit"),
        new_max_stack_size: Some(10),
    }];
    let a = compute_item_metadata_bump(&overrides);
    let b = compute_item_metadata_bump(&overrides);
    assert_eq!(a, b, "same overrides must hash to the same bump");
    assert_eq!(a & 0x1, 0x1, "low bit must be set");

    // A change to either field changes the bump (so the client refetches).
    let different_icon = [ItemOverride {
        item_id: 2893,
        new_icon_location: Some("set:ItemIcon001 image:Spray_Injector"),
        new_max_stack_size: Some(10),
    }];
    assert_ne!(
        compute_item_metadata_bump(&overrides),
        compute_item_metadata_bump(&different_icon),
        "icon edit must change the bump",
    );
    let different_stack = [ItemOverride {
        item_id: 2893,
        new_icon_location: Some("set:ItemIcon001 image:Medkit"),
        new_max_stack_size: Some(99),
    }];
    assert_ne!(
        compute_item_metadata_bump(&overrides),
        compute_item_metadata_bump(&different_stack),
        "stack-size edit must change the bump",
    );
}

// ── pick_first_open_bag ─────────────────────────────────────────────────────

/// Happy path: a bag with room in the item's container set is picked.
#[test]
fn pick_first_open_bag_picks_first_valid_when_empty() {
    let slot_indices = HashMap::new();
    // Item can live in INV_MAIN (1) or INV_BANDOLIER (3). BAG_FILL_ORDER
    // visits BANDOLIER (3) before MAIN (1), so BANDOLIER wins.
    let container_sets = vec![INV_MAIN, INV_BANDOLIER];
    assert_eq!(
        pick_first_open_bag(&container_sets, &slot_indices),
        Some(INV_BANDOLIER),
        "BANDOLIER comes before MAIN in BAG_FILL_ORDER and both are empty"
    );
}

/// **Regression guard for the live 2026-06-02 bug:** when the
/// fill-order-preferred bag is full but a later valid bag still has
/// room, the picker must overflow to the later bag rather than dropping
/// the item.
///
/// Pre-fix the inline `.find()` in `character_create` only checked
/// "does this bag accept this item type" — it ignored fullness — so
/// once the preferred bag filled up, every subsequent same-type item
/// got `continue`'d. Real impact: starter item 4343 disappeared at
/// character create.
///
/// Reverting `pick_first_open_bag` to ignore fullness must fail this
/// assertion.
#[test]
fn pick_first_open_bag_overflows_when_preferred_bag_is_full() {
    // Item can live in BANDOLIER (3, max 4) or MAIN (1, max 40).
    let container_sets = vec![INV_MAIN, INV_BANDOLIER];
    let mut slot_indices = HashMap::new();
    // Bandolier already at max: next_slot would be 4, == bag_max_slots(3).
    slot_indices.insert(INV_BANDOLIER, bag_max_slots(INV_BANDOLIER));
    assert_eq!(
        pick_first_open_bag(&container_sets, &slot_indices),
        Some(INV_MAIN),
        "BANDOLIER full → overflow to MAIN. Reverting to the pre-fix \
         'ignore fullness' selection would return BANDOLIER and then the \
         caller would drop the item."
    );
}

/// All valid bags full → `None`. Caller is expected to log "all valid
/// containers full" and drop the item, but the picker just reports the
/// fact — keeps the policy where it belongs.
#[test]
fn pick_first_open_bag_returns_none_when_all_valid_bags_full() {
    let container_sets = vec![INV_MAIN, INV_BANDOLIER];
    let mut slot_indices = HashMap::new();
    slot_indices.insert(INV_BANDOLIER, bag_max_slots(INV_BANDOLIER));
    slot_indices.insert(INV_MAIN, bag_max_slots(INV_MAIN));
    assert_eq!(pick_first_open_bag(&container_sets, &slot_indices), None);
}

/// Item with no valid container → `None`. Distinct from "all full" at
/// the caller level (different warn message), but at the picker level
/// the answer is the same.
#[test]
fn pick_first_open_bag_returns_none_when_no_valid_container() {
    // Container 17 is out of BAG_FILL_ORDER entirely.
    let container_sets = vec![17];
    let slot_indices = HashMap::new();
    assert_eq!(pick_first_open_bag(&container_sets, &slot_indices), None);
}

/// Edge case: a bag with one slot free (`next_slot == max - 1`) is
/// picked, and a bag at exactly max is rejected. Guards the boundary
/// arithmetic between `<` and `<=`.
#[test]
fn pick_first_open_bag_boundary_one_slot_free_vs_at_max() {
    let container_sets = vec![INV_BANDOLIER];
    let mut indices = HashMap::new();
    indices.insert(INV_BANDOLIER, bag_max_slots(INV_BANDOLIER) - 1);
    assert_eq!(
        pick_first_open_bag(&container_sets, &indices),
        Some(INV_BANDOLIER),
        "one slot free must be pickable"
    );

    indices.insert(INV_BANDOLIER, bag_max_slots(INV_BANDOLIER));
    assert_eq!(
        pick_first_open_bag(&container_sets, &indices),
        None,
        "exactly at max must NOT be pickable"
    );
}

/// Multi-item placement walk: simulate placing N items, ensuring the
/// picker advances across bags as they fill. End-to-end shape of the
/// production loop in `character_create`.
#[test]
fn pick_first_open_bag_multi_item_walk_fills_then_overflows() {
    let container_sets = vec![INV_MAIN, INV_BANDOLIER];
    let mut slot_indices: HashMap<i32, i32> = HashMap::new();
    let mut placements: Vec<i32> = Vec::new();

    // Place 6 items. First 4 land in BANDOLIER (slots 0..3), next 2
    // land in MAIN (slots 0, 1).
    for _ in 0..6 {
        let bag = pick_first_open_bag(&container_sets, &slot_indices)
            .expect("six placements with this fixture must all succeed");
        placements.push(bag);
        // Mirror the loop's bookkeeping.
        *slot_indices
            .entry(bag)
            .or_insert_with(|| bag_min_slot(bag)) += 1;
    }
    assert_eq!(
        placements,
        vec![
            INV_BANDOLIER,
            INV_BANDOLIER,
            INV_BANDOLIER,
            INV_BANDOLIER,
            INV_MAIN,
            INV_MAIN
        ],
        "first 4 fill BANDOLIER, next 2 overflow to MAIN"
    );
}
