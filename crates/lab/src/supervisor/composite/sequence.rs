//! `client_ui_sequence`: one scripted UI step (open a bag, click an NPC's
//! greet topic, press a dialog button, wait for a window) as one call.
//!
//! Actions: `click` (`window`, `button` 0/1), `key` (`key`, `action`
//! tap/down/up, `hold_ms`), `type` (`text`, optional `into` edit box to
//! click first), `drag` (`from` / `to` = `{container, slot}` or
//! `{window}`, `split`), `wait_window` (`window`, `gone`, `timeout_ms`),
//! `wait` (`ms`). Each runs through the same supervisor path as its
//! single tool (`client_ui_click`, `client_input_key`, `client_type_text`,
//! `client_drag_drop`), so each is real input or the client's own CEGUI
//! injectors and carries that native level; the result stamps the least
//! native one, like any other UI tool.

use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::supervisor::ui::drag_drop::DragEnd;
use crate::supervisor::ui::slots::ContainerRef;
use crate::supervisor::ui::{NativeLevel, NativeTrail};
use crate::supervisor::Supervisor;

/// Most actions one sequence may hold.
pub const MAX_ACTIONS: usize = 32;
const DEFAULT_WINDOW_WAIT: Duration = Duration::from_secs(10);
const DRAG_STEPS: u32 = 8;
const DRAG_WAIT: Duration = Duration::from_secs(3);

/// One UI action.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Click {
        window: String,
        button: usize,
    },
    Key {
        key: String,
        action: String,
        hold_ms: Option<u64>,
    },
    Type {
        text: String,
        into: Option<String>,
    },
    Drag {
        from: DragEnd,
        to: DragEnd,
        split: bool,
    },
    WaitWindow {
        window: String,
        gone: bool,
        timeout: Duration,
    },
    Wait {
        ms: u64,
    },
}

fn s(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(Value::as_str).map(str::to_string)
}

fn drag_end(v: Option<&Value>, which: &str) -> Result<DragEnd, String> {
    let v = v.ok_or_else(|| format!("drag needs `{which}`"))?;
    match (
        s(v, "container"),
        v.get("slot").and_then(Value::as_u64),
        s(v, "window"),
    ) {
        (Some(c), Some(slot), None) => Ok(DragEnd::Slot {
            container: ContainerRef::parse(&c),
            slot: slot as u32,
        }),
        (None, None, Some(w)) => Ok(DragEnd::Window(w)),
        _ => Err(format!("`{which}` needs container + slot, or window")),
    }
}

/// Parse one action object (`{"do": "click", ...}`; `op` works too).
pub fn parse_action(v: &Value) -> Result<Action, String> {
    let kind = s(v, "do").or_else(|| s(v, "op")).ok_or("no `do`")?;
    let need = |k: &str| s(v, k).ok_or_else(|| format!("{kind} needs `{k}`"));
    Ok(match kind.as_str() {
        "click" => Action::Click {
            window: need("window")?,
            button: v.get("button").and_then(Value::as_u64).unwrap_or(0) as usize,
        },
        "key" => Action::Key {
            key: need("key")?,
            action: s(v, "action").unwrap_or_else(|| "tap".into()),
            hold_ms: v.get("hold_ms").and_then(Value::as_u64),
        },
        "type" => Action::Type {
            text: need("text")?,
            into: s(v, "into"),
        },
        "drag" => Action::Drag {
            from: drag_end(v.get("from"), "from")?,
            to: drag_end(v.get("to"), "to")?,
            split: v.get("split").and_then(Value::as_bool).unwrap_or(false),
        },
        "wait_window" => Action::WaitWindow {
            window: need("window")?,
            gone: v.get("gone").and_then(Value::as_bool).unwrap_or(false),
            timeout: v
                .get("timeout_ms")
                .and_then(Value::as_u64)
                .map(|ms| Duration::from_millis(ms.min(120_000)))
                .unwrap_or(DEFAULT_WINDOW_WAIT),
        },
        "wait" => Action::Wait {
            ms: v
                .get("ms")
                .and_then(Value::as_u64)
                .ok_or("wait needs `ms`")?,
        },
        other => {
            return Err(format!(
                "unknown action {other:?} (click, key, type, drag, wait_window, wait)"
            ))
        }
    })
}

/// Parse the `actions` array.
pub fn parse_actions(raw: &[Value]) -> Result<Vec<Action>, String> {
    if raw.is_empty() {
        return Err("no actions".into());
    }
    if raw.len() > MAX_ACTIONS {
        return Err(format!("{} actions; at most {MAX_ACTIONS}", raw.len()));
    }
    raw.iter()
        .enumerate()
        .map(|(i, a)| parse_action(a).map_err(|e| format!("action {}: {e}", i + 1)))
        .collect()
}

/// Whether a `client_drag_drop` result moved the item: `moved` true and
/// `effect_ok` not false.
pub fn drag_moved(r: &Value) -> bool {
    r["moved"] == json!(true) && r["effect_ok"] != json!(false)
}

/// The native level a result reports, else the level the action implies.
fn level_of(result: &Value, default: NativeLevel) -> NativeLevel {
    match result["native_level"].as_str() {
        Some("real_input") => NativeLevel::RealInput,
        Some("native_cegui") => NativeLevel::NativeCegui,
        Some("slash_command") => NativeLevel::SlashCommand,
        Some("client_ui_lua") => NativeLevel::ClientUiLua,
        Some("native_call") => NativeLevel::NativeCall,
        _ => default,
    }
}

