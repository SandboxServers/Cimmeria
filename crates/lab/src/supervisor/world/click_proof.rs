//! The evidence behind a world click (contract C1, C2, C4): did the client
//! show the effect the caller asked for, and was a right-click even in range.
//! Pure: it takes plain values, so the tests need no `Sim`.

use serde_json::{json, Value};

use super::click::{met, ui_snap, Expect, UiSnap};
use super::click_range::INTERACT_RANGE_M;
use super::fresh_wait::wait_fresh;
use super::io::WorldIo;
use super::{Steps, WorldError};
use crate::proof::fresh::{mark_unstamped, Freshness};
use crate::proof::{Check, Evidence, Source, Why};

/// What the click step knows once it has taken its fresh post-read.
pub(super) struct ClickFacts<'a> {
    pub expect: Expect,
    pub entity: Option<u32>,
    pub before: &'a UiSnap,
    /// The fresh post-read (step 2).
    pub after: &'a UiSnap,
    pub hover_verified: bool,
    /// 0 left, 1 right.
    pub button: u8,
    pub range_m: Option<f64>,
    pub fresh: Freshness,
    pub read_frame: Option<u64>,
}

/// The checks of a click, in the order the verdict reads them.
pub(super) fn checks(f: &ClickFacts) -> Vec<Check> {
    let mut out = Vec::new();
    // The server ignores a right-click beyond its interact range, so a click
    // from there can never have done what it was for (#1660).
    if f.button == 1 {
        if let Some(d) = f.range_m {
            let in_range = d <= INTERACT_RANGE_M;
            let mut c = Check::new("in_range", Source::Client, Some(in_range), !in_range)
                .with_before(json!(INTERACT_RANGE_M))
                .with_after(json!((d * 100.0).round() / 100.0));
            if !in_range {
                c = c.with_why(Why::OutOfRange);
            }
            out.push(c);
        }
    }
    let opened: Vec<&String> = f
        .after
        .windows
        .iter()
        .filter(|w| !f.before.windows.contains(w))
        .collect();
    let decider = match f.expect {
        Expect::Target => Some((
            "target_is_entity",
            met(Expect::Target, f.entity, f.before, f.after),
            json!(f.before.target),
            json!(f.after.target),
        )),
        Expect::Window => Some((
            "window_opened",
            met(Expect::Window, f.entity, f.before, f.after),
            json!(f.before.windows),
            json!(opened),
        )),
        Expect::Any => Some((
            "effect_seen",
            met(Expect::Any, f.entity, f.before, f.after),
            json!({ "target": f.before.target, "windows": f.before.windows }),
            json!({ "target": f.after.target, "windows_opened": opened }),
        )),
        Expect::Nothing => None,
    };
    match decider {
        Some((name, ok, before, after)) => {
            let (ok, why, after) = match f.fresh {
                Freshness::Stale => (None, Why::StaleRead, after),
                Freshness::Unstamped => (Some(ok), Why::NoEffect, mark_unstamped(after)),
                Freshness::Fresh => (Some(ok), Why::NoEffect, after),
            };
            out.push(decider_check(name, ok, why, before, after, f.read_frame));
        }
        None => {
            out.push(Check::new("post_read", Source::Client, None, true).with_why(Why::NoPostRead))
        }
    }
    out.push(
        Check::new(
            "hover_verified",
            Source::Client,
            Some(f.hover_verified),
            false,
        )
        .with_why(Why::NoHover),
    );
    out
}

/// What `judge` needs besides the I/O: the click as it was planned.
pub(super) struct ClickCase<'a> {
    pub expect: Expect,
    pub entity: Option<u32>,
    pub before: &'a UiSnap,
    pub hover_verified: bool,
    pub button: usize,
    pub range_m: Option<f64>,
    /// `(Unit.Target, Unit.MouseOver)` slots for the post-read.
    pub slots: (i32, i32),
}

/// Wait for a fresh frame, read the UI once more and attach the evidence.
/// `expect: nothing` takes no post-read (a click may start a world load).
pub(super) async fn judge<W: WorldIo>(
    io: &mut W,
    steps: &mut Steps,
    action_frame: Option<u64>,
    c: ClickCase<'_>,
) -> Result<(), WorldError> {
    let (mut fresh, mut read_frame) = (Freshness::Fresh, None);
    let mut post = None;
    if c.expect != Expect::Nothing {
        (fresh, read_frame) = wait_fresh(io, action_frame).await;
        post = Some(
            ui_snap(io, c.slots.0, c.slots.1)
                .await
                .map_err(|e| steps.fail(io.now_ms(), "read_after", e))?,
        );
    }
    let after = post.as_ref().unwrap_or(c.before);
    let facts = ClickFacts {
        expect: c.expect,
        entity: c.entity,
        before: c.before,
        after,
        hover_verified: c.hover_verified,
        button: u8::from(c.button == 1),
        range_m: c.range_m,
        fresh,
        read_frame,
    };
    steps.judged(Evidence::judge(checks(&facts)));
    Ok(())
}

fn decider_check(
    name: &str,
    ok: Option<bool>,
    why: Why,
    before: Value,
    after: Value,
    frame: Option<u64>,
) -> Check {
    let mut c = Check::new(name, Source::Client, ok, true)
        .with_before(before)
        .with_after(after)
        .with_why(why);
    if let Some(fr) = frame {
        c = c.with_frame(fr);
    }
    c
}
