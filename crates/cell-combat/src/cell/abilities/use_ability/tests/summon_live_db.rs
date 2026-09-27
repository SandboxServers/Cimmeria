//! Live-DB guards that the seeded summon abilities are what the PT-03 summon
//! path and its unit fixture (`summon.rs`) assume.
//!
//! The unit tests hard-code 2826's row. These pin the seed to it, and pin
//! the flag fix: no seeded ability may carry `AF_CHANNEL_ALLOWS_MOVEMENT`,
//! and every summon carries `SpeedPet`. While the channel flag was bit 14
//! both statements were about the same bit, and every summon warmed up
//! immune to the move interrupt.

use cimmeria_entity::abilities::{AF_CHANNEL_ALLOWS_MOVEMENT, AF_SPEED_PET, TARGET_SELF};

use crate::cell::spawner::{load_ability_defs, load_pet_summons};
use crate::test_support::require_db_or_skip;

/// 2826 Summon Straegis, as the unit fixture models it.
const SUMMON_STRAEGIS: i32 = 2826;

/// Every `pet_summons` ability is a warmed-up Self cast with `SpeedPet`,
/// an event set, and no move exemption. 2826 matches the unit fixture
/// exactly (warmup 6 s, cooldown 5 s, flags 18192, event set 1121).
#[tokio::test]
async fn seeded_summons_match_the_summon_path() {
    let pool = require_db_or_skip!();
    let defs = load_ability_defs(&pool).await.expect("abilities load");
    let summons = load_pet_summons(&pool).await.expect("pet summons load");
    assert!(
        summons.pet_summon_for(SUMMON_STRAEGIS).is_some(),
        "2826 must have its pet_summons row"
    );

    let mut ids: Vec<i32> = defs
        .keys()
        .copied()
        .filter(|id| summons.pet_summon_for(*id).is_some())
        .collect();
    ids.sort_unstable();
    assert_eq!(
        ids.len(),
        summons.len(),
        "every summon row names a loaded ability"
    );
    for id in ids {
        let def = &defs[&id];
        assert_eq!(
            def.target_type_id, TARGET_SELF,
            "{id}: a summon is a Self cast"
        );
        assert!(def.warmup > 0.0, "{id}: the warmup is the spawn timer");
        assert_ne!(
            def.flags & AF_SPEED_PET,
            0,
            "{id}: SpeedPet scales the warmup"
        );
        assert_eq!(
            def.flags & AF_CHANNEL_ALLOWS_MOVEMENT,
            0,
            "{id}: a summon warmup must be interruptible by movement"
        );
        assert!(
            def.event_set_id.is_some(),
            "{id}: the summon needs its cast VFX"
        );
    }

    let straegis = &defs[&SUMMON_STRAEGIS];
    assert_eq!(straegis.flags, 18192);
    assert!((straegis.warmup - 6.0).abs() < 1e-6);
    assert!((straegis.cooldown - 5.0).abs() < 1e-6);
    assert_eq!(straegis.event_set_id, Some(1121));
}

/// `AF_CHANNEL_ALLOWS_MOVEMENT` is a Cimmeria-side bit that no seeded
/// ability sets, so moving it off the client's `SpeedPet` bit changed no
/// channel's behaviour.
#[tokio::test]
async fn no_seeded_ability_sets_the_channel_movement_flag() {
    let pool = require_db_or_skip!();
    let defs = load_ability_defs(&pool).await.expect("abilities load");
    let mut flagged: Vec<i32> = defs
        .values()
        .filter(|d| d.flags & AF_CHANNEL_ALLOWS_MOVEMENT != 0)
        .map(|d| d.ability_id)
        .collect();
    flagged.sort_unstable();
    assert!(
        flagged.is_empty(),
        "abilities carrying AF_CHANNEL_ALLOWS_MOVEMENT: {flagged:?}"
    );
}
