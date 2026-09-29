//! What happened after the press, read from the events that followed it.
//!
//! The signals, all from the event store (bridge ring + the lab's Lua
//! rings):
//!
//! | Signal | Event |
//! |---|---|
//! | the client sent the use | `net.out` with method `useAbility*` |
//! | a cooldown/warmup timer | `cme.event` `*onTimerUpdate` |
//! | the cast started (animation sequence) | `cme.event` `*onSequence` |
//! | an effect landed | `cme.event` `*onEffectResults`, or a `combat.hit` |
//! | the server refused | `cme.event` `*onErrorCode` |
//! | refusal / feedback text | `chat.line` (feedback and system channels) |
//!
//! `cme.event` carries names, not payloads (no ability id on the wire
//! events yet), so a timer or effect from another source in the same window
//! is counted too; the combat records do name the ability.

use serde_json::{json, Value};

use crate::supervisor::events::predicate::glob;
use crate::supervisor::events::store::StoredEvent;
use crate::supervisor::events::{KIND_CHAT, KIND_COMBAT};

/// The observed outcome.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Outcome {
    /// `(seq, method)` of the outgoing call(s).
    pub sent: Vec<(u64, String)>,
    pub timer_updates: u32,
    pub sequences: u32,
    pub effect_results: u32,
    pub error_codes: u32,
    pub combat: Vec<Value>,
    pub feedback: Vec<String>,
    /// Ms from the press to the first sent / cast / effect / error, from
    /// the events' client timestamps.
    pub first_ms: FirstMs,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FirstMs {
    pub sent: Option<i64>,
    pub timer: Option<i64>,
    pub cast: Option<i64>,
    pub effect: Option<i64>,
    pub error: Option<i64>,
}

fn name_of(e: &StoredEvent) -> String {
    ["event", "method", "name"]
        .iter()
        .find_map(|k| e.fields.get(*k).and_then(Value::as_str))
        .unwrap_or_default()
        .to_string()
}

fn first(slot: &mut Option<i64>, ms: i64) {
    if slot.is_none() {
        *slot = Some(ms.max(0));
    }
}

