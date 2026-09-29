//! A small player-state read for the combat flows: world, position,
//! health, focus, the current target, and whether the defeat window is up.
//! Every value is read under its own `pcall` through the stock UI's
//! bindings (`getCurrentWorldID`, `unitWorld`, `unitPosition`,
//! `getUnitStat(Unit.Player, Stat.Health)`, `unitName(Unit.Target)`), so a
//! missing binding blanks one field, not the read.

use serde_json::{json, Value};

use crate::supervisor::Supervisor;

/// The defeat window (`Core/PlayerDefeat/SGWUI_PlayerDefeat.layout`).
pub const DEFEAT_WIN: &str = "PlayerDefeatWin";

pub const PLAYER_CHUNK: &str = r#"local out = {}
local function put(k, f)
  local ok, v = pcall(f)
  if ok and v ~= nil then out[#out + 1] = k .. "\t" .. string.gsub(tostring(v), "%c", " ") end
end
put("world_id", function() return getCurrentWorldID() end)
put("world", function() return unitWorld(Unit.Player) end)
put("x", function() return unitPosition(Unit.Player).x end)
put("y", function() return unitPosition(Unit.Player).y end)
put("z", function() return unitPosition(Unit.Player).z end)
put("health", function() return getUnitStat(Unit.Player, Stat.Health).current end)
put("health_max", function() return getUnitStat(Unit.Player, Stat.Health).max end)
put("focus", function() return getUnitStat(Unit.Player, Stat.Focus).current end)
put("focus_max", function() return getUnitStat(Unit.Player, Stat.Focus).max end)
put("target_exists", function() return unitExists(Unit.Target) end)
put("target", function() if unitExists(Unit.Target) then return unitName(Unit.Target) end end)
put("target_hostility", function() if unitExists(Unit.Target) then return unitHostilityToPlayer(Unit.Target) end end)
put("defeat_visible", function() return PlayerDefeatWin ~= nil and PlayerDefeatWin:isVisible() end)
return unpack(out)"#;

/// A player-state snapshot.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlayerState {
    pub world_id: Option<i64>,
    pub world: Option<String>,
    pub pos: Option<[f64; 3]>,
    pub health: Option<f64>,
    pub health_max: Option<f64>,
    pub focus: Option<f64>,
    pub focus_max: Option<f64>,
    pub target: Option<String>,
    pub target_hostility: Option<String>,
    pub defeat_visible: bool,
}

impl PlayerState {
    pub fn alive(&self) -> Option<bool> {
        self.health.map(|h| h > 0.0)
    }

    /// Straight-line distance between two snapshots' positions.
    pub fn distance_to(&self, other: &PlayerState) -> Option<f64> {
        let (a, b) = (self.pos?, other.pos?);
        Some(((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt())
    }

    pub fn to_json(&self) -> Value {
        json!({
            "world_id": self.world_id,
            "world": self.world,
            "position": self.pos,
            "health": self.health,
            "health_max": self.health_max,
            "focus": self.focus,
            "focus_max": self.focus_max,
            "alive": self.alive(),
            "target": self.target,
            "target_hostility": self.target_hostility,
            "defeat_window": self.defeat_visible,
        })
    }
}

pub fn parse_player(lines: &[String]) -> PlayerState {
    let mut p = PlayerState::default();
    let (mut x, mut y, mut z) = (None, None, None);
    for line in lines {
        let Some((k, v)) = line.split_once('\t') else {
            continue;
        };
        let f = || v.parse::<f64>().ok().filter(|f| f.is_finite());
        match k {
            "world_id" => p.world_id = f().map(|f| f as i64),
            "world" => p.world = Some(v.to_string()),
            "x" => x = f(),
            "y" => y = f(),
            "z" => z = f(),
            "health" => p.health = f(),
            "health_max" => p.health_max = f(),
            "focus" => p.focus = f(),
            "focus_max" => p.focus_max = f(),
            "target" => p.target = Some(v.to_string()),
            "target_hostility" => p.target_hostility = Some(v.to_string()),
            "defeat_visible" => p.defeat_visible = v == "true",
            _ => {}
        }
    }
    if let (Some(x), Some(y), Some(z)) = (x, y, z) {
        p.pos = Some([x, y, z]);
    }
    p
}

impl Supervisor {
    pub async fn player_state(&self) -> Result<PlayerState, String> {
        Ok(parse_player(&self.lua_results(PLAYER_CHUNK).await?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn a_full_read_parses() {
        let p = parse_player(&s(&[
            "world_id\t1\n",
            "world_id\t701",
            "world\tCastle_CellBlock",
            "x\t10.5",
            "y\t-2",
            "z\t3",
            "health\t0",
            "health_max\t850",
            "target\tCellblock Guard",
            "defeat_visible\ttrue",
        ]));
        assert_eq!(p.world_id, Some(701));
        assert_eq!(p.pos, Some([10.5, -2.0, 3.0]));
        assert_eq!(p.alive(), Some(false));
        assert!(p.defeat_visible);
        assert_eq!(p.to_json()["target"], "Cellblock Guard");
    }

    #[test]
    fn a_partial_position_is_no_position() {
        let p = parse_player(&s(&["x\t1", "y\t2"]));
        assert_eq!(p.pos, None);
        assert_eq!(p.alive(), None);
    }

    #[test]
    fn distance_between_snapshots() {
        let a = parse_player(&s(&["x\t0", "y\t0", "z\t0"]));
        let b = parse_player(&s(&["x\t3", "y\t4", "z\t0"]));
        assert_eq!(a.distance_to(&b), Some(5.0));
    }

    #[test]
    fn the_chunk_pcalls_each_field() {
        assert!(PLAYER_CHUNK.contains("put(\"health\""));
        assert!(PLAYER_CHUNK.contains("PlayerDefeatWin:isVisible()"));
        assert_eq!(DEFEAT_WIN, "PlayerDefeatWin");
    }
}
