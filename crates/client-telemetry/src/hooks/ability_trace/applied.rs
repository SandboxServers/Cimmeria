//! What the client applied: `client.ability.applied` (AB-C4).
//!
//! The detours (`hooks::inline_hooks::ability_apply`) observe the stock
//! handlers and the calls they make; this module turns what they saw into
//! one event per handler call. Every anchor is in
//! `docs/reverse-engineering/findings/ability-client-hook-anchors.md`
//! § AB-C4, with the corrections recorded there on 2026-10-04 (argument
//! counts, the stat functor's argument order, and what `0x00e0a810` is).
//!
//! | `kind` | From | Means |
//! |---|---|---|
//! | `effect_bar_add` | `EffectSet` timer handler `0x00e09160`, type 5, no entry for `SecondaryId`, `BigWorldTimeComplete` in the future | A new icon entry. `ui` says what the bar did with it: `posted` (the add went to the UI, `0x00e0a2d0` ran), `data_requested` (the display data was not cached: the client sent `Event_NetOut_elementDataRequest`, category 9, for the effect id instead), `data_request_pending` (a request was already out), or `no_ui` (no effect UI existed, so nothing was sent) |
//! | `effect_bar_refresh` | same, an entry exists, complete time in the future | The entry's interval was moved |
//! | `effect_bar_clear` | same, an entry exists, complete time not in the future | The server ended the effect (it sends `0.0`); the bar drops the entry on its next clock check |
//! | `effect_bar_ignored` | same, no entry, complete time not in the future | `reason = expired_on_arrival`: the handler creates nothing |
//! | `cooldown` | `CooldownManager` timer handler `0x00ea6af0` | `outcome`: `applied` (the UI callback `0x00ea62b0` ran), `no_button` (no slot shows this `(Type, ID)`), `other_source` (the timer is another being's) |
//! | `stat` / `stat_base` | `GameBeing` stat handlers `0x00e01f40` / `0x00e02060` and their per-stat functors | One event per message, every stat the functor stored |
//! | `state_flag` | `GameBeing::onStateFieldUpdate` `0x00e01c90` | The word before and after, and the bits that changed |
//!
//! There is no native "effect expired" call: the bar's entries lapse on the
//! clock (`getEffectInfo` computes the remaining time). A row carries
//! `complete_time`, `now` and `remaining`, so the expiry is
//! `complete_time` on the same clock; no `effect_bar_expired` event is
//! raised, because no verified seam exists for one.

use serde_json::{json, Value};

use super::clock;
use super::event_bag::Bag;
use super::wire_decode::float;
use crate::hooks::entity_trace::Fields;

/// `client.ability.applied`.
pub(crate) const TARGET_APPLIED: &str = "client.ability.applied";

/// The timer type the effect bar acts on (`0x00e09160` returns for any
/// other).
pub(crate) const TIMER_TYPE_EFFECT: u8 = 5;

/// The fields of an `onTimerUpdate` event, as the handlers read them.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Timer {
    pub id: Option<i32>,
    pub timer_type: Option<u8>,
    pub source_id: Option<i32>,
    pub secondary_id: Option<i32>,
    pub total_time: Option<f32>,
    pub complete_time: Option<f32>,
}

impl Timer {
    /// Read every field the two timer handlers read, with the getter each
    /// uses (`Type` is a byte; the rest are `GetInt` / `GetFloat`).
    pub(crate) fn read(bag: &dyn Bag) -> Self {
        Self {
            id: bag.int(b"ID\0"),
            timer_type: bag.byte(b"Type\0"),
            source_id: bag.int(b"SourceID\0"),
            secondary_id: bag.int(b"SecondaryId\0"),
            total_time: bag.float(b"TotalTime\0"),
            complete_time: bag.float(b"BigWorldTimeComplete\0"),
        }
    }

    fn push(&self, f: &mut Fields, now: Option<f64>) {
        f.push(("timer_id", json!(self.id)));
        f.push(("timer_type", json!(self.timer_type)));
        f.push(("source_id", json!(self.source_id)));
        f.push(("secondary_id", json!(self.secondary_id)));
        f.push(("total_time", self.total_time.map_or(Value::Null, float)));
        f.push((
            "complete_time",
            self.complete_time.map_or(Value::Null, float),
        ));
        if let Some(now) = now {
            f.push(("now", json!((now * 1000.0).round() / 1000.0)));
            if let Some(c) = self.complete_time {
                f.push(("remaining", json!(clock::remaining(c, now))));
            }
        }
    }
}