impl Supervisor {
    /// Run one action: its short result and the native level it drove at
    /// (`None` for the waits, which drive nothing).
    async fn ui_action(&self, a: &Action) -> Result<(Value, Option<NativeLevel>), String> {
        match a {
            Action::Click { window, button } => {
                let r = self.ui_click(window, *button).await?;
                let l = level_of(&r, NativeLevel::RealInput);
                Ok((json!(true), Some(l)))
            }
            Action::Key {
                key,
                action,
                hold_ms,
            } => {
                self.input_key(key, action, *hold_ms).await?;
                Ok((json!(true), Some(NativeLevel::RealInput)))
            }
            Action::Type { text, into } => {
                let mut level = NativeLevel::RealInput;
                if let Some(w) = into {
                    let r = self.ui_click(w, 0).await?;
                    level = level.max(level_of(&r, NativeLevel::RealInput));
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                self.type_text(text).await?;
                Ok((json!(true), Some(level)))
            }
            Action::Drag { from, to, split } => {
                let r = self
                    .drag_drop(from, to, *split, DRAG_STEPS, true, DRAG_WAIT)
                    .await?;
                // A drag that moved nothing is a failed step, not a pass
                // (review of #1309; the UAT runner fails effect_ok false).
                if !drag_moved(&r) {
                    return Err(format!(
                        "the drag moved nothing (drag_started {}, drop_notified {}, snap_back {})",
                        r["drag_started"], r["drop_notified"], r["snap_back"]
                    ));
                }
                let l = level_of(&r, NativeLevel::NativeCegui);
                Ok((
                    json!({ "moved": true, "native_level": r["native_level"] }),
                    Some(l),
                ))
            }
            Action::WaitWindow {
                window,
                gone,
                timeout,
            } => {
                let ms = self.wait_window(window, *gone, *timeout).await?;
                Ok((json!({ "ms": ms }), None))
            }
            Action::Wait { ms } => {
                self.idle(Duration::from_millis((*ms).min(30_000))).await?;
                Ok((Value::Null, None))
            }
        }
    }

    /// `client_ui_sequence`: run `actions` in order.
    pub async fn ui_sequence(&self, actions: &[Action], stop_on_error: bool) -> Value {
        let t0 = Instant::now();
        let mut trail = NativeTrail::default();
        let mut results = Vec::with_capacity(actions.len());
        let mut stopped_at = None;
        for (i, a) in actions.iter().enumerate() {
            match self.ui_action(a).await {
                Ok((v, level)) => {
                    if let Some(l) = level {
                        trail.push(format!("{}: {a:?}", i + 1), l);
                    }
                    results.push(v);
                }
                Err(e) => {
                    results.push(json!({ "error": e }));
                    if stop_on_error {
                        stopped_at = Some(i + 1);
                        break;
                    }
                }
            }
        }
        let mut out = json!({ "results": results, "ms": t0.elapsed().as_millis() as u64 });
        if let Some(i) = stopped_at {
            out["stopped_at"] = json!(i);
        }
        trail.stamp(&mut out);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actions_parse_with_defaults() {
        let a = parse_actions(&[
            json!({ "do": "key", "key": "B" }),
            json!({ "do": "click", "window": "Dialog_Done" }),
            json!({ "do": "type", "text": "/who", "into": "Inst1Chat_Input" }),
            json!({ "do": "wait_window", "window": "InventoryWin" }),
            json!({ "do": "drag", "from": { "container": "Main", "slot": 1 },
                    "to": { "window": "Trade_LocalSlot1" } }),
            json!({ "op": "wait", "ms": 200 }),
        ])
        .unwrap();
        assert_eq!(
            a[0],
            Action::Key {
                key: "B".into(),
                action: "tap".into(),
                hold_ms: None
            }
        );
        assert_eq!(
            a[1],
            Action::Click {
                window: "Dialog_Done".into(),
                button: 0
            }
        );
        assert!(matches!(&a[2], Action::Type { into: Some(w), .. } if w == "Inst1Chat_Input"));
        assert_eq!(
            a[3],
            Action::WaitWindow {
                window: "InventoryWin".into(),
                gone: false,
                timeout: DEFAULT_WINDOW_WAIT
            }
        );
        assert!(matches!(
            &a[4],
            Action::Drag {
                from: DragEnd::Slot { slot: 1, .. },
                to: DragEnd::Window(_),
                split: false
            }
        ));
        assert_eq!(a[5], Action::Wait { ms: 200 });
    }

    #[test]
    fn bad_actions_name_their_position() {
        let e = parse_actions(&[json!({ "do": "key", "key": "B" }), json!({ "do": "click" })])
            .unwrap_err();
        assert!(e.starts_with("action 2:") && e.contains("window"), "{e}");
        assert!(parse_actions(&[json!({ "do": "fly" })])
            .unwrap_err()
            .contains("unknown action"));
        assert!(parse_actions(&[]).is_err());
        let half =
            json!({ "do": "drag", "from": { "container": "Main" }, "to": { "window": "W" } });
        assert!(parse_actions(&[half]).unwrap_err().contains("`from`"));
    }

    /// Regression guard (review of #1309): a drag that moved nothing fails.
    #[test]
    fn a_drag_that_moved_nothing_is_not_a_pass() {
        assert!(drag_moved(&json!({ "moved": true, "effect_ok": true })));
        assert!(!drag_moved(&json!({ "moved": false, "effect_ok": false })));
        assert!(!drag_moved(&json!({ "moved": true, "effect_ok": false })));
        assert!(!drag_moved(&json!({})));
    }

    #[test]
    fn levels_come_from_the_result_or_the_action() {
        assert_eq!(
            level_of(
                &json!({ "native_level": "native_call" }),
                NativeLevel::RealInput
            ),
            NativeLevel::NativeCall
        );
        assert_eq!(
            level_of(&json!({}), NativeLevel::NativeCegui),
            NativeLevel::NativeCegui
        );
    }
}
