//! Which ability the caller means, and where the player can reach it.
//!
//! Sources, all client-side: the hotbar (see [`super::super::hotbar`]) and
//! the known abilities of the Ability window's training trees
//! (`getTrainingTreeCount`, `getTrainableList(tree)`,
//! `getTrainableInfo(id).haveIt`, `getAbilityInfo(id).name`, the same calls
//! `AbilityMod.refreshAbilityTree` makes). The window shows tree `t`'s
//! list in order, so list index `i` is button `Ability_Button<i>`.
//! An ability granted outside the trees (GM `.giveability`) is known to
//! neither list until it is put on the bar; by id it still resolves, with
//! only the Lua fallback left to fire it.

use serde_json::{json, Value};

use super::super::hotbar::Hotbar;

/// Ability-window buttons per tree (`AbilityMod.MAX_BUTTONS`).
pub const WINDOW_BUTTONS: u32 = 30;

/// Build the known-abilities read. `probe_id` > 0 also reads that id's
/// `getAbilityInfo` (to name an ability that is in neither list).
pub fn known_chunk(probe_id: i64) -> String {
    format!(
        r#"local out = {{}}
local function add(...) out[#out + 1] = table.concat({{...}}, "\t") end
local function clean(s) if s == nil then return "" end return (string.gsub(tostring(s), "%c", " ")) end
local okc, n = pcall(getTrainingTreeCount)
if okc and type(n) == "number" then
  for t = 1, n do
    local okl, list = pcall(getTrainableList, t)
    if okl and type(list) == "table" then
      for i, id in pairs(list) do
        local okt, ti = pcall(getTrainableInfo, id)
        if okt and type(ti) == "table" and ti.id ~= nil and ti.haveIt then
          local oka, ai = pcall(getAbilityInfo, id)
          add("known", clean(id), t, clean(i), clean(oka and type(ai) == "table" and ai.name or ""))
        end
      end
    end
  end
else
  add("warn", "getTrainingTreeCount unavailable")
end
if {probe_id} > 0 then
  local oka, ai = pcall(getAbilityInfo, {probe_id})
  if oka and type(ai) == "table" and ai.id ~= nil then
    add("info", clean(ai.id), clean(ai.name), clean(ai.isWeaponAbility), clean(ai.isDeployAbility))
  end
end
return unpack(out)"#
    )
}

/// A known ability and where the Ability window shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct Known {
    pub id: i64,
    pub tree: u32,
    pub index: u32,
    pub name: String,
}

/// `known_chunk`'s results.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct KnownRead {
    pub known: Vec<Known>,
    /// `(id, name)` of the probed id, when the client knows it.
    pub info: Option<(i64, String)>,
    pub warnings: Vec<String>,
}

pub fn parse_known(lines: &[String]) -> KnownRead {
    let mut r = KnownRead::default();
    for line in lines {
        let f: Vec<&str> = line.split('\t').collect();
        match f.as_slice() {
            ["known", id, t, i, name] => {
                if let (Ok(id), Ok(tree), Ok(index)) =
                    (id.parse::<f64>(), t.parse::<f64>(), i.parse::<f64>())
                {
                    r.known.push(Known {
                        id: id as i64,
                        tree: tree as u32,
                        index: index as u32,
                        name: name.to_string(),
                    });
                }
            }
            ["info", id, name, ..] => {
                if let Ok(id) = id.parse::<f64>() {
                    r.info = Some((id as i64, name.to_string()));
                }
            }
            ["warn", w] => r.warnings.push(w.to_string()),
            _ => r.warnings.push(format!("unparsed line {line:?}")),
        }
    }
    r
}

/// What the caller asked for.
#[derive(Debug, Clone, PartialEq)]
pub enum Query {
    Id(i64),
    Name(String),
}

/// The resolved ability.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub id: i64,
    pub name: String,
    /// Hotbar buttons holding it, best first.
    pub buttons: Vec<u32>,
    /// Where the Ability window shows it (tree, index), when it is known.
    pub window: Option<(u32, u32)>,
}

impl Resolved {
    pub fn to_json(&self) -> Value {
        json!({
            "id": self.id,
            "name": self.name,
            "hotbar_buttons": self.buttons,
            "ability_window": self.window.map(|(t, i)| json!({ "tree": t, "index": i })),
        })
    }
}

