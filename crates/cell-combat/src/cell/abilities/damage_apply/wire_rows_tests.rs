//! AB-T4 guards for a hit's wire burst (`hit_wire`): the `onEffectResults`
//! and `onStatUpdate` rows on `abilities.wire`, with the hit's cast.

use super::single_damage_path_tests::seq_rolling;
use super::tests::{drain, make_ability, make_mgr_player_vs_npc};
use super::*;
use crate::test_support::{Captured, LogCapture};
use cimmeria_entity::abilities::EffectDef;
use cimmeria_entity::stats::FOCUS;

const PISTOL_SHOT: i32 = 592;
const PISTOL_SHOT_EFFECT: i32 = 654;
const NPC: u32 = 2;

fn rows(all: &[Captured], method: &str) -> Vec<Captured> {
    all.iter()
        .filter(|c| {
            c.target == "abilities.wire"
                && c.has_field("event", "wire_sent")
                && c.has_field("method", method)
        })
        .cloned()
        .collect()
}

fn field<'a>(c: &'a Captured, key: &str) -> &'a str {
    c.fields
        .get(key)
        .map_or_else(|| panic!("row has no `{key}`: {c:?}"), String::as_str)
}

/// **Guard (AB-T4).** A Pistol Shot hit on an NPC with an empty Focus pool
/// (the 65 Health bleed): one `onEffectResults` row carrying the cast's
/// `cast_id`, the effect id it sent (`effect_seq`), the result code and the
/// one `(stat, delta)` pair; one `onStatUpdate` row for the NPC's Health,
/// counted over its one witness. Fails if `hit_wire` goes back to the plain
/// routing helpers (no rows) or stops naming the ability.
#[tokio::test]
async fn a_hit_logs_its_effect_results_and_stat_update_rows() {
    let mut mgr = make_mgr_player_vs_npc();
    let effect = EffectDef {
        effect_id: PISTOL_SHOT_EFFECT,
        script_name: Some("RangedPhysicalDamage".into()),
        params: [("HealthDamage", 15), ("FocusDamage", 150)]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        ..Default::default()
    };
    let ability = make_ability(PISTOL_SHOT, vec![PISTOL_SHOT_EFFECT]);
    mgr.ability_defs.insert(PISTOL_SHOT, ability.clone());
    mgr.effect_defs.insert(PISTOL_SHOT_EFFECT, effect);
    let npc = mgr.get_entity_mut(NPC).unwrap();
    for (stat, cur) in [(HEALTH, 1000), (FOCUS, 0)] {
        let s = npc.stats.get_mut(stat).unwrap();
        s.update(0, cur, 1000);
        s.clear_dirty();
    }
    let seq = seq_rolling(&mgr, (1, NPC), PISTOL_SHOT, false);
    let (tx, mut rx) = mpsc::channel(256);
    let logs = LogCapture::install();

    let outer = mgr.enter_cast_scope(Some(seq as i32));
    apply_damage_to_target(
        1,
        NPC,
        PISTOL_SHOT,
        &Some(ability),
        seq,
        false,
        &tx,
        &mut mgr,
    )
    .await;
    mgr.exit_cast_scope(outer);
    drain(&mut rx);

    let all = logs.all();
    let results = rows(&all, "onEffectResults");
    assert_eq!(
        results.len(),
        1,
        "an NPC target gets no second fan-out: {all:#?}"
    );
    let r = &results[0];
    assert_eq!(field(r, "entity_id"), "1");
    assert_eq!(field(r, "player_id"), "100");
    assert_eq!(field(r, "ability_id"), PISTOL_SHOT.to_string());
    assert_eq!(
        field(r, "effect_id"),
        seq.to_string(),
        "the id the client got"
    );
    assert_eq!(field(r, "cast_id"), seq.to_string());
    assert_eq!(field(r, "target_id"), "2");
    assert_eq!(field(r, "results_count"), "1");
    assert_eq!(field(r, "results"), format!("{HEALTH}:-65"));
    assert!(r.fields.contains_key("result_code"));
    assert_eq!(field(r, "self_sent"), "true");

    let stats = rows(&all, "onStatUpdate");
    assert_eq!(stats.len(), 1, "{all:#?}");
    let s = &stats[0];
    assert_eq!(field(s, "entity_id"), "2");
    assert_eq!(field(s, "witness_count"), "1");
    assert_eq!(field(s, "witness_player_ids"), "100");
    assert_eq!(field(s, "ability_id"), PISTOL_SHOT.to_string());
    assert!(
        field(s, "stats").contains(&format!("{HEALTH}:935/1000")),
        "the NPC's Health after the bleed: {s:?}"
    );
}
