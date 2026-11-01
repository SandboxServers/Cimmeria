//! Checks for the item, drag, vendor and world-map tools (AX-41).
//!
//! Each function takes the plain values its tool already computed and
//! returns the [`Check`] list the tool judges. No supervisor, no bridge and
//! no `await` here: the tools call [`Evidence::judge`] on the result. A
//! check that is not measured stays `ok: None`, so the verdict is
//! inconclusive rather than a pass.

use serde_json::{json, Value};

use crate::proof::{Check, Evidence, Source, Why};

/// The loot window's counters around a loot action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LootFacts {
    pub count_before: u32,
    pub count_after: u32,
    pub window_open_after: bool,
}

/// The evidence of `client_item_action`, from the result it is building.
/// `is_loot` is true for Loot All and one loot row, whose counters decide.
pub fn item_evidence(out: &Value, expect_change: bool, is_loot: bool) -> Evidence {
    let loot = is_loot.then(|| LootFacts {
        count_before: out["loot_count_before"].as_u64().unwrap_or_default() as u32,
        count_after: out["loot_count_after"].as_u64().unwrap_or_default() as u32,
        window_open_after: out["loot_window_open"] == json!(true),
    });
    let inventory_changed = out["inventory_changed"] == json!(true);
    let double_use_risk = out["double_use_risk"] == json!(true);
    Evidence::judge(item_checks(
        expect_change,
        inventory_changed,
        &out["diff"],
        loot,
        double_use_risk,
    ))
}

/// The checks of `client_item_action`. Loot decides by its counters; a
/// positive action must change the inventory; a negative one (`expect_change`
/// false) must not. A possible double use is a note, never a verdict.
pub fn item_checks(
    expect_change: bool,
    inventory_changed: bool,
    diff: &Value,
    loot: Option<LootFacts>,
    double_use_risk: bool,
) -> Vec<Check> {
    let mut checks = Vec::new();
    if let Some(l) = loot {
        let taken = l.count_after < l.count_before || !l.window_open_after || inventory_changed;
        checks.push(
            Check::new("loot_taken", Source::Client, Some(taken), true)
                .with_before(json!(l.count_before))
                .with_after(json!(l.count_after)),
        );
    } else if expect_change {
        checks.push(
            Check::new(
                "inventory_changed",
                Source::Client,
                Some(inventory_changed),
                true,
            )
            .with_after(diff.clone()),
        );
    } else {
        checks.push(
            Check::new(
                "inventory_unchanged",
                Source::Client,
                Some(!inventory_changed),
                true,
            )
            .with_after(diff.clone()),
        );
    }
    if double_use_risk {
        checks.push(
            Check::new("double_use_risk", Source::Client, Some(false), false)
                .with_after(json!(
                    "the use may have fired twice; the inventory read cannot tell"
                ))
                .with_why(Why::NoEffect),
        );
    }
    checks
}

/// The checks of `client_drag_drop`: the drag must start, and then the
/// item must move. A drag that never started is ignored, not failed.
pub fn drag_checks(
    drag_started: bool,
    moved: bool,
    moved_check: &str,
    snap_back: bool,
) -> Vec<Check> {
    vec![
        Check::new("drag_started", Source::Client, Some(drag_started), true).with_why(Why::Ignored),
        Check::new("moved", Source::Client, Some(moved), true)
            .with_after(json!({ "moved_check": moved_check, "snap_back": snap_back }))
            .with_why(Why::NoEffect),
    ]
}

/// The checks of `client_vendor_buy`. A purchase must arrive, and the
/// quantity must match; a negative check (`expect_change` false) must buy
/// nothing.
pub fn vendor_checks(
    expect_change: bool,
    bought: bool,
    items_gained: u32,
    want: u32,
) -> Vec<Check> {
    if expect_change {
        vec![
            Check::new(
                "items_arrived",
                Source::Client,
                Some(bought && items_gained > 0),
                true,
            ),
            Check::new(
                "quantity_matched",
                Source::Client,
                Some(items_gained == want),
                true,
            )
            .with_before(json!(want))
            .with_after(json!(items_gained))
            .with_why(Why::NoEffect),
        ]
    } else {
        vec![Check::new(
            "nothing_bought",
            Source::Client,
            Some(items_gained == 0),
            true,
        )]
    }
}

/// The checks of `client_world_map_click`: the map must close. The server
/// half (the destination packet) is a note: the tool cannot drain the
/// packet tap, which the runner owns, so it says so instead of guessing.
pub fn map_checks(map_closed: bool) -> Vec<Check> {
    vec![
        Check::new("map_closed", Source::Client, Some(map_closed), true),
        Check::new("destination_sent", Source::Server, None, false)
            .with_after(json!("method 91 on the packet tap; a UAT clause proves it"))
            .with_why(Why::ServerUnreachable),
    ]
}

#[cfg(test)]
#[path = "item_proof_tests.rs"]
mod tests;
