//! `client_input_release` goes through the supervisor's one release path.

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::server::LabServer;
use crate::supervisor::events::fake_bridge::{self, Responder};

#[tokio::test]
async fn tool_release_goes_through_the_helper() {
    let seen: Arc<Mutex<Vec<String>>> = Arc::default();
    let log = seen.clone();
    let r: Responder = Arc::new(move |m: &str, _p: &Value| {
        log.lock().unwrap().push(m.to_string());
        Ok(json!({ "posted": true, "held": [] }))
    });
    let sup = Arc::new(fake_bridge::supervisor(r).await);
    sup.input_key_at(7, "W", "down", None).await.unwrap();
    assert_eq!(sup.held_count().await, 1);

    let server = LabServer::new(sup.clone());
    server.client_input_release().await.unwrap();

    assert!(seen.lock().unwrap().iter().any(|m| m == "input_release"));
    assert_eq!(sup.held_count().await, 0, "the hold book must be emptied");
}
