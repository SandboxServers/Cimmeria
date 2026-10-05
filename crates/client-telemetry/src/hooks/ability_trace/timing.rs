//! Client-side timing of a cast (AB-C6 of the ability-mechanics telemetry
//! plan): press to send, send to the first matching receive, and receive to
//! applied, all on the client's own clock ([`super::now_ms`], the process's
//! monotonic milliseconds).
//!
//! | Interval | Field, on | Join |
//! |---|---|---|
//! | press to send | `press_to_sent_ms` on `client.ability.sent` | the press the router claimed (`press_id`, `press::pending`) |
//! | send to first receive | `sent_to_recv_ms`, `send_id`, `press_id`, `sent_method`, `send_reply = first` on `client.ability.recv` | see "The send join" |
//! | receive to applied | `recv_to_applied_ms` on `client.ability.applied` | the latest receive of the method that feeds the handler, for the same entity (and timer id) |
//!
//! # The send join
//!
//! - **Only answerable sends wait.** A send is held only when the server
//!   answers it with ability replies ([`ANSWERED_METHODS`]: `useAbility`,
//!   `useAbilityOnGroundTarget`, `petInvokeAbility`). `trainAbility`,
//!   `resetMyAbilities`, `confirmationResponse`, `petAbilityToggle` and the
//!   `gmDebug*` methods are answered by other messages or by nothing, so
//!   they never sit where they could take a cast's answer.
//! - **Short expiry.** A held send waits [`PENDING_TTL_MS`] for its first
//!   reply. The server's first reply (the warmup or cooldown timer, the
//!   results of an instant cast, or a refusal) follows the receipt within a
//!   round trip, so a send unanswered that long was refused silently or
//!   lost.
//! - **Local receipts only.** Only a reply addressed to the local player's
//!   own entity can claim or follow a send; a witnessed cast of the same
//!   ability by someone else never does.
//! - **FIFO, one cast per send.** A reply names its ability and its kind
//!   (warmup timer, cooldown timer, effect results, refusal). It belongs to
//!   the oldest answered send of that ability that has not had a reply of
//!   that kind yet; `onEffectResults` carries the server's `cast_id`, and
//!   every reply with a `cast_id` already bound to a send belongs to that
//!   send. Only when no answered send takes it does it claim the oldest
//!   pending send. A refusal (`onErrorCode`) is the one exception: the
//!   server refuses before it launches, so a refusal claims the oldest held
//!   send of its ability when there is one. So with two sends of one ability pending, cast 1's
//!   warmup, cooldown and results all join send 1, and cast 2's first
//!   reply claims send 2. Follow-up replies carry `send_id`, `press_id`,
//!   `sent_method` and `send_reply = follow_up`, without an interval.
//!
//! Every interval is also recorded, before the per-name throttle, in the
//! uploader's histograms (`governor::ability_timing`), labelled by method or
//! applied kind only.
//!
//! **Bounds.** At most [`MAX_SENDS`] held and [`MAX_SENDS`] answered sends
//! (an answered send is forgotten [`CAST_TTL_MS`] after its last reply); at
//! most [`MAX_RECVS`] receive keys, each matched for [`APPLY_TTL_MS`].

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use serde_json::{json, Value};

use crate::governor::ability_timing;
use crate::hooks::entity_trace::Fields;

/// The sends the server answers with ability replies.
pub(crate) const ANSWERED_METHODS: &[&str] =
    &["useAbility", "useAbilityOnGroundTarget", "petInvokeAbility"];
/// How long a send waits for its first reply.
pub(crate) const PENDING_TTL_MS: u64 = 5_000;
/// How long an answered send keeps taking its cast's later replies after
/// the last one (long enough for a warmup and its fire).
pub(crate) const CAST_TTL_MS: u64 = 30_000;
/// Held (and, separately, answered) sends kept at most.
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

/// What a reply to a cast is. A cast has at most one of each, apart from
/// `Results` (one per effect and target, all with the same `cast_id`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReplyKind {
    Warmup,
    Cooldown,
    Results,
    Refusal,
}

impl ReplyKind {
    fn bit(self) -> u8 {
        match self {
            Self::Warmup => 1,
            Self::Cooldown => 2,
            Self::Results => 4,
            Self::Refusal => 8,
        }
    }
}

