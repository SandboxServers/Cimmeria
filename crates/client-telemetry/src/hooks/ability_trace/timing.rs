//! Client-side timing of a cast (AB-C6 of the ability-mechanics telemetry
//! plan): press to send, send to the first matching receive, and receive to
//! applied, all on the client's own clock ([`super::now_ms`], the process's
//! monotonic milliseconds).
//!
//! | Interval | Field, on | Join |
//! |---|---|---|
//! | press to send | `press_to_sent_ms` on `client.ability.sent` | the press the router claimed (`press_id`, `press::pending`) |
//! | send to first receive | `sent_to_recv_ms`, `send_id`, `press_id`, `sent_method` on `client.ability.recv` | the oldest unanswered send of the same `ability_id` (see [`recv_ability_id`]) |
//! | receive to applied | `recv_to_applied_ms` on `client.ability.applied` | the latest receive of the method that feeds the handler, for the same entity (and timer id) |
//!
//! The `cast_id` is the server's `effect_seq`, which reaches the client
//! only as `onEffectResults.EffectID`; the receive row already carries it
//! as `effect_id`, and the send row joins it through `send_id`. A press
//! the client dropped has no send, so it has no timing (the plan's
//! fallback join is server side).
//!
//! Every interval is also recorded, before the per-name throttle, in the
//! uploader's histograms (`governor::ability_timing`), labelled by method or
//! applied kind only.
//!
//! **Bounds.** At most [`MAX_SENDS`] open sends, each answered once and
//! forgotten after [`SEND_TTL_MS`] (long enough for a warmup); at most
//! [`MAX_RECVS`] receive keys, each matched for [`APPLY_TTL_MS`].

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use serde_json::{json, Value};

use crate::governor::ability_timing;
use crate::hooks::entity_trace::Fields;

/// How long a send waits for its first response.
pub(crate) const SEND_TTL_MS: u64 = 30_000;
/// Open sends kept at most.
pub(crate) const MAX_SENDS: usize = 32;
/// How long a receive stays joinable to what it applied.
pub(crate) const APPLY_TTL_MS: u64 = 5_000;
/// Receive keys kept at most.
pub(crate) const MAX_RECVS: usize = 256;

/// `onTimerUpdate.Type` values whose `ID` is an ability id.
const TIMER_ABILITY_WARMUP: i64 = 1;
const TIMER_ABILITY_COOLDOWN: i64 = 2;
/// `onErrorCode.SystemID` of the ability system (`ERRORCODE_SYSTEM_ABILITY`).
const ERROR_SYSTEM_ABILITY: i64 = 0;

/// The ability a received method answers, from its decoded arguments:
/// `onEffectResults.AbilityID`, the `ID` of an ability warmup or cooldown
/// `onTimerUpdate`, and the `InstanceID` of an ability-system
/// `onErrorCode`. Every other method answers no send.
pub(crate) fn recv_ability_id(method: &str, arg: impl Fn(&str) -> Option<i64>) -> Option<i32> {
    let id = match method {
        "onEffectResults" => arg("ability_id"),
        "onTimerUpdate" => match arg("timer_type") {
            Some(TIMER_ABILITY_WARMUP | TIMER_ABILITY_COOLDOWN) => arg("timer_id"),
            _ => None,
        },
        "onErrorCode" if arg("system_id") == Some(ERROR_SYSTEM_ABILITY) => arg("instance_id"),
        _ => None,
    }?;
    i32::try_from(id).ok().filter(|&id| id != 0)
}

/// The received method that feeds an applied `kind`
/// (`client.ability.applied`'s `kind` field).
pub(crate) fn source_method(kind: &str) -> Option<&'static str> {
    Some(match kind {
        k if k.starts_with("effect_bar") || k == "cooldown" => "onTimerUpdate",
        "stat" => "onStatUpdate",
        "stat_base" => "onStatBaseUpdate",
        "state_flag" => "onStateFieldUpdate",
        _ => return None,
    })
}

