//! Click a ring destination, with an explicit Lua fallback when the map
//! image consumes the mouse event before its parent handler sees it.

use std::time::Duration;

use serde_json::{json, Value};

use super::{NativeLevel, NativeTrail, Supervisor};
use crate::supervisor::input::MouseInput;

const PREFLIGHT: &str = r#"
if WorldMapWin == nil or WorldMapImageWin == nil or WorldMapMod == nil or
   RingTransporterWorldMapMode == nil or not WorldMapWin:isVisible() or
   WorldMapMod.currentMode ~= RingTransporterWorldMapMode then
  return __jenc({ ready = false, reason = 'ring destination map is not open' })
end
local rect = WorldMapImageWin:getUnclippedPixelRect()
return __jenc({ ready = true, rect = { rect.left, rect.top, rect.right, rect.bottom } })
"#;

const CLOSED: &str =
    "return __jenc({ closed = WorldMapWin ~= nil and not WorldMapWin:isVisible() })";

fn fallback_chunk(x: i32, y: i32) -> String {
    format!(
        "local ok, err = pcall(WorldMapMod.onClick, nil, WorldMapWin, CEGUI.Vector2({x},{y})) \
         return __jenc({{ invoked = ok, error = not ok and tostring(err) or nil }})"
    )
}

fn validate_point(rect: &Value, x: i32, y: i32) -> Result<(), String> {
    let rect = rect
        .as_array()
        .filter(|r| r.len() == 4)
        .ok_or("world map image has no screen rectangle")?;
    let edge = |i: usize| rect.get(i).and_then(Value::as_f64).unwrap_or(f64::NAN);
    if !(f64::from(x) >= edge(0)
        && f64::from(x) < edge(2)
        && f64::from(y) >= edge(1)
        && f64::from(y) < edge(3))
    {
        return Err(format!(
            "click ({x},{y}) is outside the world map image [{:.0},{:.0},{:.0},{:.0}]",
            edge(0),
            edge(1),
            edge(2),
            edge(3)
        ));
    }
    Ok(())
}

fn click_trail(fallback: bool) -> NativeTrail {
    let mut trail = NativeTrail::default();
    trail.push("click world map pixel", NativeLevel::RealInput);
    if fallback {
        trail.push("call stock WorldMapMod.onClick", NativeLevel::ClientUiLua);
    }
    trail
}

impl Supervisor {
    /// Click one map pixel with posted mouse messages. If the map remains
    /// open and `allow_fallback`, invoke its stock Lua handler at that pixel.
    pub async fn world_map_click(
        &self,
        x: i32,
        y: i32,
        allow_fallback: bool,
    ) -> Result<Value, String> {
        let setup = self.lua_json(PREFLIGHT).await?;
        if setup["ready"] != json!(true) {
            return Err(setup["reason"]
                .as_str()
                .unwrap_or("ring destination map is not open")
                .to_string());
        }
        validate_point(&setup["rect"], x, y)?;

        let click = self
            .input_mouse(MouseInput {
                button: Some(0),
                at: Some((x, y)),
                ..MouseInput::default()
            })
            .await?;
        tokio::time::sleep(Duration::from_millis(200)).await;
        let mut closed = self.lua_json(CLOSED).await?["closed"] == json!(true);
        let mut fallback = false;
        if !closed && allow_fallback {
            let result = self.lua_json(&fallback_chunk(x, y)).await?;
            if result["invoked"] != json!(true) {
                return Err(format!(
                    "WorldMapMod.onClick fallback failed: {}",
                    result["error"].as_str().unwrap_or("unknown Lua error")
                ));
            }
            fallback = true;
            tokio::time::sleep(Duration::from_millis(200)).await;
            closed = self.lua_json(CLOSED).await?["closed"] == json!(true);
        }
        let mut result = json!({
            "at": [x, y],
            "map_closed": closed,
            "effect_ok": closed,
            "fallback": fallback,
            "mouse": click,
        });
        click_trail(fallback).stamp(&mut result);
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_map_click_rejects_pixels_outside_the_image() {
        let rect = json!([25, 120, 1255, 675]);
        assert!(validate_point(&rect, 652, 398).is_ok());
        assert!(validate_point(&rect, 24, 398).is_err());
        assert!(validate_point(&rect, 1255, 398).is_err());
        assert!(validate_point(&rect, 652, 675).is_err());
        assert!(validate_point(&json!([25, 120]), 652, 398).is_err());
    }

    #[test]
    fn assisted_ring_selection_cannot_be_graded_as_native() {
        let mut native = json!({});
        click_trail(false).stamp(&mut native);
        assert_eq!(native["native_tier"], "N1");
        assert_eq!(native["native_pass"], true);

        let mut assisted = json!({});
        click_trail(true).stamp(&mut assisted);
        assert_eq!(assisted["native_tier"], "N3");
        assert_eq!(assisted["native_pass"], false);
        assert_eq!(assisted["native_level"], "client_ui_lua");
    }
}
