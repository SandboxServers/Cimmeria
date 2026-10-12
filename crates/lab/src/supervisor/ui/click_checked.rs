//! Clicking a window until its effect shows, escalating through the lab's
//! input paths and saying which one worked.
//!
//! A posted button message can be lost on one window and work on the next
//! (#1660: a click on `Inventory_GeneralTabInactive` registered as native
//! input and changed nothing). So a click whose effect can be read back is
//! tried in order, each step followed by the readback:
//!
//! 1. `ui_click`: native cursor move, posted button messages (N1);
//! 2. the same point through CEGUI's own button injectors (N1);
//! 3. the button's own stock `EventClicked` handler called from Lua (N3),
//!    only when the caller names one.
//!
//! The trail records the step that worked, so a tab opened by step 3 makes
//! the surrounding tool N3 and says why.

use std::time::Duration;

use serde_json::json;

use super::slots::lua_path;
use super::window_click::{click_point, locate_chunk, ClickTarget};
use super::{NativeLevel, NativeTrail, Supervisor};
use crate::supervisor::flows::widgets::lua_quote;

/// A frame or two for the client to apply a click.
const SETTLE: Duration = Duration::from_millis(200);

/// Lua body for the readback "is window `name` visible": `{ done }`.
pub fn visible_chunk(name: &str) -> String {
    format!(
        "local w = _G[{n}] if w == nil then w = __jcall(getWindow, {n}) end \
         return __jenc({{ done = w ~= nil and __jcall(function() return w:isVisible() end) == true }})",
        n = lua_quote(name)
    )
}

/// Lua body that calls a stock click handler (`InventoryMod.onGeneralClicked`)
/// with the button, as the button's `EventClicked` would: `{ ok }`.
pub fn handler_chunk(handler: &str, window: &str) -> String {
    format!(
        "local f = {lookup} local w = _G[{w}] \
         local ok = type(f) == 'function' and w ~= nil and pcall(f, w) \
         return __jenc({{ ok = ok == true }})",
        lookup = lua_path(handler),
        w = lua_quote(window)
    )
}

impl Supervisor {
    async fn click_took_effect(&self, done: &str) -> Result<bool, String> {
        tokio::time::sleep(SETTLE).await;
        Ok(self.lua_json(done).await?["done"] == json!(true))
    }

    /// Click `window` until the readback `done` (a Lua body returning
    /// `{ done = <bool> }`) says it worked. `handler` is the stock Lua
    /// handler to call as the last resort. Errors name every attempt.
    pub async fn click_until(
        &self,
        window: &str,
        done: &str,
        handler: Option<&str>,
        trail: &mut NativeTrail,
    ) -> Result<(), String> {
        let mut tried: Vec<String> = Vec::new();
        match self.ui_click(window, 0).await {
            Ok(_) => {
                if self.click_took_effect(done).await? {
                    trail.push(format!("click {window}"), NativeLevel::RealInput);
                    return Ok(());
                }
                tried.push("posted click: no effect".to_string());
            }
            Err(e) => tried.push(format!("posted click: {e}")),
        }
        let loc = self
            .lua_json(&locate_chunk(&ClickTarget::Named(window.to_string())))
            .await?;
        match click_point(&loc) {
            Some((x, y)) => match self.click_at_native(x, y, 0, false).await {
                Ok(placed) => {
                    if self.click_took_effect(done).await? {
                        placed.note_fallback(trail);
                        trail.push(
                            format!("click {window} (CEGUI injectors)"),
                            NativeLevel::NativeCegui,
                        );
                        return Ok(());
                    }
                    tried.push("injected click: no effect".to_string());
                }
                Err(e) => tried.push(format!("injected click: {e}")),
            },
            None => tried.push(format!("{window} has no screen rectangle")),
        }
        if let Some(h) = handler {
            let r = self.lua_json(&handler_chunk(h, window)).await?;
            if r["ok"] == json!(true) && self.click_took_effect(done).await? {
                trail.push(format!("call {h}"), NativeLevel::ClientUiLua);
                return Ok(());
            }
            tried.push(format!("{h}: no effect"));
        }
        Err(format!(
            "{window} was clicked but the expected change did not show ({})",
            tried.join("; ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::supervisor::cegui_fake::{self, FakeCegui};
    use crate::supervisor::events::fake_bridge;

    async fn run(
        state: FakeCegui,
        handler: Option<&str>,
    ) -> (Result<(), String>, NativeTrail, Arc<Mutex<FakeCegui>>) {
        let s = Arc::new(Mutex::new(state));
        let sup = fake_bridge::supervisor(cegui_fake::responder(s.clone())).await;
        let mut trail = NativeTrail::default();
        let r = sup
            .click_until("TabBtn", &visible_chunk("TabActive"), handler, &mut trail)
            .await;
        (r, trail, s)
    }

    /// The posted click is unavailable in the fake (no game window), the
    /// injected one works: N1, the handler is never called.
    #[tokio::test]
    async fn an_injected_click_is_the_second_step_and_stays_n1() {
        let (r, trail, s) = run(
            FakeCegui {
                tab_click_works: true,
                ..FakeCegui::default()
            },
            Some("InventoryMod.onGeneralClicked"),
        )
        .await;
        r.unwrap();
        assert_eq!(trail.overall(), Some(NativeLevel::NativeCegui));
        let s = s.lock().unwrap();
        assert_eq!(s.edges(), vec!["down 0", "up 0"]);
        assert_eq!(s.tab_handler_calls, 0);
    }

    /// Neither click registers: the stock handler finishes the job and the
    /// trail says N3, so a drag that needed it is not a native pass.
    #[tokio::test]
    async fn the_stock_handler_is_the_last_resort_and_is_n3() {
        let (r, trail, s) = run(FakeCegui::default(), Some("InventoryMod.onGeneralClicked")).await;
        r.unwrap();
        assert_eq!(trail.overall(), Some(NativeLevel::ClientUiLua));
        assert_eq!(s.lock().unwrap().tab_handler_calls, 1);
    }

    /// No step works and there is no handler: an error that names the
    /// attempts, not five silent retries.
    #[tokio::test]
    async fn a_click_that_never_takes_effect_is_an_error_naming_the_attempts() {
        let (r, _, _) = run(FakeCegui::default(), None).await;
        let e = r.unwrap_err();
        assert!(e.contains("did not show"), "{e}");
        assert!(e.contains("injected click: no effect"), "{e}");
    }
}
