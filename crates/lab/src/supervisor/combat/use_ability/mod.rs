//! `client_use_ability`: fire an ability the way a player does, and report
//! what came back.
//!
//! Order of preference (the owner's native-first rule):
//!
//! 1. **On the hotbar** — press the button's bound key
//!    (`getBindingKey('ActionButton<n>')`, a real `WM_KEYDOWN`/`UP`), or
//!    click the button when it has no key the lab can post. N1.
//! 2. **Not on the hotbar, `place: true`** — put it on the first visible
//!    empty button the way the drop handler does
//!    (`getUnusedAction` + `ActionProfileMod.setButtonCurrentAction` +
//!    `setActionToAbility`: N3, because a CEGUI drag cannot be started from
//!    posted mouse moves yet), then press that button's key or click it
//!    (N1). Placement persists in the client's profile, like a player's.
//! 3. **Not on the hotbar** — click it in the Ability window (N1 click;
//!    the window is opened with its bound key, or its own toggle handler as
//!    N3 when unbound).
//! 4. **`fallback: "lua"`** — `useAbility(id, Unit.Target)`, what the
//!    Ability window's button runs, labelled N3.
//!
//! No slash command for abilities is known in the client's command table,
//! so there is no N2 path.
//!
//! The result is read from the event store (see [`outcome`]) for up to
//! `observe_ms` after the press, stopping early once an effect or a refusal
//! arrives.

pub mod outcome;
pub mod resolve;
pub mod window;

use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::hotbar::{Hotbar, HotbarButton};
use super::NativeLevel;
use crate::supervisor::flows::{settle, FlowError, FlowRun};
use crate::supervisor::{now_ms, Supervisor};
use outcome::classify;
use resolve::{known_chunk, parse_known, resolve, Query};
use window::pressable;

pub const DEFAULT_OBSERVE_MS: u64 = 2500;
pub const MAX_OBSERVE_MS: u64 = 30_000;
const OBSERVE_POLL: Duration = Duration::from_millis(250);
/// After the first settling event, keep reading this long for stragglers
/// (the cooldown timer usually lands right after the effect).
const STRAGGLE: Duration = Duration::from_millis(400);

/// How to press a hotbar button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    /// Its key when the lab can press it, else a click.
    Auto,
    Key,
    Click,
}

/// What to do when the ability is not on the hotbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fallback {
    /// Click it in the Ability window.
    Window,
    /// `useAbility(id, Unit.Target)` (N3).
    Lua,
    /// Report and stop.
    None,
}

#[derive(Debug, Clone)]
pub struct UseAbilityRequest {
    pub query: Query,
    pub press: Press,
    pub place: bool,
    pub fallback: Fallback,
    pub observe: Duration,
}

impl Press {
    pub fn parse(s: Option<&str>) -> Result<Self, String> {
        match s.unwrap_or("auto") {
            "auto" => Ok(Self::Auto),
            "key" => Ok(Self::Key),
            "click" => Ok(Self::Click),
            o => Err(format!("press {o:?}: auto, key or click")),
        }
    }
}

impl Fallback {
    pub fn parse(s: Option<&str>) -> Result<Self, String> {
        match s.unwrap_or("window") {
            "window" | "ability_window" => Ok(Self::Window),
            "lua" => Ok(Self::Lua),
            "none" => Ok(Self::None),
            o => Err(format!("fallback {o:?}: window, lua or none")),
        }
    }
}

/// Lua: place `ability` on hotbar `button` the way
/// `ActionProfileMod.receiveDrag` does after a drop.
pub fn place_chunk(button: u32, ability: i64) -> String {
    format!(
        r#"local bid = {button}
local actionId = ActionButtonMod.getActionForButton(bid)
if actionId == nil or actionId < 1 then
  actionId = getUnusedAction()
  if actionId == nil or actionId < 1 then return "error", "no unused action (the client's action pool is full)" end
  ActionProfileMod.setButtonCurrentAction(bid, actionId)
end
setActionToAbility(actionId, {ability})
return "ok", tostring(actionId)"#
    )
}

