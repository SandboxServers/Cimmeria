//! `client_inventory`: every container the client knows (bags, equipment,
//! the bandolier with its ammo, and the vault or team/command vaults while
//! their windows are open) plus cash, read through the stock bindings
//! (`getContainerSize`, `getItemIDForSlot`, `getQuantityForSlot`, ...).
//!
//! Container ids come from the client's own `Container` table at run time,
//! never from a hard-coded list. A container the client has not loaded
//! reads as size 0 and is listed under `unloaded`.
//!
//! Snapshots are kept by label in the supervisor so a caller can take one
//! before an action and diff against it after ([`diff`]).

use std::collections::BTreeMap;
use std::time::Instant;

use serde_json::{json, Value};

use super::lua_json::list;
use super::{memory, stamp_read, Supervisor};
use crate::supervisor::flows::widgets::lua_quote;

/// Lua (after the JSON prelude): `__lab_cnames()` maps container id to
/// its `Container.*` name, and `__lab_items(id, name, include_empty)`
/// reads one container's slots (1-based, as the inventory UI numbers them).
pub const ITEMS_FN: &str = r#"
local function __lab_cnames()
  local names = {}
  if type(Container) == 'table' then
    for k, v in pairs(Container) do if type(v) == 'number' then names[v] = k end end
  end
  return names