/// What the probes saw inside one `EffectSet` timer handler call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct EffectProbe {
    /// The entry lookup `0x00e08570` ran, and whether it found one.
    pub lookup_hit: Option<bool>,
    /// The new entry was announced (`0x00e0a9e0`).
    pub announced: bool,
    /// Inside the announcement, the display data was missing and the
    /// data request `0x00e0a810` ran; `Some(true)` when a request was
    /// already outstanding (`[this+0x48]` set), so nothing new was sent.
    pub data_request: Option<bool>,
    /// Inside the announcement, the add was posted to the effect UI
    /// (`0x00e0a2d0`).
    pub posted: bool,
}

/// `(kind, extra fields)` for one effect-bar handler call, or `None` for
/// a timer the bar ignores by type. `complete_in_future` is the handler's
/// own test (`BigWorldTimeComplete > now`), as far as the hook could read
/// the clock.
pub(crate) fn effect_bar_kind(
    timer_type: Option<u8>,
    probe: &EffectProbe,
    complete_in_future: Option<bool>,
) -> Option<(&'static str, Fields)> {
    if timer_type != Some(TIMER_TYPE_EFFECT) {
        return None;
    }
    let mut extra: Fields = Vec::new();
    let kind = if probe.announced {
        let ui = match (probe.posted, probe.data_request) {
            (true, _) => "posted",
            (false, Some(false)) => "data_requested",
            (false, Some(true)) => "data_request_pending",
            // `0x00e0a9e0` returns at once when `0x00e0a1f0` finds no
            // effect UI (before the UI exists): nothing was sent anywhere.
            (false, None) => "no_ui",
        };
        extra.push(("ui", json!(ui)));
        "effect_bar_add"
    } else {
        match (probe.lookup_hit, complete_in_future) {
            (Some(true), Some(false)) => "effect_bar_clear",
            (Some(true), _) => "effect_bar_refresh",
            (Some(false), _) => {
                extra.push(("reason", json!("expired_on_arrival")));
                "effect_bar_ignored"
            }
            // The lookup probe did not report (its hook is not installed).
            (None, _) => {
                extra.push(("reason", json!("unobserved")));
                "effect_bar_update"
            }
        }
    };
    Some((kind, extra))
}

/// The `effect_bar_*` event for one handler call on the `EffectSet` of
/// `owner_id`.
pub(crate) fn effect_bar_fields(
    owner_id: Option<i32>,
    timer: &Timer,
    probe: &EffectProbe,
    now: Option<f64>,
) -> Option<(&'static str, Fields)> {
    let future = match (timer.complete_time, now) {
        (Some(c), Some(n)) => Some(f64::from(c) > n),
        _ => None,
    };
    let (kind, extra) = effect_bar_kind(timer.timer_type, probe, future)?;
    let mut f: Fields = vec![("kind", json!(kind)), ("entity_id", json!(owner_id))];
    // The effect-bar entry is keyed by `SecondaryId` and shows `ID`.
    f.push(("effect_id", json!(timer.id)));
    timer.push(&mut f, now);
    f.extend(extra);
    Some((kind, f))
}

/// What the `CooldownManager` handler did with one timer.
pub(crate) fn cooldown_outcome(
    owner_id: Option<i32>,
    source_id: Option<i32>,
    ui_called: bool,
) -> &'static str {
    if owner_id.is_some() && source_id.is_some() && owner_id != source_id {
        "other_source"
    } else if ui_called {
        "applied"
    } else {
        "no_button"
    }
}

/// The `cooldown` event. `ui_values` are the two floats the handler hands
/// the button callback (`0x00ea62b0`, arguments 3 and 4), from the
/// manager's interval query `0x00ea6120`.
pub(crate) fn cooldown_fields(
    owner_id: Option<i32>,
    timer: &Timer,
    ui_values: Option<(f32, f32)>,
    now: Option<f64>,
) -> (&'static str, Fields) {
    let outcome = cooldown_outcome(owner_id, timer.source_id, ui_values.is_some());
    let mut f: Fields = vec![
        ("kind", json!("cooldown")),
        ("outcome", json!(outcome)),
        ("entity_id", json!(owner_id)),
        ("ability_id", json!(timer.id)),
    ];
    timer.push(&mut f, now);
    if let Some((a, b)) = ui_values {
        f.push(("ui_values", json!([float(a), float(b)])));
    }
    let level = if outcome == "other_source" {
        "debug"
    } else {
        "info"
    };
    (level, f)
}

/// Stats kept per event; the count field has them all.
pub(crate) const MAX_STATS: usize = 64;

