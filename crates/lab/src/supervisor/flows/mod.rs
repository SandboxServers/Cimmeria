//! Client-driving flows: login, character select, play, dialog, logout.
//!
//! Each flow is supervisor-side orchestration over the native-input
//! primitives in [`super::input`] (`ui_click`, key taps, typing) plus Lua
//! *reads* (visibility, widget text, the character list). Lua never
//! presses a button here: every click and key goes through the game's own
//! input handling, so a flow exercises the same path a player does. The
//! one exception is selecting a server row by name (the rows are list
//! items, not named windows) — see [`login`].
//!
//! Every wait is a Lua boolean expression polled until it holds
//! ([`FlowRun::wait`]), optionally with a *fail* expression (usually "a
//! prompt is showing") that ends the wait early with the prompt's text.
//! A flow returns JSON with per-step timings; a failure is a
//! [`FlowError`] that names the flow, the step, the widget or state that
//! failed, and the screens visible at that moment.
//!
//! The widget names and timings were proven on the live client by the
//! prototype scripts this module replaces (2026-09-29, against the colo).

pub mod characters;
pub mod create_confirm;
pub mod login;
pub mod ui_state;
pub mod widgets;
pub mod world;

use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::Supervisor;

/// Default poll cadence for flow waits.
pub const POLL: Duration = Duration::from_millis(500);

/// A flow failure: which flow, which step, what went wrong, and what the
/// client was showing. Boxed so a flow's `Result` stays pointer-sized.
#[derive(Debug, Clone)]
pub struct FlowError(Box<FlowFailure>);

impl std::ops::Deref for FlowError {
    type Target = FlowFailure;
    fn deref(&self) -> &FlowFailure {
        &self.0
    }
}

/// The fields of a [`FlowError`].
#[derive(Debug, Clone)]
pub struct FlowFailure {
    pub flow: &'static str,
    pub step: String,
    pub message: String,
    /// Visible screens / prompt text at failure time (best effort).
    pub state: Option<Value>,
    pub elapsed_ms: u64,
    /// Steps completed before the failure, with their timings.
    pub steps: Vec<Value>,
}

impl FlowError {
    /// One-line message for the MCP error text.
    pub fn summary(&self) -> String {
        let state = self
            .state
            .as_ref()
            .map(|s| format!("; client state: {s}"))
            .unwrap_or_default();
        format!(
            "{} failed at step '{}' after {} ms: {}{state}",
            self.flow, self.step, self.elapsed_ms, self.message
        )
    }

    /// Structured copy for the MCP error's `data`.
    pub fn to_json(&self) -> Value {
        json!({
            "flow": self.flow,
            "step": self.step,
            "error": self.message,
            "state": self.state,
            "elapsed_ms": self.elapsed_ms,
            "steps": self.steps,
        })
    }
}

impl From<FlowError> for String {
    fn from(e: FlowError) -> Self {
        e.summary()
    }
}

/// One run of a flow: the supervisor it drives and the step log.
pub struct FlowRun<'a> {
    sup: &'a Supervisor,
    flow: &'static str,
    started: Instant,
    steps: Vec<Value>,
}

impl<'a> FlowRun<'a> {
    pub fn new(sup: &'a Supervisor, flow: &'static str) -> Self {
        Self {
            sup,
            flow,
            started: Instant::now(),
            steps: Vec::new(),
        }
    }

    fn elapsed_ms(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }

    /// Log a finished step with its duration and optional detail.
    pub fn record(&mut self, step: &str, t0: Instant, detail: Value) {
        let mut s = json!({ "step": step, "ms": t0.elapsed().as_millis() as u64 });
        if !detail.is_null() {
            s["detail"] = detail;
        }
        self.steps.push(s);
    }

    /// A failure without a client-state read (for argument errors).
    pub fn fail(&self, step: &str, message: impl Into<String>) -> FlowError {
        FlowError(Box::new(FlowFailure {
            flow: self.flow,
            step: step.to_string(),
            message: message.into(),
            state: None,
            elapsed_ms: self.elapsed_ms(),
            steps: self.steps.clone(),
        }))
    }

    /// A failure that also records which screens were visible.
    pub async fn fail_with_state(&self, step: &str, message: impl Into<String>) -> FlowError {
        let mut e = self.fail(step, message);
        e.0.state = self.sup.screen_summary().await.ok();
        e
    }

    /// Run a Lua chunk and return its results.
    pub async fn lua(&self, step: &str, chunk: &str) -> Result<Vec<String>, FlowError> {
        match self.sup.lua_results(chunk).await {
            Ok(r) => Ok(r),
            Err(e) => Err(self.fail(step, e)),
        }
    }

