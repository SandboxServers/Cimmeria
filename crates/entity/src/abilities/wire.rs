//! Wire-format helpers for the ability system's client-facing messages
//! (`onEffectResults`, `onTimerUpdate`).

/// A single stat change result sent to the client in `onEffectResults`.
///
/// Wire per entry: `stat_id:i8, delta:i32, damage_code:i8, stat_result_code:i8` (7 bytes).
#[derive(Debug, Clone)]
pub struct ClientEffectResult {
    pub stat_id: i8,
    pub delta: i32,
    pub damage_code: i8,
    pub stat_result_code: i8,
}

impl ClientEffectResult {
    pub fn serialize(&self, buf: &mut Vec<u8>) {
        buf.push(self.stat_id as u8);
        buf.extend_from_slice(&self.delta.to_le_bytes());
        buf.push(self.damage_code as u8);
        buf.push(self.stat_result_code as u8);
    }
}

/// Serialize `onTimerUpdate` arguments.
///
/// Wire: `id:i32, type:i8, sourceId:i32, secondaryId:i32, totalTime:f32, expireTime:f32`.
pub fn serialize_timer_update(
    id: i32,
    timer_type: i8,
    source_id: i32,
    total_time: f32,
    expire_time: f32,
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(21);
    buf.extend_from_slice(&id.to_le_bytes());
    buf.push(timer_type as u8);
    buf.extend_from_slice(&source_id.to_le_bytes());
    buf.extend_from_slice(&0i32.to_le_bytes()); // secondaryId always 0
    buf.extend_from_slice(&total_time.to_le_bytes());
    buf.extend_from_slice(&expire_time.to_le_bytes());
    buf
}

/// Serialize `onEffectResults` arguments.
///
/// Wire: `sourceId:i32, abilityId:i32, effectId:i32, targetId:i32,
///        resultCode:u8, count:u32, [ClientEffectResult...]`.
pub fn serialize_effect_results(
    source_id: i32,
    ability_id: i32,
    effect_id: i32,
    target_id: i32,
    result_code: u8,
    stat_results: &[ClientEffectResult],
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(21 + stat_results.len() * 7);
    buf.extend_from_slice(&source_id.to_le_bytes());
    buf.extend_from_slice(&ability_id.to_le_bytes());
    buf.extend_from_slice(&effect_id.to_le_bytes());
    buf.extend_from_slice(&target_id.to_le_bytes());
    buf.push(result_code);
    buf.extend_from_slice(&(stat_results.len() as u32).to_le_bytes());
    for result in stat_results {
        result.serialize(&mut buf);
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::{
        DT_ENERGY, DT_PHYSICAL, RC_CRITICAL, RC_HIT, SRC_ABSORB, SRC_NONE, TIMER_ABILITY_COOLDOWN,
    };

    #[test]
    fn serialize_timer_update_format() {
        let data = serialize_timer_update(597, TIMER_ABILITY_COOLDOWN, 100, 5.0, 12345.0);
        assert_eq!(data.len(), 21);
        let id = i32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        assert_eq!(id, 597);
        assert_eq!(data[4], TIMER_ABILITY_COOLDOWN as u8);
    }

    #[test]
    fn serialize_effect_results_empty() {
        let data = serialize_effect_results(100, 597, 101, 200, RC_HIT, &[]);
        assert_eq!(data.len(), 21); // 4*4 + 1 + 4 = 21
        assert_eq!(data[16], RC_HIT);
        let count = u32::from_le_bytes([data[17], data[18], data[19], data[20]]);
        assert_eq!(count, 0);
    }

    #[test]
    fn serialize_effect_results_with_stats() {
        let results = vec![ClientEffectResult {
            stat_id: 10, // health
            delta: -50,
            damage_code: DT_PHYSICAL,
            stat_result_code: SRC_NONE,
        }];
        let data = serialize_effect_results(100, 597, 101, 200, RC_CRITICAL, &results);
        assert_eq!(data.len(), 21 + 7); // 21 header + 7 per result
        assert_eq!(data[16], RC_CRITICAL);
        let count = u32::from_le_bytes([data[17], data[18], data[19], data[20]]);
        assert_eq!(count, 1);
        // stat_id
        assert_eq!(data[21], 10);
    }

    #[test]
    fn client_effect_result_serialize() {
        let r = ClientEffectResult {
            stat_id: 10,
            delta: -100,
            damage_code: DT_ENERGY,
            stat_result_code: SRC_ABSORB,
        };
        let mut buf = Vec::new();
        r.serialize(&mut buf);
        assert_eq!(buf.len(), 7);
        assert_eq!(buf[0], 10); // stat_id
        let delta = i32::from_le_bytes([buf[1], buf[2], buf[3], buf[4]]);
        assert_eq!(delta, -100);
        assert_eq!(buf[5], DT_ENERGY as u8);
        assert_eq!(buf[6], SRC_ABSORB as u8);
    }
}
