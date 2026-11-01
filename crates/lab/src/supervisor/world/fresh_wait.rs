//! Wait for a post-read to be fresh (C4).

#![allow(dead_code)] // Callers land in wave 3 (AX-30..).

use super::io::WorldIo;
use crate::proof::fresh::{freshness, Freshness};

/// Re-reads after the first, one sleep apart.
const MAX_REREADS: u32 = 3;
const REREAD_SLEEP_MS: u64 = 50;

/// Read the frame counter until it is `FRESH_FRAMES` past `action_frame`.
/// A `Stale` read sleeps 50 ms and reads again, at most 3 times; the last
/// freshness and frame are returned. World tools call this after their
/// action and before the post-read.
pub async fn wait_fresh<W: WorldIo>(
    io: &mut W,
    action_frame: Option<u64>,
) -> (Freshness, Option<u64>) {
    let mut frame = io.frame().await;
    let mut fresh = freshness(action_frame, frame);
    for _ in 0..MAX_REREADS {
        if fresh != Freshness::Stale {
            break;
        }
        io.sleep(REREAD_SLEEP_MS).await;
        frame = io.frame().await;
        fresh = freshness(action_frame, frame);
    }
    (fresh, frame)
}