/// A send that has not been answered yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OpenSend {
    send_id: u32,
    press_id: Option<u32>,
    method: &'static str,
    ability_id: i32,
    at_ms: u64,
}

/// The send a receive answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Answered {
    /// The send's id (`client.ability.sent`).
    pub send_id: u32,
    /// The press behind it, when one matched.
    pub press_id: Option<u32>,
    /// The method that was sent.
    pub sent_method: &'static str,
    /// Milliseconds from the send to this receive.
    pub sent_to_recv_ms: u64,
}

/// Which receive an applied row joins: the method, the entity it was
/// called on, and the timer id for `onTimerUpdate`.
pub(crate) type RecvKey = (&'static str, Option<i32>, Option<i32>);

/// The join tables.
#[derive(Debug, Default)]
pub(crate) struct Timing {
    sends: VecDeque<OpenSend>,
    recvs: HashMap<RecvKey, u64>,
}

impl Timing {
    /// A press reached the wire `press_to_sent_ms` after it was posted.
    pub(crate) fn press_sent(method: &'static str, press_to_sent_ms: u64) {
        ability_timing::observe("press_to_sent", method, press_to_sent_ms);
    }

    /// Remember a send that names an ability, until its first answer.
    pub(crate) fn note_sent(
        &mut self,
        send_id: u32,
        press_id: Option<u32>,
        method: &'static str,
        ability_id: Option<i32>,
        now_ms: u64,
    ) {
        let Some(ability_id) = ability_id.filter(|&a| a != 0) else {
            return;
        };
        self.expire(now_ms);
        if self.sends.len() >= MAX_SENDS {
            self.sends.pop_front();
        }
        self.sends.push_back(OpenSend {
            send_id,
            press_id,
            method,
            ability_id,
            at_ms: now_ms,
        });
    }

    fn expire(&mut self, now_ms: u64) {
        self.sends
            .retain(|s| now_ms.saturating_sub(s.at_ms) <= SEND_TTL_MS);
    }

    /// A receive of `method` arrived for `entity_id`. Records it for the
    /// applied join under `timer_id` (for `onTimerUpdate`), and, when it
    /// names `ability_id`, answers the oldest open send of that ability.
    pub(crate) fn on_recv(
        &mut self,
        method: &'static str,
        entity_id: Option<i32>,
        timer_id: Option<i32>,
        ability_id: Option<i32>,
        now_ms: u64,
    ) -> Option<Answered> {
        let key = (method, entity_id, timer_id);
        if !self.recvs.contains_key(&key) && self.recvs.len() >= MAX_RECVS {
            self.recvs
                .retain(|_, &mut at| now_ms.saturating_sub(at) <= APPLY_TTL_MS);
            if self.recvs.len() >= MAX_RECVS {
                self.recvs.clear();
            }
        }
        self.recvs.insert(key, now_ms);

        let ability_id = ability_id?;
        self.expire(now_ms);
        let i = self.sends.iter().position(|s| s.ability_id == ability_id)?;
        let s = self.sends.remove(i)?;
        let a = Answered {
            send_id: s.send_id,
            press_id: s.press_id,
            sent_method: s.method,
            sent_to_recv_ms: now_ms.saturating_sub(s.at_ms),
        };
        ability_timing::observe("sent_to_recv", method, a.sent_to_recv_ms);
        Some(a)
    }

    /// An applied row of `kind` for `entity_id` (and `timer_id`): the
    /// milliseconds since the receive that fed it, when one is recent.
    pub(crate) fn on_applied(
        &self,
        kind: &str,
        entity_id: Option<i32>,
        timer_id: Option<i32>,
        now_ms: u64,
    ) -> Option<u64> {
        let method = source_method(kind)?;
        let timer_id = if method == "onTimerUpdate" {
            timer_id
        } else {
            None
        };
        let at = *self.recvs.get(&(method, entity_id, timer_id))?;
        let ms = now_ms.checked_sub(at)?;
        if ms > APPLY_TTL_MS {
            return None;
        }
        ability_timing::observe("recv_to_applied", applied_label(kind)?, ms);
        Some(ms)
    }
}

static TIMING: Mutex<Option<Timing>> = Mutex::new(None);

/// Run `f` on the process's join tables (the send, receive and applied
/// hooks share them). Poisoning is ignored.
pub(crate) fn with_timing<R>(f: impl FnOnce(&mut Timing) -> R) -> R {
    let mut g = TIMING.lock().unwrap_or_else(|e| e.into_inner());
    f(g.get_or_insert_with(Timing::default))
}

/// The histogram label of an applied kind: the effect-bar kinds share one.
fn applied_label(kind: &str) -> Option<&'static str> {
    Some(match kind {
        k if k.starts_with("effect_bar") => "effect_bar",
        "cooldown" => "cooldown",
        "stat" => "stat",
        "stat_base" => "stat_base",
        "state_flag" => "state_flag",
        _ => return None,
    })
}

