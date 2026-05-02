use super::*;

#[test]
fn stat_new_and_fields() {
    let s = Stat::new(0, 50, 100, 0, 50, 100);
    assert_eq!(s.min, 0);
    assert_eq!(s.cur, 50);
    assert_eq!(s.max, 100);
    assert_eq!(s.base_min, 0);
    assert_eq!(s.base_cur, 50);
    assert_eq!(s.base_max, 100);
    assert!(!s.dirty);
    assert!(!s.base_dirty);
}

#[test]
fn stat_update_marks_dirty() {
    let mut s = Stat::new(0, 50, 100, 0, 50, 100);
    s.update(10, 60, 90);
    assert!(s.dirty);
    assert_eq!(s.min, 10);
    assert_eq!(s.cur, 60);
    assert_eq!(s.max, 90);
}

#[test]
fn stat_set_current_clamps() {
    let mut s = Stat::new(0, 50, 100, 0, 50, 100);
    assert_eq!(s.set_current(200), 100);
    assert_eq!(s.set_current(-10), 0);
    assert_eq!(s.set_current(75), 75);
    assert!(s.dirty);
}

#[test]
fn stat_change_clamps() {
    let mut s = Stat::new(0, 50, 100, 0, 50, 100);
    assert_eq!(s.change(30), 30);
    assert_eq!(s.cur, 80);
    assert_eq!(s.change(30), 20); // clamped at 100
    assert_eq!(s.cur, 100);
    assert_eq!(s.change(-150), -100); // clamped at 0
    assert_eq!(s.cur, 0);
}

#[test]
fn stat_set_max_clamps_current() {
    let mut s = Stat::new(0, 80, 100, 0, 80, 100);
    s.set_max(50);
    assert_eq!(s.max, 50);
    assert_eq!(s.cur, 50);
}

#[test]
fn stat_set_min_clamps_current() {
    let mut s = Stat::new(0, 10, 100, 0, 10, 100);
    s.set_min(20);
    assert_eq!(s.min, 20);
    assert_eq!(s.cur, 20);
}

#[test]
fn stat_set_max_below_min_pulls_min_down() {
    // Regression for #102: set_max below current min must pull min down
    // and keep `min ≤ cur ≤ max` so we never serialize min > max.
    let mut s = Stat::new(80, 90, 100, 80, 90, 100);
    s.set_max(50);
    assert_eq!(s.min, 50);
    assert_eq!(s.cur, 50);
    assert_eq!(s.max, 50);
    assert!(s.min <= s.cur && s.cur <= s.max);
}

#[test]
fn stat_set_min_above_max_pushes_max_up() {
    // Regression for #102: set_min above current max must push max up.
    let mut s = Stat::new(0, 50, 100, 0, 50, 100);
    s.set_min(200);
    assert_eq!(s.min, 200);
    assert_eq!(s.cur, 200);
    assert_eq!(s.max, 200);
    assert!(s.min <= s.cur && s.cur <= s.max);
}

#[test]
fn stat_change_by_percent() {
    let mut s = Stat::new(0, 200, 1000, 0, 200, 1000);
    let delta = s.change_by_percent(0.5); // 50% of 200 = 100
    assert_eq!(delta, 100);
    assert_eq!(s.cur, 300);
}

#[test]
fn stat_change_by_max_percent() {
    let mut s = Stat::new(0, 200, 1000, 0, 200, 1000);
    let delta = s.change_by_max_percent(0.1); // 10% of 1000 = 100
    assert_eq!(delta, 100);
    assert_eq!(s.cur, 300);
}

#[test]
fn stat_set_base_marks_base_dirty() {
    let mut s = Stat::new(0, 50, 100, 0, 50, 100);
    s.set_base(10, 60, 200);
    assert!(s.base_dirty);
    assert_eq!(s.base_min, 10);
    assert_eq!(s.base_cur, 60);
    assert_eq!(s.base_max, 200);
}

#[test]
fn stat_clear_dirty() {
    let mut s = Stat::new(0, 50, 100, 0, 50, 100);
    s.dirty = true;
    s.base_dirty = true;
    s.clear_dirty();
    assert!(!s.dirty);
    assert!(!s.base_dirty);
}

