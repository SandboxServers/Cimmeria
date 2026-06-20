//! In-memory [`MockSender`] used by tests.

use std::sync::Arc;

use serde_json::Value;
use tokio::sync::Mutex;

use super::handle::{DiscordSender, SendError};

/// In-memory recorder used by tests. Records every `(url, body)` pair
/// and lets the test assert ordering, count, or content. `next_error`
/// lets a test simulate a failure response on the next call.
#[derive(Default)]
pub struct MockSender {
    pub(super) calls: Arc<Mutex<Vec<(String, Value)>>>,
    next_error: Arc<Mutex<Option<SendError>>>,
}

impl MockSender {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn calls(&self) -> Vec<(String, Value)> {
        self.calls.lock().await.clone()
    }

    pub async fn call_count(&self) -> usize {
        self.calls.lock().await.len()
    }

    pub async fn set_next_error(&self, e: SendError) {
        *self.next_error.lock().await = Some(e);
    }

    /// Cheap handle to the underlying call log, for assertion helpers
    /// in cross-module tests (layer.rs, integration tests, …).
    pub fn calls_handle(&self) -> Arc<Mutex<Vec<(String, Value)>>> {
        self.calls.clone()
    }
}

#[async_trait::async_trait]
impl DiscordSender for MockSender {
    async fn send(&self, url: &str, body: &Value) -> Result<(), SendError> {
        if let Some(e) = self.next_error.lock().await.take() {
            return Err(e);
        }
        self.calls
            .lock()
            .await
            .push((url.to_string(), body.clone()));
        Ok(())
    }
}