    /// Is this window visible right now?
    pub async fn visible(&self, step: &str, window: &str) -> Result<bool, FlowError> {
        let r = self
            .lua(
                step,
                &format!("return tostring({})", widgets::visible(window)),
            )
            .await?;
        Ok(r.first().map(String::as_str) == Some("true"))
    }

    /// Click a named window like a player.
    pub async fn click(&mut self, step: &str, window: &str) -> Result<(), FlowError> {
        let t0 = Instant::now();
        if let Err(e) = self.sup.ui_click(window, 0).await {
            return Err(self
                .fail_with_state(step, format!("click {window}: {e}"))
                .await);
        }
        self.record(step, t0, json!({ "clicked": window }));
        Ok(())
    }

    /// Tap a key.
    pub async fn key(&mut self, step: &str, key: &str) -> Result<(), FlowError> {
        self.sup
            .input_key(key, "tap", None)
            .await
            .map(|_| ())
            .map_err(|e| self.fail(step, format!("key {key}: {e}")))
    }

    /// Click an edit box, clear it (End, then Backspace per character),
    /// type `text`, and read the box back. `secret` keeps the text out of
    /// the step log and the error.
    pub async fn type_into(
        &mut self,
        step: &str,
        window: &str,
        text: &str,
        secret: bool,
    ) -> Result<(), FlowError> {
        // Refuse untypeable text before touching the box.
        super::keys::plan_text(text).map_err(|e| {
            self.fail(
                step,
                format!("{window}: {e} (the lab types letters, digits, space and -_/.)"),
            )
        })?;
        self.click(step, window).await?;
        let t0 = Instant::now();
        let current = self.read_text(step, window).await?;
        if !current.is_empty() {
            self.key(step, "End").await?;
            for _ in 0..current.chars().count() {
                self.sup
                    .input_key("Backspace", "tap", Some(20))
                    .await
                    .map_err(|e| self.fail(step, format!("Backspace: {e}")))?;
            }
        }
        self.sup
            .type_text(text)
            .await
            .map_err(|e| self.fail(step, format!("type into {window}: {e}")))?;
        let got = self.read_text(step, window).await?;
        if got != text {
            let msg = if secret {
                format!(
                    "{window} holds {} characters after typing, expected {}",
                    got.chars().count(),
                    text.chars().count()
                )
            } else {
                format!("{window} reads {got:?} after typing {text:?}")
            };
            return Err(self.fail_with_state(step, msg).await);
        }
        let shown = if secret {
            json!("<hidden>")
        } else {
            json!(text)
        };
        self.record(step, t0, json!({ "typed_into": window, "text": shown }));
        Ok(())
    }

    /// The text of an edit box.
    pub async fn read_text(&self, step: &str, window: &str) -> Result<String, FlowError> {
        let r = self
            .lua(step, &format!("return {}", widgets::text_of(window)))
            .await?;
        Ok(r.into_iter().next().unwrap_or_default())
    }

    /// Poll the Lua condition `cond` until it holds. When `fail` is given
    /// and evaluates to a non-nil value first, the wait ends with that
    /// value as the error (a prompt's text, typically).
    pub async fn wait(
        &mut self,
        step: &str,
        cond: &str,
        fail: Option<&str>,
        timeout: Duration,
    ) -> Result<u64, FlowError> {
        let t0 = Instant::now();
        let outcome = self.sup.poll_until(cond, fail, timeout, POLL).await;
        match outcome {
            Ok(w) if w.met => {
                self.record(step, t0, Value::Null);
                Ok(w.elapsed_ms)
            }
            Ok(w) => {
                let why = match (w.fail_text, w.last_error) {
                    (Some(f), _) => format!("the client reported: {f}"),
                    (None, Some(e)) => {
                        format!("timed out after {} ms (last Lua error: {e})", w.elapsed_ms)
                    }
                    (None, None) => format!("timed out after {} ms", w.elapsed_ms),
                };
                Err(self
                    .fail_with_state(step, format!("waiting for {cond}: {why}"))
                    .await)
            }
            Err(e) => Err(self.fail(step, e)),
        }
    }

    /// Wrap the flow's result with the total time and the step log.
    pub fn finish(self, mut out: Value) -> Value {
        out["elapsed_ms"] = json!(self.elapsed_ms());
        out["steps"] = json!(self.steps);
        out
    }
}

/// Sleep helper for the short settles between clicks the UI needs (a
/// frame or two for a selection to apply).
pub async fn settle(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms)).await;
}