/// The ability a received method answers, and what kind of reply it is,
/// from its decoded arguments: `onEffectResults.AbilityID`, the `ID` of an
/// ability warmup or cooldown `onTimerUpdate`, and the `InstanceID` of an
/// ability-system `onErrorCode`. Every other method answers no send.
pub(crate) fn recv_reply(
    method: &str,
    arg: impl Fn(&str) -> Option<i64>,
) -> Option<(i32, ReplyKind)> {
    let (id, kind) = match method {
        "onEffectResults" => (arg("ability_id"), ReplyKind::Results),
        "onTimerUpdate" => match arg("timer_type") {
            Some(TIMER_ABILITY_WARMUP) => (arg("timer_id"), ReplyKind::Warmup),
            Some(TIMER_ABILITY_COOLDOWN) => (arg("timer_id"), ReplyKind::Cooldown),
            _ => return None,
        },
        "onErrorCode" if arg("system_id") == Some(ERROR_SYSTEM_ABILITY) => {
            (arg("instance_id"), ReplyKind::Refusal)
        }
        _ => return None,
    };
    let id = i32::try_from(id?).ok().filter(|&id| id != 0)?;
    Some((id, kind))
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

/// A held send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Send {
    send_id: u32,
    press_id: Option<u32>,
    method: &'static str,
    ability_id: i32,
    at_ms: u64,
}

/// A send that has had its first reply and takes the rest of its cast's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Casting {
    send: Send,
    /// [`ReplyKind::bit`]s seen.
    kinds: u8,
    cast_id: Option<i32>,
    last_ms: u64,
}

impl Casting {
    /// Whether a reply of `kind` (with `cast_id`) can be this cast's next.
    fn takes(&self, kind: ReplyKind, cast_id: Option<i32>) -> bool {
        match (kind, cast_id, self.cast_id) {
            // Results are matched by cast id when both have one.
            (_, Some(c), Some(bound)) => c == bound,
            (ReplyKind::Results, _, _) => self.kinds & kind.bit() == 0 || cast_id.is_none(),
            _ => self.kinds & kind.bit() == 0,
        }
    }
}

/// One reply, as the join sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Reply {
    /// The received method.
    pub method: &'static str,
    /// The entity it was called on.
    pub entity_id: Option<i32>,
    /// `onTimerUpdate.ID`, for the applied join.
    pub timer_id: Option<i32>,
    /// The ability and the kind, when it is a reply to a cast.
    pub answers: Option<(i32, ReplyKind)>,
    /// The server's cast id, when the wire carries it.
    pub cast_id: Option<i32>,
    /// Addressed to the local player.
    pub local: bool,
}

/// The send a reply belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Answered {
    /// The send's id (`client.ability.sent`).
    pub send_id: u32,
    /// The press behind it, when one matched.
    pub press_id: Option<u32>,
    /// The method that was sent.
    pub sent_method: &'static str,
    /// Milliseconds from the send to this reply: only on its first reply.
    pub sent_to_recv_ms: Option<u64>,
}

