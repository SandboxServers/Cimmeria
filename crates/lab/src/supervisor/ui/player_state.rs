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
local function stat_field(s, k)
  local ok, v = pcall(function() return s[k] end)
  if ok then return v end
  return nil
end
local STAT_NAMES = {{ 'Accuracy', 'AmmoSlot1', 'AmmoSlot2', 'AmmoSlot3', 'AmmoSlot4', 'AmmoSlot5', 'Coordination', 'CoverQRModifier', 'Damage', 'Defense', 'DeploymentAmmo', 'DisguiseRating', 'EnergyAF', 'Engagement', 'Focus', 'FocusRegen', 'Fortitude', 'HazmatAF', 'Health', 'HealthRegen', 'HealthRes', 'Intelligence', 'KineticRes', 'MentalRes', 'Morale', 'MovementSpeedMod', 'Penetration', 'Perception', 'PhysicalAF', 'PsionicAF', 'QrMod', 'RangeModifier', 'RevealRating', 'StealthRating' }}
local function stat_names()
  local names = {{}}
  pcall(function()
    for name in pairs(Stat['.get']) do
      if type(name) == 'string' then names[#names + 1] = name end
    end
  end)
  if #names > 0 then
    table.sort(names)
    return names, 'get'
  end
  return STAT_NAMES, 'fallback'
end
local function stats_of(u)
  local out, listed, read = {{}}, 0, 0
  local names, source = stat_names()
  for _, name in ipairs(names) do
    local ok, raw_id = pcall(function() return Stat[name] end)
    local id = ok and tonumber(raw_id) or nil
    if id then
      listed = listed + 1
      local s = __jcall(getUnitStat, u, id)
      if (type(s) == 'table' or type(s) == 'userdata') and stat_field(s, 'current') ~= nil then
        read = read + 1
        out[name] = {{ id = id, current = stat_field(s, 'current'), max = stat_field(s, 'max'), min = stat_field(s, 'min'), base_current = stat_field(s, 'baseCurrent') }}
      end
    end
  end
  return out, {{ stat_type = type(Stat), source = source, listed = listed, read = read }}
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
if {include_stats} then me.stats, me.stats_diag = stats_of(P) end
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
      if s ~= nil and stat_field(s, 'current') ~= nil then ammo.current = stat_field(s, 'current') ammo.max = stat_field(s, 'max') end
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
    if h ~= nil and stat_field(h, 'current') ~= nil then target.health = {{ current = stat_field(h, 'current'), max = stat_field(h, 'max') }} end
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
    // An empty stat map used to look like "no stats"; say what the read saw.
    let empty = raw["stats"].as_object().is_none_or(|m| m.is_empty());
    if empty && !raw["stats_diag"].is_null() {
        let d = &raw["stats_diag"];
        out["stats_note"] = json!(format!(
            "no stat was readable: the client's Stat table is a {}, its names came from {}, {} ids were listed and {} read. Read one directly with client_lua_eval: getUnitStat(Unit.Player, Stat.Health).current.",
            d["stat_type"].as_str().unwrap_or("nil"),
            d["source"].as_str().unwrap_or("nothing"),
            d["listed"].as_u64().unwrap_or(0),
            d["read"].as_u64().unwrap_or(0),
        ));
    }
    out["effect_names"] = json!(names);
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
        // No client stat or Lua query for combat state is verified, so the
        // result must not carry an always-null in_combat.
        assert!(s.get("in_combat").is_none());
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
        assert!(c.contains("if true then me.stats, me.stats_diag = stats_of(P) end"));
        assert!(c.contains("if false then me.effects = effects_of(P) end"));
        assert!(c.contains("__jvec(__jcall(unitPosition, P))"));
    }

    /// The 34 stat names the client's `Stat['.get']` table holds (probe C-P1,
    /// 2026-10-12). The chunk must carry exactly these as its fallback list.
    const CLIENT_STAT_NAMES: [&str; 34] = [
        "Accuracy",
        "AmmoSlot1",
        "AmmoSlot2",
        "AmmoSlot3",
        "AmmoSlot4",
        "AmmoSlot5",
        "Coordination",
        "CoverQRModifier",
        "Damage",
        "Defense",
        "DeploymentAmmo",
        "DisguiseRating",
        "EnergyAF",
        "Engagement",
        "Focus",
        "FocusRegen",
        "Fortitude",
        "HazmatAF",
        "Health",
        "HealthRegen",
        "HealthRes",
        "Intelligence",
        "KineticRes",
        "MentalRes",
        "Morale",
        "MovementSpeedMod",
        "Penetration",
        "Perception",
        "PhysicalAF",
        "PsionicAF",
        "QrMod",
        "RangeModifier",
        "RevealRating",
        "StealthRating",
    ];

    /// An empty stat read must explain itself instead of looking like "no
    /// stats": the compact result drops an empty map, so the note is all the
    /// caller sees.
    #[test]
    fn shape_explains_an_empty_stat_read() {
        let raw = json!({
            "name": "Labone", "stats": {},
            "stats_diag": { "stat_type": "table", "source": "fallback", "listed": 0, "read": 0 }
        });
        let s = shape(&raw);
        assert!(s.get("health").is_none());
        let note = s["stats_note"].as_str().unwrap();
        assert!(
            note.contains("a table") && note.contains("fallback") && note.contains("0 read"),
            "{note}"
        );
        // A read that found stats adds no note.
        let ok = shape(&json!({
            "stats": { "Health": { "id": 7, "current": 5, "max": 9 } },
            "stats_diag": { "stat_type": "table", "source": "get", "listed": 1, "read": 1 }
        }));
        assert!(ok.get("stats_note").is_none());
        assert_eq!(ok["health"]["current"], 5);
    }

    /// Regression guard for the empty-`stats` bug: `Stat` is a tolua class,
    /// so `pairs(Stat)` yields only metamethods and no id was ever read. The
    /// chunk must enumerate `Stat['.get']` and carry the real names as the
    /// fallback. Reverting to `pairs(Stat)` (or to guessed names) fails this.
    #[test]
    fn chunk_enumerates_the_stat_get_table_not_the_class() {
        let c = chunk(true, false);
        assert!(
            !c.contains("pairs(Stat)"),
            "the class table has only metamethods"
        );
        assert!(c.contains("pairs(Stat['.get'])"));
        for name in CLIENT_STAT_NAMES {
            assert!(c.contains(&format!("'{name}'")), "fallback lacks {name}");
        }
        for guessed in [
            "'Energy'",
            "'Mitigation'",
            "'Awareness'",
            "'InCombat'",
            "'Intellect'",
        ] {
            assert!(!c.contains(guessed), "{guessed} is not a client stat name");
        }
    }

    /// The reader must not require an `id` field or a plain Lua table (the
    /// old guard `type(s) == 'table' and s.id`) and must report where its
    /// names came from.
    #[test]
    fn chunk_reads_stats_without_requiring_id_or_a_plain_table() {
        let c = chunk(true, false);
        assert!(!c.contains("and s.id then"), "the id guard is back");
        assert!(c.contains("stat_field(s, 'current') ~= nil"));
        assert!(c.contains("type(s) == 'userdata'"));
        assert!(c.contains("source = source"));
        assert!(c.contains("me.stats, me.stats_diag = stats_of(P)"));
    }
}