impl Supervisor {
    /// Press a hotbar button: its key when allowed and pressable, else a
    /// click on its window. Returns the step detail.
    async fn press_hotbar_button(
        &self,
        run: &mut FlowRun<'_>,
        b: &HotbarButton,
        press: Press,
    ) -> Result<Value, FlowError> {
        self.input_focus(true)
            .await
            .map_err(|e| run.fail("focus", e))?;
        let key = if press == Press::Click {
            None
        } else {
            pressable(&b.keys)
        };
        if press == Press::Key && key.is_none() {
            return Err(run.fail(
                "press",
                format!(
                    "button {} has no binding the lab can press (keys: {})",
                    b.button,
                    json!(b.keys.iter().map(|k| k.to_json()).collect::<Vec<_>>())
                ),
            ));
        }
        let t0 = Instant::now();
        let detail = match key {
            Some((binding, k)) => {
                self.press_binding(&binding, &k)
                    .await
                    .map_err(|e| run.fail("press_key", e))?;
                json!({ "how": "key", "key": k, "modifiers": binding.modifiers(),
                        "binding": binding.to_json(), "button": b.button })
            }
            None => {
                if b.window.is_empty() || !b.visible {
                    return Err(run
                        .fail_with_state(
                            "press_click",
                            format!(
                                "button {} has no pressable key and its window {:?} is not visible",
                                b.button, b.window
                            ),
                        )
                        .await);
                }
                self.ui_click(&b.window, 0)
                    .await
                    .map_err(|e| run.fail("press_click", e))?;
                json!({ "how": "click", "window": b.window, "button": b.button })
            }
        };
        run.record("press", t0, detail.clone());
        Ok(detail)
    }

