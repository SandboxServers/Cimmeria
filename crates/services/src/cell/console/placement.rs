//! Selected-entity read/set position + orientation console commands
//! (category J): `.location`, `.rotation` (P18).
//!
//! Distinct from [`super::travel`]'s `.gotoxyz`/`.goto`/`.summon`/
//! `.gotolocation`: those move an entity to a *destination* (another player,
//! a named world) and always mutate. `.location`/`.rotation` are dual-mode —
//! zero args reports the target's current placement, a complete tuple sets
//! it — and P18's own scope note says to stop for explicit design rather
//! than silently drop axes if full Euler orientation can't be represented,
//! so this is a placeholder until that packet lands.
//!
//! `.lookat` (the third command P18's ledger entry originally listed) is
//! already implemented in [`super::entity`] (P13/P14-era work) — heading-only
//! "face the caller" rotation. Not this file's concern.
//!
//! Legacy reference: `deprecated/python/cell/commands/Entity.py`.

use tokio::sync::mpsc;

use super::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Route `.location` / `.rotation` (P18) to their handlers.
pub(super) async fn dispatch(
    name: &str,
    caller_id: u32,
    target: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    match name {
        "location" => location(caller_id, target, args, tx, space_mgr).await,
        "rotation" => rotation(caller_id, target, args, tx, space_mgr).await,
        _ => {}
    }
}

/// `.location` (report) / `.location <x> <y> <z>` (set) — the target's
/// position. **Stub — P18 not yet implemented.**
async fn location(
    caller_id: u32,
    _target: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    _space_mgr: &mut SpaceManager,
) {
    send_gm_feedback(
        caller_id,
        &format!("location: not yet implemented (P18) -- args: {args:?}"),
        tx,
    )
    .await;
}

/// `.rotation` (report) / `.rotation <pitch> <yaw> <roll>` (set) — the
/// target's orientation. **Stub — P18 not yet implemented.**
async fn rotation(
    caller_id: u32,
    _target: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    _space_mgr: &mut SpaceManager,
) {
    send_gm_feedback(
        caller_id,
        &format!("rotation: not yet implemented (P18) -- args: {args:?}"),
        tx,
    )
    .await;
}