fn int(fields: &Fields, key: &str) -> Option<i64> {
    fields
        .iter()
        .find(|(k, _)| *k == key)
        .and_then(|(_, v)| v.as_i64())
}

fn as_i32(v: Option<i64>) -> Option<i32> {
    v.and_then(|v| i32::try_from(v).ok())
}

/// Join a `client.ability.recv` row (built, not yet throttled) of
/// `method`: records the receive, and when it answers a send adds
/// `send_id`, `press_id`, `sent_method` and `sent_to_recv_ms`.
pub(crate) fn annotate_recv(fields: &mut Fields, method: &'static str, now_ms: u64) {
    let entity_id = as_i32(int(fields, "entity_id"));
    let timer_id = if method == "onTimerUpdate" {
        as_i32(int(fields, "timer_id"))
    } else {
        None
    };
    let ability_id = recv_ability_id(method, |k| int(fields, k));
    let answered = with_timing(|t| t.on_recv(method, entity_id, timer_id, ability_id, now_ms));
    if let Some(a) = answered {
        fields.push(("send_id", json!(a.send_id)));
        fields.push(("press_id", a.press_id.map_or(Value::Null, |p| json!(p))));
        fields.push(("sent_method", json!(a.sent_method)));
        fields.push(("sent_to_recv_ms", json!(a.sent_to_recv_ms)));
    }
}

