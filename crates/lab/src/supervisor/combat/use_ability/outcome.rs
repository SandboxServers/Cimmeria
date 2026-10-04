//! What happened after the press, read from the events that followed it.
//!
//! The signals, all from the event store (bridge ring + the lab's Lua
//! rings). Two generations of client telemetry feed it: the older
//! `cme.event` / `net.out` names, and the `ability.*` rows of the ability
//! trace (`docs/architecture/client-telemetry.md` § Ability telemetry),
//! mirrored to the lab ring with the `client.` prefix dropped.
//!
//! | Signal | Events |
//! |---|---|
//! | the client sent the use | `net.out` with method `useAbility*`; `ability.sent` |
//! | the client dropped the press | `ability.press_dropped` (with its `reason`) |
//! | a cooldown/warmup timer | `cme.event` `*onTimerUpdate`; `ability.recv` `onTimerUpdate`; `ability.applied` `kind = cooldown` |
//! | the cast started (animation sequence) | `cme.event` `*onSequence`; `ability.recv` `onSequence` |
//! | an effect landed | `cme.event` `*onEffectResults`; `ability.recv` `onEffectResults`; `ability.applied` `kind = stat`, `stat_base`, `effect_bar_add`, `effect_bar_refresh`; `ability.shown` combat text; a `combat.hit` |
//! | the server refused | `cme.event` `*onErrorCode`; `ability.recv` `onErrorCode` |
//! | refusal / feedback text | `chat.line` (no speaker); `ability.shown` `feedback_line` |
//!
//! One server message can show up in several of these (the `cme.event`
//! name, its `ability.recv` decode, its `ability.applied` handler row), so
//! each count is the largest any one source saw, not their sum.
//!
//! `cme.event` and the timer and stat rows carry no ability id, so a timer
//! or a stat change from another source in the same window (regeneration,
//! a DoT) is counted too; the combat records, the effect results, the
//! combat text and a dropped press do name the ability, and another
//! ability's are skipped.

use serde_json::{json, Value};

use crate::supervisor::events::predicate::glob;
use crate::supervisor::events::store::StoredEvent;
use crate::supervisor::events::{KIND_CHAT, KIND_COMBAT};

/// The observed outcome.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Outcome {
    /// `(seq, method)` of the outgoing call(s), from `net.out`.
    pub sent: Vec<(u64, String)>,
    /// `ability.sent` rows (the ability trace's view of the same send).
    pub ability_sent: u32,
    pub timer_updates: u32,
    pub sequences: u32,
    pub effect_results: u32,
    pub error_codes: u32,
    /// `ability.applied` stat / stat_base rows: the client applied a stat
    /// change (a heal, a drain).
    pub stat_applies: u32,
    /// `ability.applied` effect-bar adds and refreshes: a buff or debuff.
    pub effect_bar: u32,
    /// `ability.shown` combat text / combat chat lines for this ability.
    pub combat_text: u32,
    /// `ability.press_dropped` reasons for this ability.
    pub press_dropped: Vec<String>,
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

/// The row names this ability, or names none (0 and null are "none").
fn for_ability(fields: &Value, ability_id: i64) -> bool {
    match fields["ability_id"].as_i64() {
        None | Some(0) => true,
        Some(id) => id == ability_id,
    }
}

/// One count per source, so the same message seen three ways counts once.
#[derive(Debug, Default)]
struct Tally {
    cme: u32,
    recv: u32,
    applied: u32,
}

impl Tally {
    /// A `cme.event` name, or an `ability.recv` decode.
    fn bump(&mut self, cme: bool) {
        if cme {
            self.cme += 1;
        } else {
            self.recv += 1;
        }
    }

    fn best(&self) -> u32 {
        self.cme.max(self.recv).max(self.applied)
    }
}

/// Record a feedback line once: the chat ring and the `ability.shown` row
/// both see the same line.
fn feedback(o: &mut Outcome, t: &str) {
    if !o.feedback.iter().any(|f| f == t) {
        o.feedback.push(t.to_string());
    }
}

