//! `client_vendor_buy`: set a vendor row's quantity and press Buy.
//!
//! A vendor row's quantity spinner (`Vendor_Item<N>Quantity`, a CEGUI
//! `Spinner`) starts at 0, and its value is what `vendorBuySelected` buys:
//! `VendorMod.onSpinnerValueChanged` (`Vendor.lua:30-41`) calls
//! `setVendorSelectedQuantity(row + scroll - 1, value)` on every change. So
//! the native way to set a quantity is to click the spinner's own
//! `__auto_incbtn__` / `__auto_decbtn__` until the value is right
//! ([`Supervisor::click_until`] with the value as the readback), then click
//! `Vendor_BuyButton`. Only when the buttons cannot do it (and
//! `allow_fallback`) the value is set through the spinner's Lua
//! (`setCurrentValue`, which fires the same handler) or the
//! `setVendorSelectedQuantity` binding, and the result is N3.

use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::click_checked::handler_chunk;
use super::window_click::{click_point, locate_chunk, ClickTarget};
use super::{NativeLevel, NativeTrail, Supervisor};
use crate::supervisor::flows::widgets::lua_quote;

/// Rows on screen in the stock vendor window.
pub const MAX_ROWS: u32 = 7;
/// Most spinner clicks per call; a larger quantity needs the Lua fallback.
pub const MAX_CLICKS: i64 = 30;
const BUY_BUTTON: &str = "Vendor_BuyButton";

/// The CEGUI name of row `row`'s quantity spinner.
pub fn spinner_name(row: u32) -> String {
    format!("Vendor_Item{row}Quantity")
}

/// The spinner's increment button.
pub fn inc_button(row: u32) -> String {
    format!("{}__auto_incbtn__", spinner_name(row))
}

/// The spinner's decrement button.
pub fn dec_button(row: u32) -> String {
    format!("{}__auto_decbtn__", spinner_name(row))
}

/// Lua body: `{ vendor_open, buy_visible, row_visible, current, max, scroll }`.
pub fn state_chunk(row: u32) -> String {
    format!(
        "local sp = _G[{sp}] \
         local scroll = __jcall(function() return math.floor(Vendor_Scrollbar:getProperty('ScrollPosition')) end) or 0 \
         local buy = _G['Vendor_BuyButton'] \
         return __jenc({{ vendor_open = VendorWin ~= nil and __jcall(function() return VendorWin:isVisible() end) == true, \
           buy_visible = buy ~= nil and __jcall(function() return buy:isVisible() end) == true, \
           row_visible = sp ~= nil and __jcall(function() return sp:isVisible() end) == true, \
           current = sp and __jcall(function() return tonumber(sp:getCurrentValue()) end), \
           max = sp and __jcall(function() return tonumber(sp:getMaximumValue()) end), \
           scroll = scroll }})",
        sp = lua_quote(&spinner_name(row))
    )
}

/// Lua body: `{ done }`, true when the row's spinner reads `want`.
pub fn value_is_chunk(row: u32, want: i64) -> String {
    format!(
        "local sp = _G[{sp}] \
         local cur = sp and __jcall(function() return tonumber(sp:getCurrentValue()) end) \
         return __jenc({{ done = cur == {want} }})",
        sp = lua_quote(&spinner_name(row))
    )
}

/// Lua body: set the quantity without the buttons. The spinner's
/// `setCurrentValue` fires the stock handler; if the value still differs,
/// the `setVendorSelectedQuantity(abs0, n)` binding is called directly.
/// `{ ok, via, current }`.
pub fn set_quantity_chunk(row: u32, abs0: i64, n: i64) -> String {
    format!(
        "local sp = _G[{sp}] \
         if sp == nil then return __jenc({{ ok = false }}) end \
         pcall(function() sp:setCurrentValue({n}) end) \
         local cur = __jcall(function() return tonumber(sp:getCurrentValue()) end) \
         if cur == {n} then return __jenc({{ ok = true, via = 'spinner', current = cur }}) end \
         local ok = pcall(setVendorSelectedQuantity, {abs0}, {n}) \
         cur = __jcall(function() return tonumber(sp:getCurrentValue()) end) \
         return __jenc({{ ok = ok, via = 'setVendorSelectedQuantity', current = cur }})",
        sp = lua_quote(&spinner_name(row))
    )
}