/// Fold the events after the press into an [`Outcome`]. `ability_id`
/// narrows the combat records to this ability when they name one.
pub fn classify(events: &[StoredEvent], press_ms: i64, ability_id: i64) -> Outcome {
    let mut o = Outcome::default();
    for e in events {
        let dt = e.ts_ms - press_ms;
        let name = name_of(e);
        match e.kind.as_str() {
            "net.out" if glob("useAbility*", &name) => {
                o.sent.push((e.seq, name));
                first(&mut o.first_ms.sent, dt);
            }
            "cme.event" if glob("*onTimerUpdate", &name) => {
                o.timer_updates += 1;
                first(&mut o.first_ms.timer, dt);
            }
            "cme.event" if glob("*onSequence", &name) => {
                o.sequences += 1;
                first(&mut o.first_ms.cast, dt);
            }
            "cme.event" if glob("*onEffectResults", &name) => {
                o.effect_results += 1;
                first(&mut o.first_ms.effect, dt);
            }
            "cme.event" if glob("*onErrorCode", &name) => {
                o.error_codes += 1;
                first(&mut o.first_ms.error, dt);
            }
            k if k == KIND_COMBAT => {
                let id = e.fields["ability_id"].as_i64();
                if id.is_none() || id == Some(ability_id) {
                    o.combat.push(e.fields.clone());
                    first(&mut o.first_ms.effect, dt);
                }
            }
            k if k == KIND_CHAT => {
                if let Some(t) = e.fields["text"].as_str() {
                    // Player chat is not feedback; everything the client
                    // prints on its own channels is.
                    if e.fields["speaker"].as_str().unwrap_or("").is_empty() {
                        o.feedback.push(t.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    o
}

impl Outcome {
    /// Something final came back: an effect, a refusal, or feedback text.
    pub fn settled(&self) -> bool {
        self.effect_results > 0 || !self.combat.is_empty() || self.error_codes > 0
    }

    /// One word for the result.
    pub fn verdict(&self) -> &'static str {
        if self.effect_results > 0 || !self.combat.is_empty() {
            "effect_applied"
        } else if self.error_codes > 0 {
            "refused"
        } else if self.sequences > 0 {
            "cast_started"
        } else if !self.sent.is_empty() && !self.feedback.is_empty() {
            "refused_with_feedback"
        } else if !self.sent.is_empty() {
            "sent_no_reply"
        } else if !self.feedback.is_empty() {
            "refused_client_side"
        } else {
            "nothing_observed"
        }
    }

    pub fn to_json(&self) -> Value {
        json!({
            "verdict": self.verdict(),
            "sent": !self.sent.is_empty(),
            "net_out": self.sent.iter().map(|(s, m)| json!({ "seq": s, "method": m })).collect::<Vec<_>>(),
            "cast_started": self.sequences > 0,
            "effect_applied": self.effect_results > 0 || !self.combat.is_empty(),
            "timer_updates": self.timer_updates,
            "sequences": self.sequences,
            "effect_results": self.effect_results,
            "error_codes": self.error_codes,
            "feedback": self.feedback,
            "combat": self.combat,
            "first_ms": {
                "sent": self.first_ms.sent,
                "timer": self.first_ms.timer,
                "cast": self.first_ms.cast,
                "effect": self.first_ms.effect,
                "error": self.first_ms.error,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(seq: u64, kind: &str, ts: i64, fields: Value) -> StoredEvent {
        StoredEvent {
            seq,
            kind: kind.into(),
            ts_ms: ts,
            fields,
        }
    }

    #[test]
    fn a_landed_shot_reads_as_effect_applied_with_timings() {
        let evs = vec![
            ev(1, "net.out", 1010, json!({ "method": "useAbility" })),
            ev(
                2,
                "cme.event",
                1100,
                json!({ "event": "Event_NetIn_onTimerUpdate" }),
            ),
            ev(
                3,
                "cme.event",
                1120,
                json!({ "event": "Event_NetIn_onSequence" }),
            ),
            ev(
                4,
                "cme.event",
                1400,
                json!({ "event": "Event_NetIn_onEffectResults" }),
            ),
            ev(
                5,
                "combat.hit",
                1450,
                json!({ "ability_id": 1100, "hit_name": "Hit" }),
            ),
            ev(
                6,
                "combat.hit",
                1460,
                json!({ "ability_id": 999, "hit_name": "Hit" }),
            ),
        ];
        let o = classify(&evs, 1000, 1100);
        assert_eq!(o.verdict(), "effect_applied");
        assert!(o.settled());
        assert_eq!(o.sent, vec![(1, "useAbility".into())]);
        assert_eq!(o.first_ms.sent, Some(10));
        assert_eq!(o.first_ms.cast, Some(120));
        assert_eq!(o.first_ms.effect, Some(400));
        assert_eq!(o.combat.len(), 1, "another ability's hit is not ours");
    }

    #[test]
    fn an_error_code_is_a_refusal() {
        let evs = vec![
            ev(1, "net.out", 1, json!({ "method": "useAbility" })),
            ev(
                2,
                "cme.event",
                2,
                json!({ "event": "Event_NetIn_onErrorCode" }),
            ),
            ev(
                3,
                "chat.line",
                3,
                json!({ "text": "Target is out of range", "speaker": "" }),
            ),
        ];
        let o = classify(&evs, 0, 1);
        assert_eq!(o.verdict(), "refused");
        assert_eq!(o.feedback, vec!["Target is out of range".to_string()]);
    }

    #[test]
    fn client_side_refusals_send_nothing() {
        let evs = vec![ev(
            1,
            "chat.line",
            3,
            json!({ "text": "You must have a target", "speaker": "" }),
        )];
        assert_eq!(classify(&evs, 0, 1).verdict(), "refused_client_side");
        assert_eq!(classify(&[], 0, 1).verdict(), "nothing_observed");
    }

    #[test]
    fn a_sent_use_with_no_reply_and_player_chat_is_not_feedback() {
        let evs = vec![
            ev(
                1,
                "net.out",
                1,
                json!({ "method": "useAbilityOnGroundTarget" }),
            ),
            ev(
                2,
                "chat.line",
                2,
                json!({ "text": "lol", "speaker": "Bob" }),
            ),
            ev(3, "net.out", 3, json!({ "method": "setTarget" })),
        ];
        let o = classify(&evs, 0, 1);
        assert_eq!(o.verdict(), "sent_no_reply");
        assert!(o.feedback.is_empty());
        assert_eq!(o.sent.len(), 1);
    }

    #[test]
    fn a_cast_without_an_effect_yet() {
        let evs = vec![
            ev(1, "net.out", 1, json!({ "method": "useAbility" })),
            ev(
                2,
                "cme.event",
                2,
                json!({ "event": "Event_NetIn_onSequence" }),
            ),
        ];
        let o = classify(&evs, 0, 1);
        assert_eq!(o.verdict(), "cast_started");
        assert!(!o.settled());
    }
}