/// Fold the events after the press into an [`Outcome`]. `ability_id`
/// narrows the rows that name an ability to this one.
pub fn classify(events: &[StoredEvent], press_ms: i64, ability_id: i64) -> Outcome {
    let mut o = Outcome::default();
    let (mut timers, mut seqs, mut effects, mut errors) = (
        Tally::default(),
        Tally::default(),
        Tally::default(),
        Tally::default(),
    );
    for e in events {
        let dt = e.ts_ms - press_ms;
        let name = name_of(e);
        let sub = e.fields["kind"].as_str().unwrap_or_default();
        match e.kind.as_str() {
            "net.out" if glob("useAbility*", &name) => {
                o.sent.push((e.seq, name));
                first(&mut o.first_ms.sent, dt);
            }
            "ability.sent" if glob("useAbility*", &name) => {
                o.ability_sent += 1;
                first(&mut o.first_ms.sent, dt);
            }
            "ability.press_dropped" if for_ability(&e.fields, ability_id) => {
                let reason = e.fields["reason"].as_str().unwrap_or("unknown");
                o.press_dropped.push(reason.to_string());
                first(&mut o.first_ms.error, dt);
            }
            "cme.event" | "ability.recv" => {
                let cme = e.kind == "cme.event";
                if glob("*onTimerUpdate", &name) {
                    timers.bump(cme);
                    first(&mut o.first_ms.timer, dt);
                } else if glob("*onSequence", &name) {
                    seqs.bump(cme);
                    first(&mut o.first_ms.cast, dt);
                } else if glob("*onEffectResults", &name) && for_ability(&e.fields, ability_id) {
                    effects.bump(cme);
                    first(&mut o.first_ms.effect, dt);
                } else if glob("*onErrorCode", &name) {
                    errors.bump(cme);
                    first(&mut o.first_ms.error, dt);
                }
            }
            "ability.applied" => match sub {
                // `other_source` is another being's timer on our manager.
                "cooldown" if e.fields["outcome"].as_str() != Some("other_source") => {
                    timers.applied += 1;
                    first(&mut o.first_ms.timer, dt);
                }
                "stat" | "stat_base" => {
                    o.stat_applies += 1;
                    first(&mut o.first_ms.effect, dt);
                }
                "effect_bar_add" | "effect_bar_refresh" => {
                    o.effect_bar += 1;
                    first(&mut o.first_ms.effect, dt);
                }
                _ => {}
            },
            "ability.shown" => match sub {
                "combat_text" | "combat_chat_line" if for_ability(&e.fields, ability_id) => {
                    o.combat_text += 1;
                    first(&mut o.first_ms.effect, dt);
                }
                "feedback_line" => {
                    if let Some(t) = e.fields["text"].as_str() {
                        feedback(&mut o, t);
                    }
                }
                _ => {}
            },
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
                        feedback(&mut o, t);
                    }
                }
            }
            _ => {}
        }
    }
    o.timer_updates = timers.best();
    o.sequences = seqs.best();
    o.effect_results = effects.best();
    o.error_codes = errors.best();
    o
}

impl Outcome {
    /// The client sent the use (either telemetry generation saw it).
    pub fn was_sent(&self) -> bool {
        !self.sent.is_empty() || self.ability_sent > 0
    }

    /// Something in the window says the ability did something.
    pub fn effect_applied(&self) -> bool {
        self.effect_results > 0
            || !self.combat.is_empty()
            || self.stat_applies > 0
            || self.effect_bar > 0
            || self.combat_text > 0
    }

    /// Something final came back: an effect, a refusal, or a dropped press.
    pub fn settled(&self) -> bool {
        self.effect_applied() || self.error_codes > 0 || !self.press_dropped.is_empty()
    }

    /// One word for the result.
    pub fn verdict(&self) -> &'static str {
        if self.effect_applied() {
            "effect_applied"
        } else if self.error_codes > 0 {
            "refused"
        } else if !self.press_dropped.is_empty() && !self.was_sent() {
            "refused_client_side"
        } else if self.sequences > 0 || self.timer_updates > 0 {
            // A cooldown or warmup timer means the server accepted the
            // cast; nothing has landed yet.
            "cast_started"
        } else if self.was_sent() && !self.feedback.is_empty() {
            "refused_with_feedback"
        } else if self.was_sent() {
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
            "sent": self.was_sent(),
            "net_out": self.sent.iter().map(|(s, m)| json!({ "seq": s, "method": m })).collect::<Vec<_>>(),
            "ability_sent": self.ability_sent,
            "press_dropped": self.press_dropped,
            "cast_started": self.sequences > 0,
            "effect_applied": self.effect_applied(),
            "timer_updates": self.timer_updates,
            "sequences": self.sequences,
            "effect_results": self.effect_results,
            "stat_applies": self.stat_applies,
            "effect_bar": self.effect_bar,
            "combat_text": self.combat_text,
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
#[path = "outcome_tests.rs"]
mod tests;