impl Supervisor {
    /// Click Buy and wait for the inventory or cash to change: posted
    /// click, then the injectors, then (allow_fallback) the stock handler.
    /// The last value is `double_buy_risk`: a later Buy step fired after an
    /// earlier click was sent but showed no change within `wait`, so a slow
    /// server may land both (same as item use, D-LH1 / D-LH14).
    async fn press_buy(
        &self,
        before: &Value,
        allow_fallback: bool,
        wait: Duration,
        trail: &mut NativeTrail,
    ) -> Result<(Value, Value, bool), String> {
        // Buy attempts sent so far (a click that went out, or the handler).
        let mut sent = 0u32;
        if self.ui_click(BUY_BUTTON, 0).await.is_ok() {
            sent += 1;
            let (a, d) = self.await_inventory_change(before, wait).await?;
            if d["changed"] == json!(true) {
                trail.push(format!("click {BUY_BUTTON}"), NativeLevel::RealInput);
                return Ok((a, d, false));
            }
        }
        let loc = self
            .lua_json(&locate_chunk(&ClickTarget::Named(BUY_BUTTON.to_string())))
            .await?;
        if let Some((x, y)) = click_point(&loc) {
            if self.click_at_native(x, y, 0, false).await.is_ok() {
                sent += 1;
                let (a, d) = self.await_inventory_change(before, wait).await?;
                if d["changed"] == json!(true) {
                    trail.push(
                        format!("click {BUY_BUTTON} (CEGUI injectors)"),
                        NativeLevel::NativeCegui,
                    );
                    return Ok((a, d, sent > 1));
                }
            }
        }
        if allow_fallback {
            let r = self
                .lua_json(&handler_chunk("VendorMod.onBuyClicked", BUY_BUTTON))
                .await?;
            if r["ok"] == json!(true) {
                sent += 1;
                trail.push("VendorMod.onBuyClicked via Lua", NativeLevel::ClientUiLua);
            }
        }
        let (a, d) = self.await_inventory_change(before, wait).await?;
        Ok((a, d, sent > 1))
    }