/// Resolve a query against the hotbar and the known list. By name: an
/// exact case-insensitive match wins, else a unique substring match;
/// several distinct abilities is an error naming them.
pub fn resolve(q: &Query, hotbar: &Hotbar, known: &KnownRead) -> Result<Resolved, String> {
    // (id, name) candidates from both sources.
    let mut cands: Vec<(i64, String)> = Vec::new();
    for b in &hotbar.buttons {
        if let Some(id) = b.ability_id() {
            cands.push((id, b.name.clone()));
        }
    }
    for k in &known.known {
        cands.push((k.id, k.name.clone()));
    }
    let id = match q {
        Query::Id(id) => *id,
        Query::Name(n) => {
            let want = n.trim().to_lowercase();
            if want.is_empty() {
                return Err("empty ability name".into());
            }
            let pick = |exact: bool| {
                let mut ids: Vec<i64> = cands
                    .iter()
                    .filter(|(_, name)| {
                        let l = name.to_lowercase();
                        if exact {
                            l == want
                        } else {
                            l.contains(&want)
                        }
                    })
                    .map(|(id, _)| *id)
                    .collect();
                ids.sort_unstable();
                ids.dedup();
                ids
            };
            let exact = pick(true);
            let ids = if exact.is_empty() { pick(false) } else { exact };
            match ids.as_slice() {
                [one] => *one,
                [] => {
                    let mut names: Vec<&str> = cands.iter().map(|(_, n)| n.as_str()).collect();
                    names.sort_unstable();
                    names.dedup();
                    return Err(format!(
                        "no ability named {n:?} on the hotbar or in the known abilities (known: {})",
                        names.join(", ")
                    ));
                }
                many => {
                    let named: Vec<String> = many
                        .iter()
                        .map(|id| {
                            let name = cands
                                .iter()
                                .find(|(c, _)| c == id)
                                .map_or("", |(_, n)| n.as_str());
                            format!("{id} {name}")
                        })
                        .collect();
                    return Err(format!(
                        "ability name {n:?} is ambiguous: {}; pass ability_id",
                        named.join(", ")
                    ));
                }
            }
        }
    };
    let name = cands
        .iter()
        .find(|(c, n)| *c == id && !n.is_empty())
        .map(|(_, n)| n.clone())
        .or_else(|| {
            known
                .info
                .as_ref()
                .filter(|(i, _)| *i == id)
                .map(|(_, n)| n.clone())
        })
        .unwrap_or_default();
    let buttons = hotbar
        .buttons_for_ability(id)
        .iter()
        .map(|b| b.button)
        .collect();
    let window = known
        .known
        .iter()
        .find(|k| k.id == id && k.index >= 1 && k.index <= WINDOW_BUTTONS)
        .map(|k| (k.tree, k.index));
    Ok(Resolved {
        id,
        name,
        buttons,
        window,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supervisor::combat::hotbar::parse_hotbar;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    fn bar() -> Hotbar {
        parse_hotbar(&s(&[
            "button\t1\tActionButtons_1Button\t1\t7\t1\tAbility\t1100\tPistol Shot\t-1\t0\t0\tkey=49\t",
            "button\t2\tActionButtons_2Button\t1\t8\t2\tItem\t2893\tHealth Slappack\t3\t0\t0\t\t",
        ]))
        .unwrap()
    }

    fn known() -> KnownRead {
        parse_known(&s(&[
            "known\t1100\t1\t1\tPistol Shot",
            "known\t1200\t1\t2\tSuppressive Fire",
            "known\t1201\t2\t4\tSuppressive Fire II",
            "info\t4000\tGM Ability\ttrue\tfalse",
        ]))
    }

    #[test]
    fn known_lines_parse() {
        let k = known();
        assert_eq!(k.known.len(), 3);
        assert_eq!(k.known[2].tree, 2);
        assert_eq!(k.info, Some((4000, "GM Ability".into())));
        assert!(parse_known(&s(&["junk"])).warnings[0].contains("junk"));
        let c = known_chunk(4000);
        assert!(c.contains("getTrainableList, t"));
        assert!(c.contains("pcall(getAbilityInfo, 4000)"));
    }

    #[test]
    fn by_id_finds_the_bar_and_the_window() {
        let r = resolve(&Query::Id(1100), &bar(), &known()).unwrap();
        assert_eq!(r.name, "Pistol Shot");
        assert_eq!(r.buttons, vec![1]);
        assert_eq!(r.window, Some((1, 1)));
        // An item on the bar is never an ability.
        let item = resolve(&Query::Id(2893), &bar(), &known()).unwrap();
        assert!(item.buttons.is_empty());
    }

    #[test]
    fn a_gm_granted_id_resolves_through_the_probe_only() {
        let r = resolve(&Query::Id(4000), &bar(), &known()).unwrap();
        assert_eq!(r.name, "GM Ability");
        assert!(r.buttons.is_empty() && r.window.is_none());
    }

    #[test]
    fn exact_name_beats_substring() {
        let r = resolve(&Query::Name("suppressive fire".into()), &bar(), &known()).unwrap();
        assert_eq!(r.id, 1200);
        assert_eq!(r.window, Some((1, 2)));
    }

    #[test]
    fn substring_must_be_unique() {
        let r = resolve(&Query::Name("pistol".into()), &bar(), &known()).unwrap();
        assert_eq!(r.id, 1100);
        let e = resolve(&Query::Name("fire".into()), &bar(), &known()).unwrap_err();
        assert!(e.contains("ambiguous") && e.contains("1200") && e.contains("1201"));
        let none = resolve(&Query::Name("zat blast".into()), &bar(), &known()).unwrap_err();
        assert!(none.contains("no ability named"));
        assert!(resolve(&Query::Name("  ".into()), &bar(), &known()).is_err());
    }
}
