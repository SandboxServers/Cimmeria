//! Live adapter that lets the (sync, unit-tested) autologin state
//! machine drive the real client over the async bridge.
//!
//! Autologin's [`ClientScreen`](super::autologin::ClientScreen) trait is
//! sync so its logic is testable without a runtime. This adapter runs
//! inside `spawn_blocking`, so it can `Handle::block_on` each async
//! bridge call. The same `autologin::run` that the unit tests exercise
//! runs here against the real client.
//!
//! # Reads are blocked on Phase-3 return-value capture
//!
//! `is_visible`, `character_count`, and `character_info` need
//! `client_lua_eval` to return the chunk's results. Phase-1 lua_eval
//! (#684) returns an empty `results` array, so [`first_result`] fails
//! with a clear message and autologin can't progress live yet. The
//! *actions* (`eval`) work today. This is the single largest live-
//! validation gap called out in the issue.

use std::sync::Arc;

use serde_json::{json, Value};
use tokio::runtime::Handle;

use crate::client::BridgeClient;

use super::autologin::{CharInfo, ClientScreen};

pub struct BridgeScreen {
    pub bridge: Arc<BridgeClient>,
    pub handle: Handle,
}

impl BridgeScreen {
    /// Run a Lua chunk over the bridge, blocking the current
    /// (spawn_blocking) thread on the async call.
    fn eval_raw(&self, chunk: &str) -> Result<Value, String> {
        self.handle
            .block_on(self.bridge.call("lua_eval", json!({ "chunk": chunk })))
            .map_err(|e| format!("bridge lua_eval: {e}"))
    }

    /// First stringified return value of a `return …` chunk, or an
    /// explicit "not captured yet" error under phase-1 lua_eval.
    fn first_result(&self, chunk: &str) -> Result<String, String> {
        let v = self.eval_raw(chunk)?;
        let results = v.get("results").and_then(Value::as_array);
        match results.and_then(|a| a.first()).and_then(Value::as_str) {
            Some(s) => Ok(s.to_string()),
            None => Err(format!(
                "lua_eval returned no results for {chunk:?} — autologin reads need \
                 return-value capture, a Phase-3 client-bridge TODO (issue #684 \
                 lua_eval currently discards results)"
            )),
        }
    }
}

impl ClientScreen for BridgeScreen {
    fn is_visible(&mut self, window: &str) -> Result<bool, String> {
        let s = self.first_result(&format!("return {window}:isVisible()"))?;
        Ok(matches!(s.trim(), "true" | "1"))
    }

    fn eval(&mut self, chunk: &str) -> Result<(), String> {
        // Fire-and-forget action — works under phase-1 lua_eval.
        self.eval_raw(chunk).map(|_| ())
    }

    fn character_count(&mut self) -> Result<u32, String> {
        let s = self.first_result("return getCharacterCount()")?;
        s.trim()
            .parse::<u32>()
            .map_err(|e| format!("getCharacterCount parse: {e}"))
    }

    fn character_info(&mut self, index: u32) -> Result<CharInfo, String> {
        // Two results: name, playable.
        let v = self.eval_raw(&format!(
            "local c = getCharacterInfo({index}); return c.name, c.playable"
        ))?;
        let results = v
            .get("results")
            .and_then(Value::as_array)
            .filter(|a| a.len() >= 2)
            .ok_or_else(|| {
                format!(
                    "getCharacterInfo({index}) returned <2 results — needs Phase-3 \
                     lua_eval return-value capture"
                )
            })?;
        let name = results[0].as_str().unwrap_or_default().to_string();
        let playable = results[1]
            .as_str()
            .and_then(|s| s.trim().parse::<i64>().ok())
            .unwrap_or(0);
        Ok(CharInfo { name, playable })
    }
}