/// Which receive an applied row joins: the method, the entity it was
/// called on, and the timer id for `onTimerUpdate`.
pub(crate) type RecvKey = (&'static str, Option<i32>, Option<i32>);

/// The join tables.
#[derive(Debug, Default)]
pub(crate) struct Timing {
    pending: VecDeque<Send>,
    casting: VecDeque<Casting>,
    recvs: HashMap<RecvKey, u64>,
}

impl Timing {
    /// A press reached the wire `press_to_sent_ms` after it was posted.
    pub(crate) fn press_sent(method: &'static str, press_to_sent_ms: u64) {
        ability_timing::observe("press_to_sent", method, press_to_sent_ms);
    }

    /// Hold a send the server answers with ability replies, until its first
    /// reply or [`PENDING_TTL_MS`].
    pub(crate) fn note_sent(
        &mut self,
        send_id: u32,
        press_id: Option<u32>,
        method: &'static str,
        ability_id: Option<i32>,
        now_ms: u64,
    ) {
        if !ANSWERED_METHODS.contains(&method) {
            return;
        }
        let Some(ability_id) = ability_id.filter(|&a| a != 0) else {
            return;
        };
        self.expire(now_ms);
        if self.pending.len() >= MAX_SENDS {
            self.pending.pop_front();
        }
        self.pending.push_back(Send {
            send_id,
            press_id,
            method,
            ability_id,
            at_ms: now_ms,
        });
    }

    fn expire(&mut self, now_ms: u64) {
        self.pending
            .retain(|s| now_ms.saturating_sub(s.at_ms) <= PENDING_TTL_MS);
        self.casting
            .retain(|c| now_ms.saturating_sub(c.last_ms) <= CAST_TTL_MS);
    }

    fn note_recv(&mut self, key: RecvKey, now_ms: u64) {
        if !self.recvs.contains_key(&key) && self.recvs.len() >= MAX_RECVS {
            self.recvs
                .retain(|_, &mut at| now_ms.saturating_sub(at) <= APPLY_TTL_MS);
            if self.recvs.len() >= MAX_RECVS {
                self.recvs.clear();
            }
        }
        self.recvs.insert(key, now_ms);
    }

    /// A reply arrived. Records it for the applied join, and, when it is a
    /// local reply to a cast, returns the send it belongs to (see the
    /// module docs for the rules).
    pub(crate) fn on_recv(&mut self, r: Reply, now_ms: u64) -> Option<Answered> {
        self.note_recv((r.method, r.entity_id, r.timer_id), now_ms);
        let (ability_id, kind) = r.answers?;
        if !r.local {
            return None;
        }
        self.expire(now_ms);

        // A refusal opens and ends a cast (the server refuses before it
        // launches anything), so it answers the oldest held send when there
        // is one, never a cast already under way.
        let refusal_for_held =
            kind == ReplyKind::Refusal && self.pending.iter().any(|s| s.ability_id == ability_id);
        // A reply of a cast already under way.
        if let Some(c) = self
            .casting
            .iter_mut()
            .find(|c| c.send.ability_id == ability_id && c.takes(kind, r.cast_id))
            .filter(|_| !refusal_for_held)
        {
            c.kinds |= kind.bit();
            c.cast_id = c.cast_id.or(r.cast_id);
            c.last_ms = now_ms;
            return Some(Answered {
                send_id: c.send.send_id,
                press_id: c.send.press_id,
                sent_method: c.send.method,
                sent_to_recv_ms: None,
            });
        }

        // The first reply of the oldest held send of this ability.
        let i = self
            .pending
            .iter()
            .position(|s| s.ability_id == ability_id)?;
        let send = self.pending.remove(i)?;
        if self.casting.len() >= MAX_SENDS {
            self.casting.pop_front();
        }
        self.casting.push_back(Casting {
            send,
            kinds: kind.bit(),
            cast_id: r.cast_id,
            last_ms: now_ms,
        });
        let ms = now_ms.saturating_sub(send.at_ms);
        ability_timing::observe("sent_to_recv", r.method, ms);
        Some(Answered {
            send_id: send.send_id,
            press_id: send.press_id,
            sent_method: send.method,
            sent_to_recv_ms: Some(ms),
        })
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
/// `method`, addressed to the local player when `local`: records the
/// receive, and when it belongs to a send adds `send_id`, `press_id`,
/// `sent_method`, `send_reply` (`first` | `follow_up`) and, on the first
/// reply, `sent_to_recv_ms`.
pub(crate) fn annotate_recv(fields: &mut Fields, method: &'static str, local: bool, now_ms: u64) {
    let reply = Reply {
        method,
        entity_id: as_i32(int(fields, "entity_id")),
        timer_id: if method == "onTimerUpdate" {
            as_i32(int(fields, "timer_id"))
        } else {
            None
        },
        answers: recv_reply(method, |k| int(fields, k)),
        cast_id: as_i32(int(fields, "cast_id")),
        local,
    };
    let answered = with_timing(|t| t.on_recv(reply, now_ms));
    if let Some(a) = answered {
        fields.push(("send_id", json!(a.send_id)));
        fields.push(("press_id", a.press_id.map_or(Value::Null, |p| json!(p))));
        fields.push(("sent_method", json!(a.sent_method)));
        match a.sent_to_recv_ms {
            Some(ms) => {
                fields.push(("send_reply", json!("first")));
                fields.push(("sent_to_recv_ms", json!(ms)));
            }
            None => fields.push(("send_reply", json!("follow_up"))),
        }
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
#[path = "timing_tests.rs"]
mod tests;