    /// `client_vendor_buy`: set row `row` (1-based, as shown) to `quantity`
    /// and buy it.
    pub async fn vendor_buy(
        &self,
        row: u32,
        quantity: u32,
        allow_fallback: bool,
        expect_change: bool,
        wait: Duration,
    ) -> Result<Value, String> {
        let t0 = Instant::now();
        if !(1..=MAX_ROWS).contains(&row) {
            return Err(format!(
                "row must be 1..={MAX_ROWS} (the rows on screen), not {row}"
            ));
        }
        if quantity == 0 {
            return Err("quantity must be at least 1".to_string());
        }
        let want = i64::from(quantity);
        let mut trail = NativeTrail::default();
        let st = self.lua_json(&state_chunk(row)).await?;
        if st["vendor_open"] != json!(true) {
            return Err("the vendor window is not open (open it at a vendor first)".to_string());
        }
        if st["buy_visible"] != json!(true) {
            return Err("the Buy tab is not active (click Vendor_BuyTabInactive)".to_string());
        }
        if st["row_visible"] != json!(true) {
            return Err(format!(
                "row {row} shows no item (its quantity spinner is hidden)"
            ));
        }
        if let Some(max) = st["max"].as_i64() {
            if want > max {
                return Err(format!("row {row} allows at most {max}, not {want}"));
            }
        }
        let before = self.inventory_read(&[], false).await?;
        let start = st["current"].as_i64().unwrap_or(0);
        let mut cur = start;
        let mut clicks = 0i64;
        let mut click_error: Option<String> = None;
        while cur != want && clicks < MAX_CLICKS {
            let (button, next) = if cur < want {
                (inc_button(row), cur + 1)
            } else {
                (dec_button(row), cur - 1)
            };
            match self
                .click_until(&button, &value_is_chunk(row, next), None, &mut trail)
                .await
            {
                Ok(()) => {
                    cur = next;
                    clicks += 1;
                }
                Err(e) => {
                    click_error = Some(e);
                    break;
                }
            }
        }
        let mut quantity_via = "spinner_buttons";
        if cur != want {
            if !allow_fallback {
                return Err(format!(
                    "the quantity spinner stopped at {cur}, not {want}: {}",
                    click_error.unwrap_or_else(|| format!("more than {MAX_CLICKS} clicks needed"))
                ));
            }
            let abs0 = st["scroll"].as_i64().unwrap_or(0) + i64::from(row) - 1;
            let r = self.lua_json(&set_quantity_chunk(row, abs0, want)).await?;
            trail.push(
                format!(
                    "set quantity {want} via {}",
                    r["via"].as_str().unwrap_or("Lua")
                ),
                NativeLevel::ClientUiLua,
            );
            cur = r["current"].as_i64().unwrap_or(-1);
            if r["ok"] != json!(true) || cur != want {
                return Err(format!(
                    "the quantity could not be set to {want} (reads {cur})"
                ));
            }
            quantity_via = "lua";
        }
        let (after, d, double_buy_risk) = self
            .press_buy(&before, allow_fallback, wait, &mut trail)
            .await?;
        // Bought means the items arrived: some item's total went up and
        // the cash did not. A cash-only or slot-shuffle change is not a buy.
        let gained: i64 = d["by_item"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|i| i["delta"].as_i64())
                    .filter(|x| *x > 0)
                    .sum()
            })
            .unwrap_or(0);
        let cash_ok = d["cash_delta"].as_i64().is_none_or(|c| c <= 0);
        let bought = gained > 0 && cash_ok;
        let mut out = json!({
            "items_gained": gained,
            "quantity_matched": gained == want,
            "row": row,
            "quantity": want,
            "spinner_start": start,
            "spinner_clicks": clicks,
            "spinner_final": cur,
            "quantity_via": quantity_via,
            "bought": bought,
            "double_buy_risk": double_buy_risk,
            "inventory_changed": d["changed"],
            "diff": d,
            "cash_after": after["cash"],
        });
        trail.stamp(&mut out);
        if expect_change {
            let ok = bought && gained == want;
            out["effect_ok"] = json!(ok);
            if !ok {
                out["native_pass"] = json!(false);
            }
        }
        out["elapsed_ms"] = json!(t0.elapsed().as_millis() as u64);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::supervisor::cegui_fake::{self, FakeCegui};
    use crate::supervisor::events::fake_bridge;

    async fn buy(
        state: FakeCegui,
        quantity: u32,
        allow_fallback: bool,
    ) -> (Result<Value, String>, Arc<Mutex<FakeCegui>>) {
        let s = Arc::new(Mutex::new(state));
        let sup = fake_bridge::supervisor(cegui_fake::responder(s.clone())).await;
        let out = sup
            .vendor_buy(1, quantity, allow_fallback, true, Duration::ZERO)
            .await;
        (out, s)
    }

    #[test]
    fn names_follow_the_stock_layout() {
        assert_eq!(spinner_name(3), "Vendor_Item3Quantity");
        assert_eq!(inc_button(3), "Vendor_Item3Quantity__auto_incbtn__");
        assert_eq!(dec_button(3), "Vendor_Item3Quantity__auto_decbtn__");
        let c = set_quantity_chunk(2, 5, 4);
        assert!(c.contains("sp:setCurrentValue(4)"));
        assert!(c.contains("pcall(setVendorSelectedQuantity, 5, 4)"));
    }

    /// The spinner's increment button is clicked until the value is right,
    /// then Buy: every step is an injected click (the posted path is not
    /// available in the fake), so N1 and a native pass.
    #[tokio::test]
    async fn the_spinner_buttons_set_the_quantity_at_n1() {
        let (out, s) = buy(
            FakeCegui {
                inc_works: true,
                ..FakeCegui::default()
            },
            2,
            true,
        )
        .await;
        let out = out.unwrap();
        assert_eq!(out["spinner_clicks"], 2);
        assert_eq!(out["spinner_final"], 2);
        assert_eq!(out["quantity_via"], "spinner_buttons");
        assert_eq!(out["bought"], true);
        assert_eq!(out["effect_ok"], true);
        assert_eq!(out["native_level"], "native_cegui");
        assert_eq!(out["native_pass"], true);
        let s = s.lock().unwrap();
        assert_eq!(s.lua_set_calls, 0);
        assert_eq!(
            s.edges(),
            vec!["down 0", "up 0", "down 0", "up 0", "down 0", "up 0"]
        );
    }

    /// The buttons do nothing: the Lua setter finishes the job and the
    /// result is N3, never a native pass.
    #[tokio::test]
    async fn the_lua_setter_is_the_fallback_and_is_n3() {
        let (out, s) = buy(FakeCegui::default(), 2, true).await;
        let out = out.unwrap();
        assert_eq!(out["quantity_via"], "lua");
        assert_eq!(out["bought"], true);
        assert_eq!(out["native_level"], "client_ui_lua");
        assert_eq!(out["native_pass"], false);
        assert_eq!(s.lock().unwrap().lua_set_calls, 1);
    }

    /// Without the fallback a spinner that will not move is an error that
    /// names where it stopped; nothing is bought.
    #[tokio::test]
    async fn without_fallback_a_stuck_spinner_is_an_error() {
        let (out, s) = buy(FakeCegui::default(), 2, false).await;
        let e = out.unwrap_err();
        assert!(e.contains("stopped at 0, not 2"), "{e}");
        assert!(!s.lock().unwrap().bought);
    }

    /// Above the row's maximum is refused before any click.
    #[tokio::test]
    async fn a_quantity_above_the_row_maximum_is_refused() {
        let (out, s) = buy(FakeCegui::default(), 11, true).await;
        assert!(out.unwrap_err().contains("allows at most 10"));
        assert!(s.lock().unwrap().edges().is_empty());
    }

    /// Nothing bought (the quantity was never set, so Buy does nothing):
    /// effect_ok and native_pass are false, not an error and not a pass.
    #[tokio::test]
    async fn a_buy_that_changes_nothing_fails_the_effect_check() {
        let s = Arc::new(Mutex::new(FakeCegui::default()));
        let sup = fake_bridge::supervisor(cegui_fake::responder(s.clone())).await;
        let mut trail = NativeTrail::default();
        let before = sup.inventory_read(&[], false).await.unwrap();
        let (after, d, risk) = sup
            .press_buy(&before, false, Duration::ZERO, &mut trail)
            .await
            .unwrap();
        assert_eq!(d["changed"], false);
        assert!(!risk, "one silent click and no retry is not a double buy");
        assert_eq!(after["cash"], 0);
        assert!(!s.lock().unwrap().bought);
    }

    /// D-LH14: the injected Buy click is lost (no change within the wait), so
    /// the stock handler runs as well. The result must carry
    /// `double_buy_risk`; a clean first-click buy must not.
    #[tokio::test]
    async fn a_buy_retry_after_a_silent_click_reports_double_buy_risk() {
        let (out, s) = buy(
            FakeCegui {
                vendor_value: 1,
                buy_swallow: 1,
                ..FakeCegui::default()
            },
            1,
            true,
        )
        .await;
        let out = out.unwrap();
        assert_eq!(out["bought"], true);
        assert_eq!(s.lock().unwrap().lua_buy_calls, 1);
        assert_eq!(out["double_buy_risk"], true);

        let (clean, _) = buy(
            FakeCegui {
                inc_works: true,
                ..FakeCegui::default()
            },
            1,
            true,
        )
        .await;
        assert_eq!(clean.unwrap()["double_buy_risk"], false);
    }

    #[tokio::test]
    async fn a_zero_quantity_is_refused() {
        let (out, _) = buy(FakeCegui::default(), 0, true).await;
        assert!(out.unwrap_err().contains("at least 1"));
    }
}