/// One stat as the functor received it. The iterator (`0x00e00e60`) reads
/// `StatId`, `Current`, `Max`, `Min` from each list element and calls the
/// functor as `(StatId, Max, Min, Current)` (pushes at `0x00e011d3` to
/// `0x00e011de`): not the wire order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StatCall {
    pub stat_id: i32,
    pub max: i32,
    pub min: i32,
    pub current: i32,
}

impl StatCall {
    /// `[StatId, Min, Current, Max]`: the wire order, so an applied row
    /// compares directly with the recv row's `stats`.
    pub(crate) fn row(&self) -> Value {
        json!([self.stat_id, self.min, self.current, self.max])
    }
}

/// The `stat` / `stat_base` event for one handler call.
pub(crate) fn stat_fields(
    entity_id: Option<i32>,
    base: bool,
    calls: &[StatCall],
    total: usize,
) -> Fields {
    vec![
        ("kind", json!(if base { "stat_base" } else { "stat" })),
        ("entity_id", json!(entity_id)),
        ("stats_count", json!(total)),
        (
            "stats",
            Value::Array(calls.iter().take(MAX_STATS).map(StatCall::row).collect()),
        ),
    ]
}

/// The `state_flag` event: the being's state word before and after the
/// handler (`[GameBeing+0x158]`, which `0x00e01c90` overwrites with
/// `bStateField` at `0x00e01d75`).
pub(crate) fn state_flag_fields(
    entity_id: Option<i32>,
    old: Option<u32>,
    new: Option<u32>,
) -> Fields {
    let mut f: Fields = vec![
        ("kind", json!("state_flag")),
        ("entity_id", json!(entity_id)),
        ("old", json!(old)),
        ("new", json!(new)),
    ];
    if let (Some(o), Some(n)) = (old, new) {
        f.push(("changed", json!(o ^ n)));
        f.push(("set", json!(n & !o)));
        f.push(("cleared", json!(o & !n)));
    }
    f
}

#[cfg(test)]
mod tests {
    use super::super::event_bag::fake::FakeBag;
    use super::*;

    fn get(f: &Fields, k: &str) -> Value {
        f.iter()
            .find(|(n, _)| *n == k)
            .map(|(_, v)| v.clone())
            .unwrap_or(Value::Null)
    }

    fn timer(complete: f32) -> Timer {
        Timer {
            id: Some(1201),
            timer_type: Some(5),
            source_id: Some(77),
            secondary_id: Some(31337),
            total_time: Some(30.0),
            complete_time: Some(complete),
        }
    }

    #[test]
    fn the_timer_is_read_with_each_fields_own_getter() {
        let mut bag = FakeBag::default();
        bag.ints.insert("ID", 597);
        bag.bytes.insert("Type", 5);
        bag.ints.insert("SourceID", 77);
        bag.ints.insert("SecondaryId", 31337);
        bag.floats.insert("TotalTime", 12.5);
        bag.floats.insert("BigWorldTimeComplete", 1234.5);
        let t = Timer::read(&bag);
        assert_eq!(
            t,
            Timer {
                id: Some(597),
                timer_type: Some(5),
                source_id: Some(77),
                secondary_id: Some(31337),
                total_time: Some(12.5),
                complete_time: Some(1234.5),
            }
        );
        // A field the bag lacks is absent, not 0.
        assert_eq!(Timer::read(&FakeBag::default()).id, None);
    }