    pub async fn use_ability_flow(&self, req: UseAbilityRequest) -> Result<Value, FlowError> {
        let mut run = FlowRun::new(self, "client_use_ability");
        // Baseline pump: installs the combat/chat rings and fixes the seq
        // the outcome is read from.
        let pump = self
            .pump_events(true)
            .await
            .map_err(|e| run.fail("baseline", e))?;
        let seq_before = pump.head;
        let t0 = Instant::now();
        // Placing needs the empty buttons too: the bound-only read lists
        // none of them, and on a fresh character's empty bar it lists
        // nothing at all (the 2026-10-04 live run's "no visible empty
        // hotbar button" on a bar of 100 empty buttons).
        let hotbar = self
            .read_hotbar(req.place)
            .await
            .map_err(|e| run.fail("read_hotbar", e))?;
        let probe = match &req.query {
            Query::Id(id) => *id,
            Query::Name(_) => 0,
        };
        let known = parse_known(&run.lua("known_abilities", &known_chunk(probe)).await?);
        let ability = resolve(&req.query, &hotbar, &known).map_err(|e| run.fail("resolve", e))?;
        run.record("resolve", t0, ability.to_json());
        let before = self
            .player_state()
            .await
            .map_err(|e| run.fail("player_state", e))?;

        // Decide the path.
        let mut placed = Value::Null;
        let mut hotbar_now: Hotbar = hotbar;
        let mut button = ability
            .buttons
            .first()
            .and_then(|n| hotbar_now.buttons.iter().find(|b| b.button == *n))
            .cloned();
        if button.is_none() && req.place {
            let target = hotbar_now.first_empty_visible().cloned().ok_or_else(|| {
                run.fail(
                    "place",
                    format!(
                        "no visible empty hotbar button to place the ability on \
                         ({} buttons read, {} visible)",
                        hotbar_now.buttons.len(),
                        hotbar_now.buttons.iter().filter(|b| b.visible).count()
                    ),
                )
            })?;
            let tp = Instant::now();
            let r = run
                .lua("place", &place_chunk(target.button, ability.id))
                .await?;
            if r.first().map(String::as_str) != Some("ok") {
                return Err(run.fail("place", format!("placement refused: {r:?}")));
            }
            settle(200).await;
            hotbar_now = self
                .read_hotbar(false)
                .await
                .map_err(|e| run.fail("read_hotbar_after_place", e))?;
            button = hotbar_now
                .buttons_for_ability(ability.id)
                .first()
                .map(|b| (*b).clone());
            if button.is_none() {
                return Err(run
                    .fail_with_state(
                        "place",
                        format!(
                            "button {} does not hold ability {} after placement",
                            target.button, ability.id
                        ),
                    )
                    .await);
            }
            placed = json!({ "button": target.button, "action_id": r.get(1),
                             "native_level": NativeLevel::UiLuaCall.to_json() });
            run.record("place", tp, placed.clone());
        }

        let press_ms = now_ms();
        let (path, level, press_detail, cooldown_before) = if let Some(b) = &button {
            let d = self.press_hotbar_button(&mut run, b, req.press).await?;
            ("hotbar", NativeLevel::RealInput, d, b.on_cooldown())
        } else {
            match (req.fallback, ability.window) {
                (Fallback::Window, Some((tree, index))) => {
                    let (d, _open) = self
                        .fire_from_ability_window(&mut run, tree, index, ability.id)
                        .await?;
                    ("ability_window", NativeLevel::RealInput, d, false)
                }
                (Fallback::Lua, _) => {
                    let tl = Instant::now();
                    run.lua(
                        "use_ability_lua",
                        &format!("useAbility({}, Unit.Target) return 'ok'", ability.id),
                    )
                    .await?;
                    let d = json!({ "how": "useAbility(id, Unit.Target)" });
                    run.record("use_ability_lua", tl, d.clone());
                    ("lua", NativeLevel::UiLuaCall, d, false)
                }
                (Fallback::Window, None) => {
                    return Err(run.fail(
                        "no_native_path",
                        format!(
                            "ability {} ({}) is not on the hotbar and not in the Ability window's trees \
                             (a GM-granted ability?); pass place=true to put it on the bar, or fallback=\"lua\"",
                            ability.id, ability.name
                        ),
                    ))
                }
                (Fallback::None, _) => {
                    return Err(run.fail(
                        "not_on_hotbar",
                        format!(
                            "ability {} ({}) is not on the hotbar (place=true puts it there)",
                            ability.id, ability.name
                        ),
                    ))
                }
            }
        };

        // Observe.
        let tobs = Instant::now();
        let mut settled_at: Option<Instant> = None;
        let mut out = outcome::Outcome::default();
        let mut last_head = seq_before;
        while tobs.elapsed() < req.observe {
            settle(OBSERVE_POLL.as_millis() as u64).await;
            let p = self
                .pump_events(true)
                .await
                .map_err(|e| run.fail("observe", e))?;
            last_head = p.head;
            let events = self
                .events
                .inner
                .lock()
                .await
                .since(seq_before, 8192)
                .events;
            out = classify(&events, press_ms, ability.id);
            if out.settled() && settled_at.is_none() {
                settled_at = Some(Instant::now());
            }
            if settled_at.is_some_and(|s| s.elapsed() >= STRAGGLE) {
                break;
            }
        }
        run.record("observe", tobs, json!({ "verdict": out.verdict() }));

        let cooldown_after = if path == "hotbar" {
            self.read_hotbar(false).await.ok().and_then(|hb| {
                button.as_ref().and_then(|b| {
                    hb.buttons.iter().find(|x| x.button == b.button).map(|x| {
                        json!({ "remaining_s": x.cooldown_remaining, "total_s": x.cooldown_total,
                                "active": x.on_cooldown() })
                    })
                })
            })
        } else {
            None
        };
        let after = self.player_state().await.ok();
        Ok(run.finish(json!({
            "ability": ability.to_json(),
            "path": path,
            "native_level": level.to_json(),
            "press": press_detail,
            "placed": placed,
            "was_on_cooldown": cooldown_before,
            "target_before": { "name": before.target, "hostility": before.target_hostility },
            "result": out.to_json(),
            "cooldown_after": cooldown_after,
            "player_after": after.map(|a| a.to_json()),
            "event_seq": { "before": seq_before, "after": last_head },
            // Host epoch ms just before the key or click went out: the
            // UAT runner centres its cast-id press window on it.
            "press_ms": press_ms,
            "rings": pump.to_json(),
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placement_mirrors_the_drop_handler() {
        let c = place_chunk(4, 1100);
        assert!(c.contains("ActionButtonMod.getActionForButton(bid)"));
        assert!(c.contains("getUnusedAction()"));
        assert!(c.contains("ActionProfileMod.setButtonCurrentAction(bid, actionId)"));
        assert!(c.contains("setActionToAbility(actionId, 1100)"));
    }

    #[test]
    fn press_and_fallback_parse() {
        assert_eq!(Press::parse(None).unwrap(), Press::Auto);
        assert_eq!(Press::parse(Some("click")).unwrap(), Press::Click);
        assert!(Press::parse(Some("tap")).is_err());
        assert_eq!(Fallback::parse(None).unwrap(), Fallback::Window);
        assert_eq!(Fallback::parse(Some("lua")).unwrap(), Fallback::Lua);
        assert!(Fallback::parse(Some("slash")).is_err());
    }
}