#[test]
fn statlist_default_has_all_stats() {
    let list = StatList::new();
    // Check a sample of stats exist with expected defaults
    let health = list.get(HEALTH).unwrap();
    assert_eq!(health.cur, 100);
    assert_eq!(health.max, 100);

    let coord = list.get(COORDINATION).unwrap();
    assert_eq!(coord.cur, 1);
    assert_eq!(coord.max, 1);

    let speed = list.get(MOVEMENT_SPEED_MOD).unwrap();
    assert_eq!(speed.cur, 100);
    assert_eq!(speed.max, 500);

    let accuracy = list.get(ACCURACY).unwrap();
    assert_eq!(accuracy.min, -1000);
    assert_eq!(accuracy.max, 1000);

    // Should have 70+ stats
    assert!(list.len() >= 70);
}

#[test]
fn statlist_apply_archetype() {
    let mut list = StatList::new();
    let arch = ArchetypeStatValues {
        coordination: 15,
        engagement: 10,
        fortitude: 12,
        morale: 13,
        perception: 14,
        intelligence: 11,
        health: 500,
        focus: 200,
        health_per_level: 10,
        focus_per_level: 70,
    };
    list.apply_archetype(&arch);

    let health = list.get(HEALTH).unwrap();
    assert_eq!(health.cur, 500);
    assert_eq!(health.max, 500);
    assert_eq!(health.base_cur, 500);
    assert_eq!(health.base_max, 500);

    let coord = list.get(COORDINATION).unwrap();
    assert_eq!(coord.cur, 15);

    let kres = list.get(KINETIC_RES).unwrap();
    assert_eq!(kres.cur, 40);
    assert_eq!(kres.max, 2000);

    let deploy = list.get(DEPLOYMENT_BAR_AMMO).unwrap();
    assert_eq!(deploy.max, 1);
}

#[test]
fn statlist_serialize_all_wire_format() {
    let mut list = StatList::new();
    // Apply archetype so we have known values
    let arch = ArchetypeStatValues {
        coordination: 15, engagement: 10, fortitude: 12,
        morale: 13, perception: 14, intelligence: 11,
        health: 500, focus: 200,
        health_per_level: 10, focus_per_level: 70,
    };
    list.apply_archetype(&arch);

    let data = list.serialize_all();
    // First 4 bytes = count (u32 LE)
    let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    assert_eq!(count as usize, list.len());
    // Total size = 4 + count * 16
    assert_eq!(data.len(), 4 + (count as usize) * 16);

    // Verify one specific entry exists in the serialized data
    // Each entry is (stat_id:i32, min:i32, cur:i32, max:i32) = 16 bytes
    let mut found_health = false;
    for i in 0..count as usize {
        let offset = 4 + i * 16;
        let id = i32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
        if id == HEALTH {
            let cur = i32::from_le_bytes([data[offset+8], data[offset+9], data[offset+10], data[offset+11]]);
            let max = i32::from_le_bytes([data[offset+12], data[offset+13], data[offset+14], data[offset+15]]);
            assert_eq!(cur, 500);
            assert_eq!(max, 500);
            found_health = true;
        }
    }
    assert!(found_health, "Health stat should be in serialized data");
}

#[test]
fn statlist_serialize_dirty_only() {
    let mut list = StatList::new();
    // Nothing is dirty initially
    let data = list.serialize_dirty();
    let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    assert_eq!(count, 0);

    // Dirty one stat
    list.get_mut(HEALTH).unwrap().set_current(50);
    let data = list.serialize_dirty();
    let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    assert_eq!(count, 1);
    // Verify it's health
    let id = i32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    assert_eq!(id, HEALTH);
}

#[test]
fn statlist_serialize_public() {
    let list = StatList::new();
    let data = list.serialize_public();
    let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    assert_eq!(count as usize, PUBLIC_STATS.len());
}

#[test]
fn statlist_clear_dirty() {
    let mut list = StatList::new();
    list.get_mut(HEALTH).unwrap().set_current(50);
    list.get_mut(FOCUS).unwrap().set_base(0, 100, 200);
    assert!(list.has_dirty());
    list.clear_dirty();
    assert!(!list.has_dirty());
}

