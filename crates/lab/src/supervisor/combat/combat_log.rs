//! `client_combat_log`: the floating-combat-text feed with a read cursor.
//!
//! The records come from the lab's wrapper around `SCTMod.onUnitCombat`
//! (see [`crate::supervisor::events::lua_rings`]), one per
//! `Events.UnitCombat`: ability id and name, hit type (`HitType.Hit`,
//! `Miss`, `Glance`, `Critical`, ...), source and target unit names and
//! whether each is the player, whether the hit was mortal, and every stat
//! change (`Stat.Health`, `Stat.Focus`, ...) with its value and result
//! code. Capture starts at the first pump after the client enters the
//! world (any `client_combat_log`, `client_wait_event` or
//! `client_use_ability` call installs it); earlier hits are not seen.

use serde_json::{json, Map, Value};

use crate::supervisor::events::store::{StoredEvent, STORE_CAP};
use crate::supervisor::events::KIND_COMBAT;
use crate::supervisor::Supervisor;

pub const DEFAULT_CURSOR: &str = "combat_log";

/// A read's arguments.
#[derive(Debug, Clone, Default)]
pub struct CombatLogRequest {
    pub since_seq: Option<u64>,
    pub cursor: Option<String>,
    /// Do not move the cursor.
    pub peek: bool,
    pub max: Option<usize>,
}

/// Totals over a batch: hits by hit type, and summed stat deltas by
/// (direction, stat), where direction is `dealt` (player is the source),
/// `taken` (player is the target) or `other`.
pub fn summarize(events: &[&StoredEvent]) -> Value {
    let mut by_hit: Map<String, Value> = Map::new();
    let mut deltas: Map<String, Value> = Map::new();
    let mut mortal = 0u64;
    for e in events {
        let f = &e.fields;
        let hit = f["hit_name"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map_or_else(|| format!("type_{}", f["hit_type"]), str::to_string);
        let n = by_hit.get(&hit).and_then(Value::as_u64).unwrap_or(0);
        by_hit.insert(hit, json!(n + 1));
        if f["mortal"] == json!(true) {
            mortal += 1;
        }
        let dir = if f["source_is_player"] == json!(true) {
            "dealt"
        } else if f["target_is_player"] == json!(true) {
            "taken"
        } else {
            "other"
        };
        if let Some(stats) = f["stats"].as_array() {
            for s in stats {
                let (Some(v), name) = (s["value"].as_f64(), s["stat"].as_str().unwrap_or(""))
                else {
                    continue;
                };
                let key = format!(
                    "{dir}.{}",
                    if name.is_empty() {
                        format!("stat_{}", s["stat_id"])
                    } else {
                        name.to_string()
                    }
                );
                let cur = deltas.get(&key).and_then(Value::as_f64).unwrap_or(0.0);
                deltas.insert(key, json!(cur + v));
            }
        }
    }
    json!({ "records": events.len(), "by_hit_type": by_hit, "stat_totals": deltas, "mortal": mortal })
}

impl Supervisor {
    pub async fn combat_log(&self, req: CombatLogRequest) -> Result<Value, String> {
        let t0 = std::time::Instant::now();
        let pump = self.pump_events(true).await?;
        let cursor = req.cursor.unwrap_or_else(|| DEFAULT_CURSOR.to_string());
        let max = req.max.unwrap_or(200).clamp(1, 2000);
        let mut st = self.events.inner.lock().await;
        let after = req.since_seq.or_else(|| st.cursor(&cursor)).unwrap_or(0);
        let slice = st.since(after, STORE_CAP);
        let combat: Vec<&StoredEvent> = slice
            .events
            .iter()
            .filter(|e| e.kind == KIND_COMBAT)
            .take(max)
            .collect();
        let truncated = slice
            .events
            .iter()
            .filter(|e| e.kind == KIND_COMBAT)
            .count()
            > combat.len();
        // Resume point: the last returned record when the batch was cut
        // short, else everything scanned (non-combat events included).
        let next = if truncated {
            combat.last().map_or(after, |e| e.seq)
        } else {
            slice.events.last().map_or(after, |e| e.seq).max(after)
        };
        let summary = summarize(&combat);
        let events: Vec<Value> = combat.iter().map(|e| e.to_json()).collect();
        if !req.peek {
            st.set_cursor(&cursor, next);
        }
        let installed = pump
            .lua_install
            .iter()
            .find(|(r, _)| r == "combat")
            .map(|(_, s)| s.clone());
        Ok(json!({
            "events": events,
            "summary": summary,
            "truncated": truncated,
            "gap": slice.gap,
            "cursor": { "name": cursor, "from": after, "next_since_seq": next, "moved": !req.peek },
            "capture": {
                "status": installed,
                "lua_error": pump.lua_error,
                "lua_epoch_changed": pump.lua_epoch_changed,
            },
            "native_level": super::NativeLevel::UiLuaRead.to_json(),
            "elapsed_ms": t0.elapsed().as_millis() as u64,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(seq: u64, fields: Value) -> StoredEvent {
        StoredEvent {
            seq,
            kind: KIND_COMBAT.into(),
            ts_ms: 0,
            fields,
        }
    }

    #[test]
    fn summary_splits_dealt_and_taken() {
        let a = hit(
            1,
            json!({ "hit_name": "Hit", "source_is_player": true, "target_is_player": false,
                    "mortal": false, "stats": [{ "stat": "Health", "value": -40 }] }),
        );
        let b = hit(
            2,
            json!({ "hit_name": "Critical", "source_is_player": true, "target_is_player": false,
                    "mortal": true, "stats": [{ "stat": "Health", "value": -90 }] }),
        );
        let c = hit(
            3,
            json!({ "hit_name": "Hit", "source_is_player": false, "target_is_player": true,
                    "mortal": false, "stats": [{ "stat": "Health", "value": -12 },
                                               { "stat": "", "stat_id": 7, "value": -1 }] }),
        );
        let s = summarize(&[&a, &b, &c]);
        assert_eq!(s["records"], 3);
        assert_eq!(s["by_hit_type"]["Hit"], 2);
        assert_eq!(s["by_hit_type"]["Critical"], 1);
        assert_eq!(s["stat_totals"]["dealt.Health"], -130.0);
        assert_eq!(s["stat_totals"]["taken.Health"], -12.0);
        assert_eq!(s["stat_totals"]["taken.stat_7"], -1.0);
        assert_eq!(s["mortal"], 1);
    }

    #[test]
    fn an_unnamed_hit_type_is_keyed_by_number() {
        let e = hit(1, json!({ "hit_name": "", "hit_type": 9, "stats": [] }));
        assert_eq!(summarize(&[&e])["by_hit_type"]["type_9"], 1);
    }
}
