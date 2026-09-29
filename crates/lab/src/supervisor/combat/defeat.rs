//! `client_die_and_respawn`: reach defeat, handle the defeat window like a
//! player, and verify the respawn.
//!
//! The defeat window (`Core/PlayerDefeat/PlayerDefeat.lua`) opens on
//! `Events.BeginWait` (the server's `onBeginAidWait(TimeToAid,
//! respawners)`), lists the respawners in `PlayerDefeat_RespawnerList`
//! (first one preselected) with a countdown in
//! `PlayerDefeat_RezCountdown`, and its **Release** button
//! (`PlayerDefeat_Release`) calls `callForAid(<selected respawner id>)`.
//! When the countdown runs out the window calls `callForAid` itself.
//! `Events.EndWait` (`onEndAidWait`) hides it.
//!
//! Getting defeated is setup, not the behaviour under test: the optional
//! `setup_health` types `/gmsethealth <n> 0` into chat (a GM command, G
//! tier) so the next hit is lethal. `/gmsethealth 0 0` alone sets the stat
//! to zero but does not run the server's death sequence (the handler only
//! writes the stat), so the flow waits for the defeat window, which only a
//! real death opens: let an enemy land the hit, or use an ability on
//! yourself.

use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::player::DEFEAT_WIN;
use super::NativeLevel;
use crate::supervisor::flows::{settle, widgets, FlowError, FlowRun};
use crate::supervisor::Supervisor;

pub const RELEASE_BUTTON: &str = "PlayerDefeat_Release";
pub const RESPAWNER_LIST: &str = "PlayerDefeat_RespawnerList";
pub const DEFAULT_DEFEAT_TIMEOUT: Duration = Duration::from_secs(60);
pub const DEFAULT_RESPAWN_TIMEOUT: Duration = Duration::from_secs(60);

/// What to do once the window is up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Respawn {
    /// Click Release (real click).
    Release,
    /// Let the countdown run out (the window releases itself).
    Auto,
    /// Stop at the window.
    None,
}