    /// Every branch of `0x00e09160`, as the probes see it.
    #[test]
    fn effect_bar_kinds_follow_the_handlers_branches() {
        let add = EffectProbe {
            lookup_hit: Some(false),
            announced: true,
            data_request: None,
            posted: true,
        };
        let (kind, f) = effect_bar_fields(Some(77), &timer(130.0), &add, Some(100.0)).unwrap();
        assert_eq!(kind, "effect_bar_add");
        assert_eq!(get(&f, "ui"), json!("posted"));
        assert_eq!(get(&f, "effect_id"), json!(1201));
        assert_eq!(get(&f, "secondary_id"), json!(31337));
        assert_eq!(get(&f, "remaining"), json!(30.0));
        assert_eq!(get(&f, "entity_id"), json!(77));

        let requested = EffectProbe {
            data_request: Some(false),
            posted: false,
            ..add
        };
        let (_, f) = effect_bar_fields(Some(77), &timer(130.0), &requested, Some(100.0)).unwrap();
        assert_eq!(get(&f, "ui"), json!("data_requested"));
        // Announced, but neither posted nor requested: no effect UI yet.
        // That must not read as shown.
        let no_ui = EffectProbe {
            posted: false,
            ..add
        };
        let (_, f) = effect_bar_fields(Some(77), &timer(130.0), &no_ui, Some(100.0)).unwrap();
        assert_eq!(get(&f, "ui"), json!("no_ui"));

        let hit = EffectProbe {
            lookup_hit: Some(true),
            ..EffectProbe::default()
        };
        let refresh = effect_bar_fields(Some(77), &timer(130.0), &hit, Some(100.0)).unwrap();
        assert_eq!(refresh.0, "effect_bar_refresh");
        // The server ends an effect by sending a complete time of 0.
        let clear = effect_bar_fields(Some(77), &timer(0.0), &hit, Some(100.0)).unwrap();
        assert_eq!(clear.0, "effect_bar_clear");
        assert_eq!(get(&clear.1, "remaining"), json!(-100.0));

        let miss = EffectProbe {
            lookup_hit: Some(false),
            ..EffectProbe::default()
        };
        let (kind, f) = effect_bar_fields(Some(77), &timer(50.0), &miss, Some(100.0)).unwrap();
        assert_eq!(kind, "effect_bar_ignored");
        assert_eq!(get(&f, "reason"), json!("expired_on_arrival"));

        // Without the clock, a hit is still a refresh, not a guess at a clear.
        let (kind, f) = effect_bar_fields(Some(77), &timer(0.0), &hit, None).unwrap();
        assert_eq!(kind, "effect_bar_refresh");
        assert_eq!(get(&f, "now"), Value::Null);
    }

    /// The bar returns at once for any timer type but 5; so does the event.
    #[test]
    fn non_effect_timers_are_not_effect_bar_events() {
        let mut t = timer(130.0);
        t.timer_type = Some(1);
        assert!(effect_bar_fields(Some(77), &t, &EffectProbe::default(), Some(1.0)).is_none());
    }

    #[test]
    fn cooldown_outcomes() {
        assert_eq!(cooldown_outcome(Some(5), Some(5), true), "applied");
        assert_eq!(cooldown_outcome(Some(5), Some(5), false), "no_button");
        assert_eq!(cooldown_outcome(Some(5), Some(6), false), "other_source");
        let mut t = timer(110.0);
        t.timer_type = Some(1);
        t.source_id = Some(5);
        let (level, f) = cooldown_fields(Some(5), &t, Some((10.0, 30.0)), Some(100.0));
        assert_eq!(level, "info");
        assert_eq!(get(&f, "outcome"), json!("applied"));
        assert_eq!(get(&f, "ability_id"), json!(1201));
        assert_eq!(get(&f, "ui_values"), json!([10.0, 30.0]));
        assert_eq!(get(&f, "remaining"), json!(10.0));
        let (level, f) = cooldown_fields(Some(9), &t, None, None);
        assert_eq!(
            (level, get(&f, "outcome")),
            ("debug", json!("other_source"))
        );
    }

    /// The functor's `(StatId, Max, Min, Current)` becomes the wire's
    /// `[StatId, Min, Current, Max]`.
    #[test]
    fn stat_rows_are_in_wire_order() {
        let calls = [StatCall {
            stat_id: 6,
            max: 1000,
            min: 0,
            current: 840,
        }];
        let f = stat_fields(Some(77), false, &calls, 1);
        assert_eq!(get(&f, "kind"), json!("stat"));
        assert_eq!(get(&f, "stats"), json!([[6, 0, 840, 1000]]));
        assert_eq!(get(&f, "stats_count"), json!(1));
        let many: Vec<StatCall> = (0..100)
            .map(|i| StatCall {
                stat_id: i,
                ..calls[0]
            })
            .collect();
        let f = stat_fields(Some(77), true, &many, 100);
        assert_eq!(get(&f, "kind"), json!("stat_base"));
        assert_eq!(get(&f, "stats").as_array().unwrap().len(), MAX_STATS);
        assert_eq!(get(&f, "stats_count"), json!(100));
    }

    #[test]
    fn state_flags_report_the_changed_bits() {
        let f = state_flag_fields(Some(77), Some(0b0110), Some(0b0011));
        assert_eq!(get(&f, "changed"), json!(0b0101));
        assert_eq!(get(&f, "set"), json!(0b0001));
        assert_eq!(get(&f, "cleared"), json!(0b0100));
        let unread = state_flag_fields(Some(77), None, Some(1));
        assert_eq!(get(&unread, "changed"), Value::Null);
    }
}