end
local function __lab_items(cid, cname, include_empty)
  local size = __jcall(getContainerSize, cid) or 0
  local slots = {}
  for s = 1, size do
    local empty = __jcall(isSlotEmpty, cid, s)
    if empty == nil then empty = (__jcall(getItemIDForSlot, cid, s) or 0) == 0 end
    if not empty then
      slots[#slots + 1] = {
        slot = s,
        item_id = __jcall(getItemIDForSlot, cid, s),
        name = __jcall(getNameForSlot, cid, s),
        qty = __jcall(getQuantityForSlot, cid, s),
        quality = __jcall(getQualityForSlot, cid, s),
        icon = __jcall(getIconForSlot, cid, s),
      }
    elseif include_empty then
      slots[#slots + 1] = { slot = s, empty = true }
    end
  end
  return { id = cid, name = cname, size = size, slots = slots }
end
"#;

/// The reader chunk body. `only` limits the read to these `Container.*`
/// names (case-insensitive); empty reads every container.
pub fn read_chunk(only: &[String], include_empty: bool) -> String {
    let filter = only
        .iter()
        .map(|n| format!("[{}] = true", lua_quote(&n.to_ascii_lowercase())))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        r#"{ITEMS_FN}
local only = {{ {filter} }}
local any = next(only) ~= nil
local names = __lab_cnames()
local ids = {{}}
for id, _ in pairs(names) do ids[#ids + 1] = id end
table.sort(ids)
local containers, unloaded = {{}}, {{}}
for _, id in ipairs(ids) do
  local n = names[id]
  if not any or only[string.lower(n)] then
    local c = __lab_items(id, n, {include_empty})
    if c.size > 0 then containers[#containers + 1] = c else unloaded[#unloaded + 1] = n end
  end
end
local band = nil
if Container and Container.Bandolier then
  local active = __jcall(getActiveSlotForContainer, Container.Bandolier)
  local size = __jcall(getContainerSize, Container.Bandolier) or 0
  local slots = {{}}
  for s = 1, size do
    local stat = nil
    if Stat and Stat.AmmoSlot1 then stat = __jcall(getUnitStat, Unit.Player, Stat.AmmoSlot1 + s - 1) end
    slots[#slots + 1] = {{
      slot = s,
      ammo_type = __jcall(getCurrentAmmoType, Container.Bandolier, s),
      ammo = stat and stat.current or nil,
      ammo_max = stat and stat.max or nil,
    }}
  end
  band = {{ active_slot = active, slots = slots }}
end
return __jenc({{ cash = __jcall(getCash), containers = containers, unloaded = unloaded, bandolier = band }})"#
    )
}

/// `(container, slot)` → the slot's item fields, from a read.
fn slot_map(read: &Value) -> BTreeMap<(String, i64), Value> {
    let mut m = BTreeMap::new();
    for c in list(&read["containers"]) {
        let cname = c["name"].as_str().unwrap_or_default().to_string();
        for s in list(&c["slots"]) {
            if s.get("empty").is_some() {
                continue;
            }
            let slot = s["slot"].as_i64().unwrap_or_default();
            m.insert(
                (cname.clone(), slot),
                json!({ "item_id": s["item_id"], "name": s["name"], "qty": s["qty"] }),
            );
        }
    }
    m
}

/// Total quantity per item name across every container of a read.
fn totals(read: &Value) -> BTreeMap<String, i64> {
    let mut t = BTreeMap::new();
    for v in slot_map(read).values() {
        let name = v["name"].as_str().unwrap_or("?").to_string();
        *t.entry(name).or_insert(0) += v["qty"].as_i64().unwrap_or(1);
    }
    t
}

/// What changed between two reads: per-slot changes, per-item quantity
/// deltas (so a move between slots nets to zero there) and the cash delta.
pub fn diff(before: &Value, after: &Value) -> Value {
    let b = slot_map(before);
    let a = slot_map(after);
    let mut keys: Vec<&(String, i64)> = b.keys().chain(a.keys()).collect();
    keys.sort();
    keys.dedup();
    let changes: Vec<Value> = keys
        .into_iter()
        .filter(|k| b.get(*k) != a.get(*k))
        .map(|k| {
            json!({
                "container": k.0,
                "slot": k.1,
                "before": b.get(k).cloned().unwrap_or(Value::Null),
                "after": a.get(k).cloned().unwrap_or(Value::Null),
            })
        })
        .collect();
    let tb = totals(before);
    let ta = totals(after);
    let mut names: Vec<&String> = tb.keys().chain(ta.keys()).collect();
    names.sort();
    names.dedup();
    let by_item: Vec<Value> = names
        .into_iter()
        .filter_map(|n| {
            let x = tb.get(n).copied().unwrap_or(0);
            let y = ta.get(n).copied().unwrap_or(0);
            (x != y).then(|| json!({ "name": n, "before": x, "after": y, "delta": y - x }))
        })
        .collect();
    let cash_delta = match (before["cash"].as_i64(), after["cash"].as_i64()) {
        (Some(x), Some(y)) => json!(y - x),
        _ => Value::Null,
    };
    json!({
        "changed": !changes.is_empty() || cash_delta.as_i64().unwrap_or(0) != 0,
        "slot_changes": changes,
        "by_item": by_item,
        "cash_delta": cash_delta,
    })
}

/// Find items in a read by instance id, or by name (case-insensitive,
/// substring). Returns `[{container, container_id, slot, item_id, name, qty}]`.
pub fn find_items(read: &Value, item_id: Option<i64>, name: Option<&str>) -> Vec<Value> {
    let needle = name.map(str::to_ascii_lowercase);
    let mut out = Vec::new();
    for c in list(&read["containers"]) {
        for s in list(&c["slots"]) {
            if s.get("empty").is_some() {
                continue;
            }
            let id_ok = item_id.is_none_or(|id| s["item_id"].as_i64() == Some(id));
            let name_ok = needle.as_deref().is_none_or(|n| {
                s["name"]
                    .as_str()
                    .is_some_and(|x| x.to_ascii_lowercase().contains(n))
            });
            if id_ok && name_ok {
                out.push(json!({
                    "container": c["name"],
                    "container_id": c["id"],
                    "slot": s["slot"],
                    "item_id": s["item_id"],
                    "name": s["name"],
                    "qty": s["qty"],
                }));
            }
        }
    }
    out
}

impl Supervisor {
    /// One inventory read (no snapshot bookkeeping).
    pub async fn inventory_read(
        &self,
        only: &[String],
        include_empty: bool,
    ) -> Result<Value, String> {
        self.lua_json(&read_chunk(only, include_empty)).await
    }

    /// `client_inventory`: read, optionally store it under `snapshot`, and
    /// optionally diff it against an earlier snapshot.
    pub async fn inventory(
        &self,
        only: &[String],
        include_empty: bool,
        snapshot: Option<&str>,
        diff_against: Option<&str>,
    ) -> Result<Value, String> {
        let t0 = Instant::now();
        let read = self.inventory_read(only, include_empty).await?;
        let mut out = read.clone();
        if let Some(label) = diff_against {
            let before = memory()
                .lock()
                .map_err(|_| "inventory memory poisoned".to_string())?
                .snapshots
                .get(label)
                .cloned()
                .ok_or_else(|| format!("no inventory snapshot named {label:?}"))?;
            out["diff"] = diff(&before, &read);
            out["diff_against"] = json!(label);
        }
        if let Some(label) = snapshot {
            memory()
                .lock()
                .map_err(|_| "inventory memory poisoned".to_string())?
                .snapshots
                .insert(label.to_string(), read);
            out["snapshot"] = json!(label);
        }
        stamp_read(&mut out, t0.elapsed().as_millis() as u64);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(main: Value, cash: i64) -> Value {
        json!({
            "cash": cash,
            "containers": [
                { "id": 1, "name": "Main", "size": 40, "slots": main },
                { "id": 4, "name": "Chest", "size": 1, "slots": {} }
            ],
            "unloaded": ["Vault"],
        })
    }

    #[test]
    fn a_consumed_stack_shows_as_a_slot_change_and_an_item_delta() {
        let before = read(
            json!([{ "slot": 3, "item_id": 900, "name": "Medkit", "qty": 5 }]),
            100,
        );
        let after = read(
            json!([{ "slot": 3, "item_id": 900, "name": "Medkit", "qty": 4 }]),
            100,
        );
        let d = diff(&before, &after);
        assert_eq!(d["changed"], true);
        assert_eq!(d["slot_changes"][0]["slot"], 3);
        assert_eq!(
            d["by_item"][0],
            json!({ "name": "Medkit", "before": 5, "after": 4, "delta": -1 })
        );
        assert_eq!(d["cash_delta"], 0);
    }

    /// A move between slots changes two slots but no totals.
    #[test]
    fn a_move_nets_to_zero_by_item() {
        let before = read(
            json!([{ "slot": 1, "item_id": 7, "name": "Zat", "qty": 1 }]),
            0,
        );
        let after = read(
            json!([{ "slot": 9, "item_id": 7, "name": "Zat", "qty": 1 }]),
            0,
        );
        let d = diff(&before, &after);
        assert_eq!(d["slot_changes"].as_array().unwrap().len(), 2);
        assert!(d["by_item"].as_array().unwrap().is_empty());
    }

    #[test]
    fn a_split_shows_the_new_stack_and_cash_counts_as_a_change() {
        let before = read(
            json!([{ "slot": 1, "item_id": 7, "name": "Ammo", "qty": 10 }]),
            50,
        );
        let after = read(
            json!([
                { "slot": 1, "item_id": 7, "name": "Ammo", "qty": 9 },
                { "slot": 2, "item_id": 8, "name": "Ammo", "qty": 1 }
            ]),
            75,
        );
        let d = diff(&before, &after);
        assert_eq!(d["slot_changes"].as_array().unwrap().len(), 2);
        assert!(d["by_item"].as_array().unwrap().is_empty());
        assert_eq!(d["cash_delta"], 25);
        let same = diff(&before, &before);
        assert_eq!(same["changed"], false);
    }

    #[test]
    fn find_items_by_id_or_name_skips_empty_slots() {
        let r = read(
            json!([
                { "slot": 1, "empty": true },
                { "slot": 2, "item_id": 44, "name": "Tok'ra Medkit", "qty": 2 }
            ]),
            0,
        );
        assert_eq!(find_items(&r, Some(44), None)[0]["slot"], 2);
        assert_eq!(find_items(&r, None, Some("medkit"))[0]["container"], "Main");
        assert!(find_items(&r, Some(45), None).is_empty());
    }

    #[test]
    fn read_chunk_filters_by_lowercased_container_name() {
        let c = read_chunk(&["Main".into(), "Bandolier".into()], false);
        assert!(c.contains(r#"["main"] = true, ["bandolier"] = true"#));
        assert!(c.contains("__lab_items(id, n, false)"));
        assert!(c.contains("getActiveSlotForContainer, Container.Bandolier"));
        let all = read_chunk(&[], true);
        assert!(all.contains("local only = {  }"));
        assert!(all.contains("__lab_items(id, n, true)"));
    }
}
