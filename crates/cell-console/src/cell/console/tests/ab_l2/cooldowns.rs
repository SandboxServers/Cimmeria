//! `.cooldowns [reset [abilityId]]`: the server's cooldowns cleared AND the
//! client's sweep stopped with the clear `onTimerUpdate`.

use std::time::Duration;

use cimmeria_entity::abilities::AbilityDef;
use tracing::Level;

use super::{console, lines, timers, world, CALLER};
use crate::test_support::LogCapture;

const MONIKER: i64 = 3_212_632_871;

/// `onTimerUpdate` clearing ability 592's cooldown on entity 1's client:
/// `id 592, type 2 (TIMER_ABILITY_COOLDOWN), source 1, secondary 0,
/// total 0.0, complete 0.0` — the bytes the warmup interrupt sends when it
/// refunds a cooldown.
const CLEAR_592: [u8; 21] = [
    0x50, 0x02, 0x00, 0x00, // id = 592
    0x02, // type = TIMER_ABILITY_COOLDOWN
    0x01, 0x00, 0x00, 0x00, // source = caster 1
    0x00, 0x00, 0x00, 0x00, // secondary = 0
    0x00, 0x00, 0x00, 0x00, // total = 0.0
    0x00, 0x00, 0x00, 0x00, // complete = 0.0
];

fn def(id: i32, monikers: Vec<i64>) -> AbilityDef {
    AbilityDef {
        ability_id: id,
        name: format!("Ability{id}"),
        cooldown: 30.0,
        warmup: 0.0,
        flags: 0,
        is_ranged: false,
        min_range: 0.0,
        max_range: 0.0,
        target_type_id: 0,
        effect_ids: vec![],
        moniker_ids: monikers,
        required_ammo: 0,
        event_set_id: None,
        velocity: 0.0,
        type_id: Default::default(),
        passive: false,
    }
}

fn with_cooldowns() -> crate::cell::space_manager::SpaceManager {
    let (mut mgr, _npc) = world(2);
    mgr.ability_defs.insert(592, def(592, vec![MONIKER]));
    mgr.ability_defs.insert(637, def(637, vec![]));
    let caller = mgr.get_entity_mut(CALLER).unwrap();
    caller
        .abilities
        .start_cooldowns(&def(592, vec![MONIKER]), 30.0);
    caller
        .abilities
        .start_ability_cooldown(637, Duration::from_secs(10));
    mgr
}

/// Byte-exact: the one-ability reset sends exactly [`CLEAR_592`] to the
/// caller's own client, and the server forgets the cooldown and its moniker
/// group. Revert proof: drop the send and `timers` is empty while the server
/// is clear, the desync this command exists to fix.
#[tokio::test]
async fn ab_l2_cooldowns_reset_one_clears_the_server_and_sends_the_clear_timer() {
    let mut mgr = with_cooldowns();
    let logs = LogCapture::install();

    let msgs = console(&mut mgr, None, ".cooldowns reset 592").await;

    assert_eq!(timers(&msgs), vec![(CALLER, CLEAR_592.to_vec())]);
    let caller = mgr.get_entity(CALLER).unwrap();
    assert!(!caller.abilities.is_on_cooldown(592));
    assert!(!caller.abilities.is_moniker_on_cooldown(MONIKER));
    assert!(caller.abilities.is_on_cooldown(637), "only the one named");
    let out = lines(&msgs);
    assert_eq!(
        out,
        vec![
            "cooldowns reset 592: was running, 1 moniker group(s) cleared; your client was sent the clear"
                .to_string()
        ]
    );
    let row = logs
        .find_message(Level::INFO, "GM cleared cooldowns")
        .expect("one abilities.gm row");
    assert_eq!(row.target, "abilities.gm");
    assert!(row.has_field("event", "cooldowns_reset"));
    assert!(row.has_field("requested_ability_id", "592"));
    assert!(row.has_field("player_id", "71"));
}

/// With nothing running server-side the clear still goes out: that is the
/// form a GM uses when the client's sweep and the server disagree.
#[tokio::test]
async fn ab_l2_cooldowns_reset_one_sends_the_clear_even_when_the_server_has_none() {
    let (mut mgr, _npc) = world(2);
    let msgs = console(&mut mgr, None, ".cooldowns reset 592").await;
    assert_eq!(timers(&msgs), vec![(CALLER, CLEAR_592.to_vec())]);
    assert!(lines(&msgs)[0].contains("was not running server-side"));
}

#[tokio::test]
async fn ab_l2_cooldowns_reset_all_clears_every_running_cooldown() {
    let mut mgr = with_cooldowns();

    let msgs = console(&mut mgr, None, ".cooldowns reset").await;

    let sent: Vec<i32> = timers(&msgs)
        .iter()
        .map(|(eid, args)| {
            assert_eq!(*eid, CALLER, "the caller's own client only");
            assert_eq!(args.len(), 21);
            assert_eq!(args[4], 2, "TIMER_ABILITY_COOLDOWN");
            assert_eq!(&args[13..21], &[0u8; 8], "zero total and complete");
            i32::from_le_bytes(args[0..4].try_into().unwrap())
        })
        .collect();
    assert_eq!(sent, vec![592, 637]);
    assert_eq!(timers(&msgs)[0].1, CLEAR_592.to_vec());
    let caller = mgr.get_entity(CALLER).unwrap();
    assert!(!caller.abilities.is_on_cooldown(592));
    assert!(!caller.abilities.is_on_cooldown(637));
    assert!(!caller.abilities.is_moniker_on_cooldown(MONIKER));
    assert_eq!(
        lines(&msgs),
        vec![
            "cooldowns reset: 2 ability cooldown(s) cleared (592, 637), 1 moniker group(s); your client was sent the clears"
                .to_string()
        ]
    );
}

#[tokio::test]
async fn ab_l2_cooldowns_lists_and_rejects_bad_arguments_visibly() {
    let mut mgr = with_cooldowns();
    let out = lines(&console(&mut mgr, None, ".cooldowns").await);
    assert_eq!(out.len(), 1);
    assert!(
        out[0].starts_with("cooldowns: 592 ") && out[0].contains("; 637 "),
        "{out:?}"
    );

    for bad in [
        ".cooldowns reset abc",
        ".cooldowns reset -4",
        ".cooldowns wipe",
    ] {
        let msgs = console(&mut mgr, None, bad).await;
        assert!(timers(&msgs).is_empty(), "{bad}: nothing sent");
        assert_eq!(lines(&msgs).len(), 1, "{bad}: one visible line");
    }
    assert!(
        mgr.get_entity(CALLER)
            .unwrap()
            .abilities
            .is_on_cooldown(592),
        "a bad argument changes nothing"
    );
}

/// `.cooldowns` acts on the caller only: a selected player keeps theirs.
#[tokio::test]
async fn ab_l2_cooldowns_never_touches_the_selected_player() {
    let mut mgr = with_cooldowns();
    const WITNESS: u32 = 2;
    mgr.get_entity_mut(WITNESS)
        .unwrap()
        .abilities
        .start_ability_cooldown(592, Duration::from_secs(30));

    let msgs = console(&mut mgr, Some(WITNESS), ".cooldowns reset").await;

    assert!(timers(&msgs).iter().all(|(eid, _)| *eid == CALLER));
    assert!(mgr
        .get_entity(WITNESS)
        .unwrap()
        .abilities
        .is_on_cooldown(592));
}