#[test]
fn statlist_serialize_base_values() {
    let mut list = StatList::new();
    let arch = ArchetypeStatValues {
        coordination: 15, engagement: 10, fortitude: 12,
        morale: 13, perception: 14, intelligence: 11,
        health: 500, focus: 200,
        health_per_level: 10, focus_per_level: 70,
    };
    list.apply_archetype(&arch);

    let data = list.serialize_all_base();
    let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    assert_eq!(count as usize, list.len());

    // Find health and verify base values
    let mut found = false;
    for i in 0..count as usize {
        let offset = 4 + i * 16;
        let id = i32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
        if id == HEALTH {
            let cur = i32::from_le_bytes([data[offset+8], data[offset+9], data[offset+10], data[offset+11]]);
            let max = i32::from_le_bytes([data[offset+12], data[offset+13], data[offset+14], data[offset+15]]);
            assert_eq!(cur, 500);
            assert_eq!(max, 500);
            found = true;
        }
    }
    assert!(found);
}

#[test]
fn stat_ids_match_python_enums() {
    // Verify a selection of stat IDs match python/Atrea/enums.py
    assert_eq!(COORDINATION, 0);
    assert_eq!(HEALTH, 7);
    assert_eq!(FOCUS, 8);
    assert_eq!(KINETIC_RES, 29);
    assert_eq!(MENTAL_RES, 34);
    assert_eq!(HEALTH_RES, 40);
    assert_eq!(AMMO_SLOT_1, 49);
    assert_eq!(DEPLOYMENT_BAR_AMMO, 54);
    assert_eq!(STEALTH_MOVEMENT, 70);
    assert_eq!(ROTATION_SPEED_MOD, 81);
    assert_eq!(ABSORB_PHYSICAL, 89);
    assert_eq!(SPEED_RELOAD, 104);
}

#[test]
fn scale_for_level_increases_health_and_focus() {
    let mut list = StatList::new();
    let arch = ArchetypeStatValues {
        coordination: 5, engagement: 4, fortitude: 3, morale: 4,
        perception: 3, intelligence: 2, health: 760, focus: 1570,
        health_per_level: 10, focus_per_level: 70,
    };
    list.apply_archetype(&arch);

    // Level 1: base values
    assert_eq!(list.get(HEALTH).unwrap().max, 760);
    assert_eq!(list.get(FOCUS).unwrap().max, 1570);

    // Scale to level 5: health = 760 + 10*(5-1) = 800, focus = 1570 + 70*(5-1) = 1850
    list.scale_for_level(5, &arch);
    let health = list.get(HEALTH).unwrap();
    assert_eq!(health.max, 800);
    assert_eq!(health.cur, 800);
    // Regression for #104: base values must track the new max on level-up,
    // otherwise serialize_all_base / serialize_dirty_base report stale data.
    assert_eq!(health.base_max, 800);
    assert_eq!(health.base_cur, 800);
    let focus = list.get(FOCUS).unwrap();
    assert_eq!(focus.max, 1850);
    assert_eq!(focus.cur, 1850);
    assert_eq!(focus.base_max, 1850);
    assert_eq!(focus.base_cur, 1850);
    assert!(list.has_dirty());
}

#[test]
fn scale_for_level_1_is_base() {
    let mut list = StatList::new();
    let arch = ArchetypeStatValues {
        coordination: 5, engagement: 4, fortitude: 3, morale: 4,
        perception: 3, intelligence: 2, health: 760, focus: 1570,
        health_per_level: 10, focus_per_level: 70,
    };
    list.apply_archetype(&arch);
    list.clear_dirty();
    list.scale_for_level(1, &arch);
    // Level 1: no bonus
    assert_eq!(list.get(HEALTH).unwrap().max, 760);
    assert_eq!(list.get(FOCUS).unwrap().max, 1570);
}

#[test]
fn scale_for_level_0_treated_as_1() {
    // Regression: level == 0 must not produce a negative bonus.
    // The DB column allows 0 but the formula needs a 1-based level —
    // without the clamp, max would be base - per_level (smaller than the
    // archetype baseline) and propagate that incorrect value into the
    // stat state. Copilot caught this on PR #107.
    let mut list = StatList::new();
    let arch = ArchetypeStatValues {
        coordination: 5, engagement: 4, fortitude: 3, morale: 4,
        perception: 3, intelligence: 2, health: 760, focus: 1570,
        health_per_level: 10, focus_per_level: 70,
    };
    list.apply_archetype(&arch);
    list.scale_for_level(0, &arch);
    // Same result as level 1: no bonus, max == archetype base.
    assert_eq!(list.get(HEALTH).unwrap().max, 760);
    assert_eq!(list.get(FOCUS).unwrap().max, 1570);
}
