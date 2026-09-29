//! `client_player_state`: what the client believes about the player —
//! position, facing (`unitOrientation` is a 0..1 turn fraction; also given
//! in degrees), world, level and experience, every stat it reports
//! (health, focus, ammo slots, ...), active effects, the current target,
//! the active weapon's ammo, alignment and archetype, cash.
//!
//! Stat ids come from the client's `Stat` table at run time (read, never
//! guessed); `getUnitStat` returns a table whose `id` is nil for a stat
//! the unit does not have, and those are left out.

use std::time::Instant;

use serde_json::{json, Value};

use super::lua_json::list;
use super::{stamp_read, Supervisor};

/// The reader chunk body.
pub fn chunk(include_stats: bool, include_effects: bool) -> String {
    format!(
        r#"
local P = Unit and Unit.Player
local function stats_of(u)
  local out = {{}}
  if type(Stat) ~= 'table' then return out end
  for name, id in pairs(Stat) do
    if type(id) == 'number' then
      local s = __jcall(getUnitStat, u, id)
      if type(s) == 'table' and s.id then out[name] = {{ id = id, current = s.current, max = s.max }} end
    end
  end
  return out
end
local function effects_of(u)
  local out = {{}}
  local n = __jcall(getEffectCount, u) or 0
  for i = 1, n do
    local e = __jcall(getEffectInfo, u, i)
    if type(e) == 'table' then e.index = i end
    out[#out + 1] = e
  end
  return out
end
local me = {{
  name = __jcall(unitName, P),
  level = __jcall(unitLevel, P),
  position = __jvec(__jcall(unitPosition, P)),
  orientation_turns = __jcall(unitOrientation, P),
  world_id = __jcall(getCurrentWorldID),
  unit_world = __jcall(unitWorld, P),
  alignment_id = __jcall(unitAlignmentId, P),
  alignment = __jcall(getPlayerAlignmentString),
  archetype_id = __jcall(unitArchetypeId, P),
  archetype = __jcall(unitArchetype, P),
  experience = __jcall(getExperience),
  experience_max = __jcall(getMaxExperience),
  cash = __jcall(getCash),
}}
if {include_stats} then me.stats = stats_of(P) end
if {include_effects} then me.effects = effects_of(P) end
if Container and Container.Bandolier then
  local slot = __jcall(getActiveSlotForContainer, Container.Bandolier)
  local ammo = {{ active_slot = slot }}
  if type(slot) == 'number' and slot > 0 then
    ammo.weapon_item_id = __jcall(getItemIDForSlot, Container.Bandolier, slot)
    ammo.weapon = __jcall(getNameForSlot, Container.Bandolier, slot)
    ammo.ammo_type = __jcall(getCurrentAmmoType, Container.Bandolier, slot)
    if Stat and Stat.AmmoSlot1 then
      local s = __jcall(getUnitStat, P, Stat.AmmoSlot1 + slot - 1)
      if type(s) == 'table' then ammo.current = s.current ammo.max = s.max end
    end
  end
  me.ammo = ammo
end
local T = Unit and Unit.Target
local target = nil
if T and __jcall(unitExists, T) then
  target = {{
    name = __jcall(unitName, T),
    level = __jcall(unitLevel, T),
    hostility = __jcall(unitHostilityToPlayer, T),
    is_friend = __jcall(unitIsFriend, T),
    mob_id = __jcall(unitMobId, T),
    archetype_id = __jcall(unitArchetypeId, T),
    alignment_id = __jcall(unitAlignmentId, T),
    position = __jvec(__jcall(unitPosition, T)),
    is_player = __jcall(unitsEqual, T, P),
  }}
  if Stat then
    local h = __jcall(getUnitStat, T, Stat.Health)
    if type(h) == 'table' and h.id then target.health = {{ current = h.current, max = h.max }} end
  end
  if {include_effects} then target.effects = effects_of(T) end
end
me.target = target
return __jenc(me)"#
    )
}

/// Pull the headline numbers up next to the raw read: health and focus
/// as `{current, max}` (from `stats`), and the effect names.
pub fn shape(raw: &Value) -> Value {
    let mut out = raw.clone();
    for (key, stat) in [("health", "Health"), ("focus", "Focus")] {
        let s = &raw["stats"][stat];
        if !s.is_null() {
            out[key] = json!({ "current": s["current"], "max": s["max"] });
        }
    }
    let names: Vec<Value> = list(&raw["effects"])
        .iter()
        .filter_map(|e| {
            ["Name", "name", "EffectName"]
                .iter()
                .find_map(|k| e.get(*k).filter(|v| v.is_string()).cloned())
        })
        .collect();
    // unitOrientation is a fraction of a full turn (0..1), not radians.
    if let Some(t) = raw["orientation_turns"].as_f64() {
        out["heading_deg"] = json!(t * 360.0);
    }
    out["effect_names"] = json!(names);
    out["in_combat"] = raw["stats"]["InCombat"]["current"].clone();
    out
}

impl Supervisor {
    /// `client_player_state`.
    pub async fn ui_player_state(&self, stats: bool, effects: bool) -> Result<Value, String> {
        let t0 = Instant::now();
        let raw = self.lua_json(&chunk(stats, effects)).await?;
        let mut out = shape(&raw);
        stamp_read(&mut out, t0.elapsed().as_millis() as u64);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_lifts_health_focus_and_effect_names() {
        let raw = json!({
            "name": "Labone", "level": 12, "orientation_turns": 0.25,
            "position": { "x": 1.0, "y": 2.0, "z": 3.0 },
            "stats": { "Health": { "id": 1, "current": 80, "max": 100 }, "Focus": { "id": 2, "current": 5, "max": 50 } },
            "effects": [ { "Name": "Adrenaline", "Beneficial": true }, { "Hidden": true } ],
            "target": null
        });
        let s = shape(&raw);
        assert_eq!(s["health"], json!({ "current": 80, "max": 100 }));
        assert_eq!(s["focus"]["current"], 5);
        assert_eq!(s["effect_names"], json!(["Adrenaline"]));
        assert_eq!(s["position"]["z"], 3.0);
        assert_eq!(s["heading_deg"], 90.0);
        assert!(s["target"].is_null());
    }

    /// Without the stat read (or on a client with no Focus stat) the
    /// headline fields are simply absent.
    #[test]
    fn shape_without_stats_leaves_headlines_out() {
        let s = shape(&json!({ "name": "Labone", "effects": {} }));
        assert!(s.get("health").is_none());
        assert_eq!(s["effect_names"], json!([]));
    }

    #[test]
    fn chunk_switches_stats_and_effects() {
        let c = chunk(true, false);
        assert!(c.contains("if true then me.stats = stats_of(P) end"));
        assert!(c.contains("if false then me.effects = effects_of(P) end"));
        assert!(c.contains("__jvec(__jcall(unitPosition, P))"));
    }
}
