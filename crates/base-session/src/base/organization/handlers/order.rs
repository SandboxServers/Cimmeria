//! Per-organization ordering of an edit and its post-commit fanout (ORG-08).
//!
//! Two edits of one organization commit in ORG-LOCK order, but their
//! post-commit sends are separate tasks that nothing orders. When the
//! second send overtakes the first, a client ends on the older state: an
//! officer note that arrives after the revoke that blanked it, or the older
//! of two MOTDs. [`org_order_guard`] closes that: a handler takes it before
//! it opens its transaction and drops it after its last send, so the sends
//! go out in commit order.
//!
//! Deadlock-free by construction: the guard is always taken before the
//! transaction, and no path waits on it while holding a database lock.
//! Holders: CM 13-17 (ORG-08), the rank change (0xD2) and `.org_rank`,
//! whose moves change who may read officer notes.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

type Guards = Mutex<HashMap<i32, Arc<AsyncMutex<()>>>>;

fn guards() -> &'static Guards {
    static GUARDS: OnceLock<Guards> = OnceLock::new();
    GUARDS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Wait for, and hold, `org_id`'s edit-and-fanout order. One entry per
/// organization ever edited in this process; an entry is a few bytes, so
/// they are not reclaimed.
pub async fn org_order_guard(org_id: i32) -> OwnedMutexGuard<()> {
    let lock = {
        let mut map = guards().lock().unwrap_or_else(|p| p.into_inner());
        map.entry(org_id).or_default().clone()
    };
    lock.lock_owned().await
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    /// The second holder of one organization waits for the first; another
    /// organization does not.
    #[tokio::test]
    async fn one_holder_per_organization() {
        let held = org_order_guard(0x7000_5801).await;
        let same =
            tokio::time::timeout(Duration::from_millis(50), org_order_guard(0x7000_5801)).await;
        assert!(same.is_err(), "a second holder of the same org must wait");
        let other =
            tokio::time::timeout(Duration::from_millis(50), org_order_guard(0x7000_5802)).await;
        assert!(other.is_ok(), "another org is independent");
        drop(held);
        assert!(
            tokio::time::timeout(Duration::from_millis(50), org_order_guard(0x7000_5801))
                .await
                .is_ok()
        );
    }
}
