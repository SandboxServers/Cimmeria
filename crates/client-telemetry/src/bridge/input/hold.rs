//! Key holds with a lease (lab autopilot contract C5). A hold is a posted
//! `WM_KEYDOWN` the lab promises to undo: it ends on an explicit up, on
//! lease expiry (checked every Tick), or when the daemon connection drops.
//!
//! Pure table: it takes the clock as an argument and returns the posts to
//! make, so host tests need neither a window nor `PostMessageW`.

use std::sync::atomic::AtomicU64;

use serde::Deserialize;

/// Most keys held at once; the 17th distinct key is an error.
pub const MAX_HOLDS: usize = 16;
/// Lease bounds in ms (D-AX8); absent means the maximum.
pub const MIN_LEASE_MS: u32 = 200;
pub const MAX_LEASE_MS: u32 = 10_000;

/// Holds ended by lease expiry.
pub(crate) static HOLDS_EXPIRED: AtomicU64 = AtomicU64::new(0);
/// Holds released because the daemon connection ended.
pub(crate) static HOLDS_RELEASED_ON_DISCONNECT: AtomicU64 = AtomicU64::new(0);

/// `input_hold` params. The daemon computes both lParams (C5).
#[derive(Debug, Clone, Deserialize)]
pub struct HoldParams {
    pub hwnd: u32,
    pub vk: u8,
    pub lparam_down: u32,
    pub lparam_up: u32,
    pub down: bool,
    #[serde(default)]
    pub lease_ms: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Hold {
    vk: u8,
    lparam_up: u32,
    hwnd: u32,
    until_ms: u64,
}

/// A window message to post.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Post {
    Down { hwnd: u32, vk: u8, lparam: u32 },
    Up { hwnd: u32, vk: u8, lparam: u32 },
}

#[derive(Debug, Default)]
pub(crate) struct HoldTable {
    holds: Vec<Hold>,
}

fn up(h: &Hold) -> Post {
    Post::Up {
        hwnd: h.hwnd,
        vk: h.vk,
        lparam: h.lparam_up,
    }
}

impl HoldTable {
    /// Hold `p.vk` down, or extend its lease. Returns whether a down was
    /// posted (false on an extension) and the posts to make.
    pub(crate) fn press(
        &mut self,
        p: &HoldParams,
        now_ms: u64,
    ) -> Result<(bool, Vec<Post>), String> {
        let lease = p
            .lease_ms
            .unwrap_or(MAX_LEASE_MS)
            .clamp(MIN_LEASE_MS, MAX_LEASE_MS);
        let until_ms = now_ms.saturating_add(u64::from(lease));
        if let Some(h) = self.holds.iter_mut().find(|h| h.vk == p.vk) {
            h.until_ms = until_ms;
            return Ok((false, Vec::new()));
        }
        if self.holds.len() >= MAX_HOLDS {
            return Err(format!("too many holds (max {MAX_HOLDS})"));
        }
        self.holds.push(Hold {
            vk: p.vk,
            lparam_up: p.lparam_up,
            hwnd: p.hwnd,
            until_ms,
        });
        Ok((
            true,
            vec![Post::Down {
                hwnd: p.hwnd,
                vk: p.vk,
                lparam: p.lparam_down,
            }],
        ))
    }

    /// Let a key go. The up is posted even when the key was not held: an
    /// extra key-up is harmless and covers a hold the table already lost.
    pub(crate) fn release(&mut self, p: &HoldParams) -> Vec<Post> {
        self.holds.retain(|h| h.vk != p.vk);
        vec![Post::Up {
            hwnd: p.hwnd,
            vk: p.vk,
            lparam: p.lparam_up,
        }]
    }

    /// Drop every hold whose lease ended, returning their key-ups.
    pub(crate) fn expire(&mut self, now_ms: u64) -> Vec<Post> {
        let mut posts = Vec::new();
        self.holds.retain(|h| {
            if h.until_ms <= now_ms {
                posts.push(up(h));
                false
            } else {
                true
            }
        });
        posts
    }

    /// Drop every hold, returning their key-ups.
    pub(crate) fn release_all(&mut self) -> Vec<Post> {
        self.holds.drain(..).map(|h| up(&h)).collect()
    }

    /// `(vk, ms left)` for each live hold.
    pub(crate) fn snapshot(&self, now_ms: u64) -> Vec<(u8, u32)> {
        self.holds
            .iter()
            .map(|h| {
                let left = h.until_ms.saturating_sub(now_ms);
                (h.vk, u32::try_from(left).unwrap_or(u32::MAX))
            })
            .collect()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.holds.is_empty()
    }
}

#[cfg(test)]
#[path = "hold_tests.rs"]
mod tests;
