//! Post-commit client updates: a failed item notification is followed by
//! one recovery pass, so the client never keeps a stack that is gone.

use std::sync::atomic::{AtomicBool, Ordering};

use super::*;
use crate::base::crafting::test_packets::{remove_item_ids, update_item_rows};
use crate::mercury::method_idx;
use crate::test_support::require_db_or_skip;

/// Refuses the first send, then passes every send to `inner`.
struct FailFirstSend {
    inner: Arc<TestTransport>,
    failed: AtomicBool,
}

impl Transport for FailFirstSend {
    fn send_to<'life0, 'life1, 'async_trait>(
        &'life0 self,
        bytes: &'life1 [u8],
        addr: SocketAddr,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::io::Result<usize>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        if !self.failed.swap(true, Ordering::SeqCst) {
            return Box::pin(async { Err(std::io::Error::other("transient send failure")) });
        }
        self.inner.send_to(bytes, addr)
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.inner.local_addr()
    }
}

/// The `onRemoveItem` for a drained stack fails, the `onUpdateItem` after
/// it goes out. The client then gets the removal again and a full
/// inventory, instead of keeping the drained stack on screen.
#[tokio::test]
async fn live_db_a_failed_removal_is_followed_by_one_recovery_pass() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let mut f = Fixture::new(&pool, 20).await;
    f.env.transport = Arc::new(FailFirstSend {
        inner: f.transport.clone(),
        failed: AtomicBool::new(false),
    });
    let drained = f.stack(COMPONENT, INV_CRAFTING, 0, 1).await;
    let kept = f.stack(COMPONENT, INV_MAIN, 0, 2).await;

    f.apply(&CraftTransaction {
        named_items: vec![NamedItem::new(kept, COMPONENT)],
        consume: vec![(COMPONENT, 2)],
        ..CraftTransaction::default()
    })
    .await
    .expect("craft commits");

    let calls = f.calls();
    let methods: Vec<u16> = calls.iter().map(|c| c.method).collect();
    assert_eq!(
        methods,
        vec![
            method_idx::ON_UPDATE_ITEM,
            method_idx::ON_REMOVE_ITEM,
            method_idx::ON_UPDATE_ITEM,
        ],
        "the update, then the removal again and a full inventory: {calls:?}"
    );
    assert_eq!(update_item_rows(&calls[0]), vec![(kept, 1, INV_MAIN, 1)]);
    assert_eq!(remove_item_ids(&calls[1]), vec![drained]);
    assert_eq!(update_item_rows(&calls[2]), vec![(kept, 1, INV_MAIN, 1)]);
    f.cleanup().await;
}
