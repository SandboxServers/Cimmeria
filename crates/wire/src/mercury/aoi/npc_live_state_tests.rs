//! Wire-format guard: the NPC `createOnClient()` cascade introduces the mob
//! with its LIVE `stateField` and HEALTH/FOCUS, as Python
//! `SGWBeing.createOnClient` did (`deprecated/python/cell/SGWBeing.py:507`,
//! `:514`). The cascade used to hardcode `onStateFieldUpdate(0)` and a
//! 100/100 HEALTH, so a corpse re-entering a witness's AoI was rebuilt
//! standing and alive (colo 2026-09-26, Hallway01_Guard 100162) —
//! `docs/reverse-engineering/findings/state-flag-broadcast.md` Bug A.

use super::compose_create_entity_cascade_body;
use crate::cell::messages::{NpcAoIData, NpcVitals};
use crate::mercury::method_idx;
use cimmeria_mercury::channel_bundle::EXTENDED_ENCODING_MARKER;

const NPC: u32 = 100_162;

/// Walk the cascade body's entity-method frames and return the args of the
/// first direct-encoded call to `method_index` on `entity_id`.
fn args_of(body: &[u8], entity_id: u32, method_index: u16) -> Option<Vec<u8>> {
    let mut i = 0;
    while i + 3 <= body.len() {
        let id = body[i];
        let len = u16::from_le_bytes([body[i + 1], body[i + 2]]) as usize;
        let payload = &body[i + 3..i + 3 + len];
        i += 3 + len;
        if id == EXTENDED_ENCODING_MARKER {
            continue;
        }
        let eid = u32::from_le_bytes(payload[..4].try_into().unwrap());
        if u16::from(id & 0x7F) == method_index && eid == entity_id {
            return Some(payload[4..].to_vec());
        }
    }
    None
}

/// `[stat_id, min, cur, max]` of `stat_id` inside an onStatUpdate payload.
fn stat_entry(args: &[u8], stat_id: i32) -> Option<[i32; 4]> {
    let count = u32::from_le_bytes(args[..4].try_into().unwrap()) as usize;
    (0..count).find_map(|n| {
        let e = &args[4 + 16 * n..20 + 16 * n];
        let v: Vec<i32> = e
            .chunks(4)
            .map(|c| i32::from_le_bytes(c.try_into().unwrap()))
            .collect();
        (v[0] == stat_id).then(|| [v[0], v[1], v[2], v[3]])
    })
}

fn corpse() -> NpcAoIData {
    NpcAoIData {
        body_set: Some("BS_HumanMale.BS_HumanMale".into()),
        components: vec!["BS_HumanMale.BS_HM_Hands_00".into()],
        faction: 10,
        state_field: 0x41, // BSF_DEAD | BSF_MOVEMENT_LOCK
        vitals: Some(Box::new(NpcVitals {
            health: [0, 0, 250],
            focus: [0, 0, 200],
        })),
        ..NpcAoIData::default()
    }
}

/// A corpse's cascade carries `onStateFieldUpdate(0x41)` and a 0/250 HEALTH.
/// Reverting either half (hardcoded 0 state, default 100/100 stats) fails.
#[test]
fn corpse_cascade_carries_live_state_field_and_health() {
    let body = compose_create_entity_cascade_body(NPC, 0x04, 1, Some(&corpse()));

    let state = args_of(&body, NPC, method_idx::ON_STATE_FIELD_UPDATE)
        .expect("cascade sends onStateFieldUpdate");
    assert_eq!(
        state,
        0x41u32.to_le_bytes().to_vec(),
        "a dead mob must be introduced with BSF_DEAD set, not 0 (alive)"
    );

    let stats =
        args_of(&body, NPC, method_idx::ON_STAT_UPDATE).expect("cascade sends onStatUpdate");
    assert_eq!(
        stat_entry(&stats, cimmeria_entity::stats::HEALTH),
        Some([cimmeria_entity::stats::HEALTH, 0, 0, 250]),
        "HEALTH must be the live 0/250, not the template 100/100"
    );
    assert_eq!(
        stat_entry(&stats, cimmeria_entity::stats::FOCUS),
        Some([cimmeria_entity::stats::FOCUS, 0, 0, 200]),
    );
}

/// No live data (`health: None`) keeps the template defaults, and an alive
/// mob still gets `onStateFieldUpdate(0)` — the fix changes nothing for the
/// common case.
#[test]
fn live_mob_without_stat_snapshot_keeps_template_defaults() {
    let npc = NpcAoIData {
        state_field: 0,
        vitals: None,
        ..corpse()
    };
    let body = compose_create_entity_cascade_body(NPC, 0x04, 1, Some(&npc));
    assert_eq!(
        args_of(&body, NPC, method_idx::ON_STATE_FIELD_UPDATE),
        Some(0u32.to_le_bytes().to_vec())
    );
    let stats = args_of(&body, NPC, method_idx::ON_STAT_UPDATE).unwrap();
    assert_eq!(
        stat_entry(&stats, cimmeria_entity::stats::HEALTH),
        Some([cimmeria_entity::stats::HEALTH, 0, 100, 100])
    );
}