impl Respawn {
    pub fn parse(s: Option<&str>) -> Result<Self, String> {
        match s.unwrap_or("release") {
            "release" => Ok(Self::Release),
            "auto" | "timeout" => Ok(Self::Auto),
            "none" => Ok(Self::None),
            o => Err(format!("respawn {o:?}: release, auto or none")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DieRequest {
    pub setup_health: Option<i32>,
    pub respawn: Respawn,
    /// Pick this respawner (by name, case-insensitive substring) instead
    /// of the preselected first row.
    pub respawner: Option<String>,
    pub defeat_timeout: Duration,
    pub respawn_timeout: Duration,
}

/// Lua: the defeat window's contents: respawner rows (text, id, selected),
/// countdown and instruction text.
pub const DEFEAT_READ_CHUNK: &str = r#"local out = {}
local function add(...) out[#out + 1] = table.concat({...}, "\t") end
local function clean(s) if s == nil then return "" end return (string.gsub(tostring(s), "%c", " ")) end
local lb = PlayerDefeat_RespawnerList
if lb ~= nil then
  for i = 0, lb:getItemCount() - 1 do
    local it = lb:getListboxItemFromIndex(i)
    if it ~= nil then add("respawner", i, clean(it:getText()), clean(it:getID()), tostring(it:isSelected())) end
  end
end
if PlayerDefeat_RezCountdown ~= nil then add("countdown", clean(PlayerDefeat_RezCountdown:getText())) end
if PlayerDefeat_RezText ~= nil then add("text", clean(PlayerDefeat_RezText:getText())) end
if PlayerDefeatMod ~= nil then add("total_s", clean(PlayerDefeatMod.totalRespawnTime)) end
return unpack(out)"#;

/// A respawner row.
#[derive(Debug, Clone, PartialEq)]
pub struct Respawner {
    pub index: u32,
    pub name: String,
    pub id: i64,
    pub selected: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DefeatWindow {
    pub respawners: Vec<Respawner>,
    pub countdown: String,
    pub text: String,
    pub total_s: Option<f64>,
}

impl DefeatWindow {
    pub fn selected(&self) -> Option<&Respawner> {
        self.respawners.iter().find(|r| r.selected)
    }

    /// The row whose name contains `want` (case-insensitive); exactly one.
    pub fn find(&self, want: &str) -> Result<&Respawner, String> {
        let w = want.to_lowercase();
        let hits: Vec<&Respawner> = self
            .respawners
            .iter()
            .filter(|r| r.name.to_lowercase().contains(&w))
            .collect();
        match hits.as_slice() {
            [one] => Ok(one),
            [] => Err(format!(
                "no respawner matching {want:?} (listed: {})",
                self.names().join(", ")
            )),
            _ => Err(format!(
                "respawner {want:?} is ambiguous (listed: {})",
                self.names().join(", ")
            )),
        }
    }

    fn names(&self) -> Vec<String> {
        self.respawners.iter().map(|r| r.name.clone()).collect()
    }

    pub fn to_json(&self) -> Value {
        json!({
            "respawners": self.respawners.iter().map(|r| json!({
                "index": r.index, "name": r.name, "id": r.id, "selected": r.selected,
            })).collect::<Vec<_>>(),
            "countdown": self.countdown,
            "text": self.text,
            "auto_respawn_s": self.total_s,
        })
    }
}

pub fn parse_defeat_window(lines: &[String]) -> DefeatWindow {
    let mut w = DefeatWindow::default();
    for line in lines {
        let f: Vec<&str> = line.split('\t').collect();
        match f.as_slice() {
            ["respawner", i, name, id, sel] => w.respawners.push(Respawner {
                index: i.parse().unwrap_or(0),
                name: name.to_string(),
                id: id.parse::<f64>().map(|f| f as i64).unwrap_or(0),
                selected: *sel == "true",
            }),
            ["countdown", c] => w.countdown = c.to_string(),
            ["text", t] => w.text = t.to_string(),
            ["total_s", t] => w.total_s = t.parse().ok(),
            _ => {}
        }
    }
    w
}

/// Lua: select respawner row `index` (the list's own selection call; rows
/// are list items, not named windows, so the lab cannot click one yet).
pub fn select_respawner_chunk(index: u32) -> String {
    format!(
        "local lb = {RESPAWNER_LIST} lb:clearAllSelections() \
         local it = lb:getListboxItemFromIndex({index}) lb:setItemSelectState(it, true) \
         return tostring(it:isSelected())"
    )
}

impl Supervisor {
    pub async fn die_and_respawn_flow(&self, req: DieRequest) -> Result<Value, FlowError> {
        let mut run = FlowRun::new(self, "client_die_and_respawn");
        let before = self
            .player_state()
            .await
            .map_err(|e| run.fail("player_state", e))?;
        let seq_before = self
            .pump_events(true)
            .await
            .map_err(|e| run.fail("baseline", e))?
            .head;
        let mut setup = Vec::new();
        if let Some(h) = req.setup_health {
            if h < 0 {
                return Err(run.fail("setup", "setup_health must be >= 0"));
            }
            let line = format!("/gmsethealth {h} 0");
            self.input_focus(true)
                .await
                .map_err(|e| run.fail("focus", e))?;
            let t0 = Instant::now();
            run.key("open_chat", "Enter").await?;
            settle(400).await;
            self.type_text(&line)
                .await
                .map_err(|e| run.fail("setup_type", e))?;
            run.key("send", "Enter").await?;
            run.record("setup", t0, json!({ "typed": line }));
            setup.push(json!({ "command": line, "native_level": NativeLevel::GmSetup.to_json() }));
            settle(500).await;
        }

        // Defeat.
        let t_wait = Instant::now();
        let defeated = self
            .poll_until(
                &widgets::visible(DEFEAT_WIN),
                None,
                req.defeat_timeout,
                Duration::from_millis(500),
            )
            .await
            .map_err(|e| run.fail("wait_defeat", e))?;
        if !defeated.met {
            let now = self.player_state().await.ok();
            return Err(run
                .fail_with_state(
                    "wait_defeat",
                    format!(
                        "the defeat window did not open within {} ms (health now {:?}; {} does not \
                         kill by itself: an enemy or an ability has to land the lethal hit)",
                        defeated.elapsed_ms,
                        now.and_then(|p| p.health),
                        if req.setup_health.is_some() { "/gmsethealth" } else { "nothing was set up and" },
                    ),
                )
                .await);
        }
        run.record(
            "defeated",
            t_wait,
            json!({ "after_ms": defeated.elapsed_ms }),
        );
        settle(300).await;
        let window = parse_defeat_window(&run.lua("read_defeat_window", DEFEAT_READ_CHUNK).await?);
        let at_death = self.player_state().await.ok();

        let mut chosen = window.selected().cloned();
        let mut selection_level = Value::Null;
        if let Some(want) = &req.respawner {
            let r = window
                .find(want)
                .map_err(|e| run.fail("select_respawner", e))?
                .clone();
            if !r.selected {
                run.lua("select_respawner", &select_respawner_chunk(r.index))
                    .await?;
                selection_level = NativeLevel::UiLuaCall.to_json();
            }
            chosen = Some(r);
        }

        let mut respawn_level = Value::Null;
        match req.respawn {
            Respawn::None => {
                return Ok(run.finish(json!({
                    "setup": setup,
                    "defeated": true,
                    "defeat_window": window.to_json(),
                    "at_death": at_death.map(|p| p.to_json()),
                    "before": before.to_json(),
                    "respawned": false,
                    "event_seq": { "before": seq_before, "after": self.event_head().await },
                })));
            }
            Respawn::Release => {
                self.input_focus(true)
                    .await
                    .map_err(|e| run.fail("focus", e))?;
                run.click("release", RELEASE_BUTTON).await?;
                respawn_level = NativeLevel::RealInput.to_json();
            }
            Respawn::Auto => {
                // The window's own timer releases (no input at all).
            }
        }

        // Respawn: window gone, alive again.
        let t_resp = Instant::now();
        let hidden = self
            .poll_until(
                &format!("not {}", widgets::visible(DEFEAT_WIN)),
                None,
                req.respawn_timeout,
                Duration::from_millis(500),
            )
            .await
            .map_err(|e| run.fail("wait_window_closed", e))?;
        if !hidden.met {
            return Err(run
                .fail_with_state(
                    "wait_window_closed",
                    format!(
                        "the defeat window was still open after {} ms",
                        hidden.elapsed_ms
                    ),
                )
                .await);
        }
        let alive = self
            .poll_until(
                "getUnitStat(Unit.Player, Stat.Health).current > 0",
                None,
                req.respawn_timeout,
                Duration::from_millis(500),
            )
            .await
            .map_err(|e| run.fail("wait_alive", e))?;
        run.record(
            "respawned",
            t_resp,
            json!({ "window_closed_ms": hidden.elapsed_ms, "alive": alive.met }),
        );
        // A respawn at another point can stream a new area; let it settle.
        settle(1500).await;
        let after = self
            .player_state()
            .await
            .map_err(|e| run.fail("player_state_after", e))?;
        let pump = self.pump_events(true).await.ok();
        let (moved, world_changed) = (
            after.distance_to(at_death.as_ref().unwrap_or(&before)),
            after.world_id != before.world_id,
        );
        let checks = json!({
            "defeat_window_closed": hidden.met,
            "alive": after.alive(),
            "moved_from_death_point_m": moved,
            "world_changed": world_changed,
        });
        Ok(run.finish(json!({
            "setup": setup,
            "defeated": true,
            "defeat_window": window.to_json(),
            "respawner": chosen.map(|r| json!({ "name": r.name, "id": r.id })),
            "respawner_selection": selection_level,
            "respawn": match req.respawn { Respawn::Release => "release", Respawn::Auto => "auto", Respawn::None => "none" },
            "native_level": if respawn_level.is_null() { json!({ "tier": "none", "label": "auto-release timer (no input)" }) } else { respawn_level },
            "before": before.to_json(),
            "at_death": at_death.map(|p| p.to_json()),
            "after": after.to_json(),
            "checks": checks,
            "verified": hidden.met && after.alive() == Some(true),
            "event_seq": { "before": seq_before, "after": pump.map(|p| p.head) },
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    fn window() -> DefeatWindow {
        parse_defeat_window(&s(&[
            "respawner\t0\tCellblock Aid Station\t12\ttrue",
            "respawner\t1\tCourtyard Aid Station\t13\tfalse",
            "countdown\tAutomatically respawn in 0:07...",
            "text\tYou have been defeated.",
            "total_s\t9",
        ]))
    }

    #[test]
    fn the_window_read_parses() {
        let w = window();
        assert_eq!(w.respawners.len(), 2);
        assert_eq!(w.selected().unwrap().id, 12);
        assert_eq!(w.total_s, Some(9.0));
        assert_eq!(
            w.to_json()["respawners"][1]["name"],
            "Courtyard Aid Station"
        );
        assert!(DEFEAT_READ_CHUNK.contains("PlayerDefeat_RespawnerList"));
    }

    #[test]
    fn respawner_lookup_is_a_unique_substring() {
        let w = window();
        assert_eq!(w.find("courtyard").unwrap().index, 1);
        assert!(w.find("aid station").unwrap_err().contains("ambiguous"));
        assert!(w.find("tollana").unwrap_err().contains("no respawner"));
    }

    #[test]
    fn respawn_modes_parse() {
        assert_eq!(Respawn::parse(None).unwrap(), Respawn::Release);
        assert_eq!(Respawn::parse(Some("auto")).unwrap(), Respawn::Auto);
        assert_eq!(Respawn::parse(Some("none")).unwrap(), Respawn::None);
        assert!(Respawn::parse(Some("revive")).is_err());
        assert!(select_respawner_chunk(1).contains("getListboxItemFromIndex(1)"));
    }
}