/// Join a `client.ability.applied` row (built, not yet throttled): adds
/// `recv_to_applied_ms` when the receive that fed it is recent.
pub(crate) fn annotate_applied(fields: &mut Fields, now_ms: u64) {
    let Some(kind) = fields
        .iter()
        .find(|(k, _)| *k == "kind")
        .and_then(|(_, v)| v.as_str())
    else {
        return;
    };
    let entity_id = as_i32(int(fields, "entity_id"));
    let timer_id = as_i32(int(fields, "timer_id"));
    let ms = with_timing(|t| t.on_applied(kind, entity_id, timer_id, now_ms));
    if let Some(ms) = ms {
        fields.push(("recv_to_applied_ms", json!(ms)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args<'a>(pairs: &'a [(&'a str, i64)]) -> impl Fn(&str) -> Option<i64> + 'a {
        move |k| pairs.iter().find(|(n, _)| *n == k).map(|(_, v)| *v)
    }

    #[test]
    fn only_ability_bearing_receives_name_an_ability() {
        assert_eq!(
            recv_ability_id("onEffectResults", args(&[("ability_id", 597)])),
            Some(597)
        );
        assert_eq!(
            recv_ability_id(
                "onTimerUpdate",
                args(&[("timer_type", 2), ("timer_id", 597)])
            ),
            Some(597)
        );
        // An effect duration timer's ID is the effect, not the ability.
        assert_eq!(
            recv_ability_id(
                "onTimerUpdate",
                args(&[("timer_type", 5), ("timer_id", 77)])
            ),
            None
        );
        assert_eq!(
            recv_ability_id(
                "onErrorCode",
                args(&[("system_id", 0), ("instance_id", 597)])
            ),
            Some(597)
        );
        assert_eq!(
            recv_ability_id(
                "onErrorCode",
                args(&[("system_id", 3), ("instance_id", 597)])
            ),
            None
        );
        assert_eq!(
            recv_ability_id("onSequence", args(&[("ability_id", 597)])),
            None
        );
        assert_eq!(
            recv_ability_id("onEffectResults", args(&[("ability_id", 0)])),
            None
        );
    }

    /// A press, its send, a warmup timer 80 ms later, the effect results
    /// 1.2 s later, and the effect bar applied 15 ms after its timer.
    #[test]
    fn a_cast_is_timed_from_send_to_first_answer_to_applied() {
        let mut t = Timing::default();
        t.note_sent(1, Some(10), "useAbility", Some(597), 1_000);
        // Another ability's answer does not claim this send.
        assert_eq!(
            t.on_recv("onEffectResults", Some(5), None, Some(42), 1_050),
            None
        );
        let first = t
            .on_recv("onTimerUpdate", Some(5), Some(597), Some(597), 1_080)
            .unwrap();
        assert_eq!(
            first,
            Answered {
                send_id: 1,
                press_id: Some(10),
                sent_method: "useAbility",
                sent_to_recv_ms: 80
            }
        );
        // The send is answered once: the later results carry no join.
        assert_eq!(
            t.on_recv("onEffectResults", Some(5), None, Some(597), 2_200),
            None
        );
        assert_eq!(
            t.on_applied("cooldown", Some(5), Some(597), 1_095),
            Some(15)
        );
        assert_eq!(t.on_applied("cooldown", Some(5), Some(598), 1_095), None);
        assert_eq!(t.on_applied("cooldown", Some(6), Some(597), 1_095), None);
        // Too long after its receive: no join.
        assert_eq!(t.on_applied("cooldown", Some(5), Some(597), 7_000), None);
        assert_eq!(t.on_applied("unknown_kind", Some(5), None, 1_095), None);
    }

    #[test]
    fn stat_and_state_flag_join_their_own_methods_by_entity() {
        let mut t = Timing::default();
        t.on_recv("onStatUpdate", Some(5), None, None, 100);
        t.on_recv("onStateFieldUpdate", Some(9), None, None, 110);
        // The timer id of a non-timer kind is ignored.
        assert_eq!(t.on_applied("stat", Some(5), Some(3), 104), Some(4));
        assert_eq!(t.on_applied("state_flag", Some(9), None, 111), Some(1));
        assert_eq!(t.on_applied("stat_base", Some(5), None, 111), None);
        assert_eq!(t.on_applied("effect_bar_add", Some(5), Some(1), 111), None);
    }

    #[test]
    fn two_sends_of_one_ability_are_answered_in_order_and_expire() {
        let mut t = Timing::default();
        t.note_sent(1, None, "useAbility", Some(7), 0);
        t.note_sent(2, None, "useAbility", Some(7), 10);
        assert_eq!(
            t.on_recv("onEffectResults", None, None, Some(7), 20)
                .unwrap()
                .send_id,
            1
        );
        assert_eq!(
            t.on_recv("onEffectResults", None, None, Some(7), 30)
                .unwrap()
                .send_id,
            2
        );
        t.note_sent(3, None, "useAbility", Some(7), 100);
        assert_eq!(
            t.on_recv(
                "onEffectResults",
                None,
                None,
                Some(7),
                100 + SEND_TTL_MS + 1
            ),
            None
        );
        // A send with no ability (resetMyAbilities) is never open.
        t.note_sent(4, None, "resetMyAbilities", None, 0);
        assert!(t.sends.is_empty());
    }

    #[test]
    fn the_tables_are_bounded() {
        let mut t = Timing::default();
        for i in 0..(MAX_SENDS as u32 + 10) {
            t.note_sent(i, None, "useAbility", Some(i as i32 + 1), 0);
        }
        assert_eq!(t.sends.len(), MAX_SENDS);
        assert_eq!(t.sends.front().unwrap().send_id, 10, "the oldest go first");
        for i in 0..(MAX_RECVS as i32 + 10) {
            t.on_recv("onStatUpdate", Some(i), None, None, 0);
        }
        assert!(t.recvs.len() <= MAX_RECVS);
    }

    fn get<'a>(f: &'a Fields, k: &str) -> Option<&'a Value> {
        f.iter().find(|(n, _)| *n == k).map(|(_, v)| v)
    }

    /// The row-level joins, through the process tables: a send, its
    /// warmup timer row, then the cooldown the client applied from it.
    /// Uses ids no other test touches, since the tables are shared.
    #[test]
    fn recv_and_applied_rows_carry_their_intervals() {
        let now = super::super::now_ms();
        with_timing(|t| t.note_sent(901, Some(902), "useAbility", Some(-31_337), now));
        let mut recv: Fields = vec![
            ("method", json!("onTimerUpdate")),
            ("entity_id", json!(-77)),
            ("timer_id", json!(-31_337)),
            ("timer_type", json!(2)),
        ];
        annotate_recv(&mut recv, "onTimerUpdate", now + 40);
        assert_eq!(get(&recv, "send_id"), Some(&json!(901)));
        assert_eq!(get(&recv, "press_id"), Some(&json!(902)));
        assert_eq!(get(&recv, "sent_method"), Some(&json!("useAbility")));
        assert_eq!(get(&recv, "sent_to_recv_ms"), Some(&json!(40)));

        let mut applied: Fields = vec![
            ("kind", json!("cooldown")),
            ("entity_id", json!(-77)),
            ("ability_id", json!(-31_337)),
            ("timer_id", json!(-31_337)),
        ];
        annotate_applied(&mut applied, now + 52);
        assert_eq!(get(&applied, "recv_to_applied_ms"), Some(&json!(12)));

        // A second receive of the same ability answers nothing more.
        let mut again = recv[..4].to_vec();
        annotate_recv(&mut again, "onTimerUpdate", now + 60);
        assert_eq!(get(&again, "send_id"), None);
        // A row with no kind, or a kind with no source, is left alone.
        let mut other: Fields = vec![("kind", json!("mystery")), ("entity_id", json!(-77))];
        annotate_applied(&mut other, now + 61);
        assert_eq!(other.len(), 2);
    }

    /// Every interval lands in the uploader's histogram under its label,
    /// method or kind, never an id.
    #[test]
    fn intervals_feed_the_histograms_with_enumerated_labels() {
        let _ = ability_timing::drain();
        let mut t = Timing::default();
        Timing::press_sent("useAbility", 4);
        t.note_sent(1, None, "useAbility", Some(597), 0);
        t.on_recv("onTimerUpdate", Some(5), Some(77), Some(597), 50);
        t.on_applied("effect_bar_refresh", Some(5), Some(77), 60);
        let mut got: Vec<(String, String, u64)> = ability_timing::drain()
            .into_iter()
            .map(|f| {
                (
                    f["stage"].as_str().unwrap().to_string(),
                    f["label"].as_str().unwrap().to_string(),
                    f["sum_ms"].as_u64().unwrap(),
                )
            })
            .collect();
        got.sort();
        assert_eq!(
            got,
            vec![
                ("press_to_sent".into(), "useAbility".into(), 4),
                ("recv_to_applied".into(), "effect_bar".into(), 10),
                ("sent_to_recv".into(), "onTimerUpdate".into(), 50),
            ]
        );
    }
}
