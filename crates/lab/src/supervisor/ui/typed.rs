//! The typed window readers: for each window UAT needs, its root windows
//! (walked as [`super::window_read`] does) and the owning module's state
//! read through the stock bindings the window's own Lua uses.
//!
//! Every name and binding below comes from the stock client UI Lua
//! (`SGWGame/Content/UI/Core/<Module>/*.lua`); the file each came from is
//! noted beside it. The extras run under `__jcall`, so a binding that is
//! missing on some build reads as null instead of failing the read.

use serde_json::{json, Value};

use super::inventory::ITEMS_FN;
use super::lua_json::list;
use super::window_read::{shape_read, walk_call, WalkOptions, WALK_FN};

/// One typed reader.
#[derive(Debug)]
pub struct TypedReader {
    pub kind: &'static str,
    pub aliases: &'static [&'static str],
    /// Root windows, walked in order; the first visible one is `primary`.
    pub roots: &'static [&'static str],
    /// Lua statements that fill the local table `extra`.
    pub extra: &'static str,
}

pub const READERS: &[TypedReader] = &[
    TypedReader {
        // Core/Vault/Vault.lua: personal bank; its slots are Container.Vault.
        kind: "vault",
        aliases: &["bank"],
        roots: &["VaultWin"],
        extra: "if Container and Container.Vault then extra.items = __lab_items(Container.Vault, 'Vault', false) end \
                extra.cash = __jcall(getCash)",
    },
    TypedReader {
        // Core/Team/TeamVault.layout, Core/Command/CommandVault.layout.
        kind: "org_vault",
        aliases: &["team_vault", "command_vault"],
        roots: &["TeamVaultWin", "CommandVaultWin"],
        extra: "for _, n in ipairs({ 'TeamVault', 'CommandVault', 'TeamBank', 'CommandBank' }) do \
                  if Container and Container[n] then extra[n] = __lab_items(Container[n], n, false) end \
                end",
    },
    TypedReader {
        // Core/Trainer/Trainer.lua: Trainer_Choices rows carry the ability id;
        // TrainerMod keeps cost/trainable/name per id.
        kind: "trainer",
        aliases: &[],
        roots: &["TrainerWin"],
        extra: "local abilities = {} \
                local n = __jcall(function() return Trainer_Choices:getItemCount() end) or 0 \
                for i = 0, n - 1 do \
                  local it = __jcall(function() return Trainer_Choices:getListboxItemFromIndex(i) end) \
                  if it then local id = it:getID() \
                    abilities[#abilities + 1] = { index = i, ability_id = id, name = TrainerMod and TrainerMod.name[id], \
                      cost = TrainerMod and TrainerMod.cost[id], trainable = TrainerMod and TrainerMod.trainable[id], \
                      selected = __jcall(function() return it:isSelected() end) } end \
                end \
                extra.abilities = abilities \
                extra.train_enabled = __jcall(function() return not Trainer_TrainBtn:isDisabled() end)",
    },
    TypedReader {
        // Core/DisciplineTrainer/DisciplineTrainer.lua.
        kind: "discipline_trainer",
        aliases: &[],
        roots: &["DiscTrainWin"],
        extra: "",
    },
    TypedReader {
        // Core/Crafting/*.lua: isCraftingAllowed(UICraftType.X) returns
        // (tool, machine); getKnownBlueprints(UICraftType.X) the list.
        kind: "crafting",
        aliases: &["craft"],
        roots: &["CraftingWin", "CraftWin", "AlloyWin", "ResearchWin", "ReverseEngWin"],
        extra: "if type(UICraftType) == 'table' then \
                  extra.allowed = {} extra.known_blueprints = {} \
                  for name, t in pairs(UICraftType) do \
                    local r = { pcall(isCraftingAllowed, t) } \
                    if r[1] then extra.allowed[name] = { tool = r[2], machine = r[3] } end \
                    local bp = __jcall(getKnownBlueprints, t) \
                    if type(bp) == 'table' then extra.known_blueprints[name] = #bp end \
                  end \
                end",
    },
    TypedReader {
        // Core/Pet/PetContainer.lua, PetInfo.lua: the pet is Unit.Pet.
        kind: "pet",
        aliases: &["pets"],
        roots: &["DefaultPetWin", "PetAbilityWin"],
        extra: "if Unit and Unit.Pet then \
                  extra.exists = __jcall(unitExists, Unit.Pet) \
                  if extra.exists then \
                    extra.name = __jcall(unitName, Unit.Pet) \
                    extra.level = __jcall(unitLevel, Unit.Pet) \
                    extra.mob_id = __jcall(unitMobId, Unit.Pet) \
                    if Stat then extra.health = __jcall(getUnitStat, Unit.Pet, Stat.Health) end \
                  end \
                end",
    },
    TypedReader {
        // Core/Organization/Organization.lua (roster is a MultiColumnList).
        kind: "organization",
        aliases: &["org", "team", "command", "squad"],
        roots: &["OrganizationWin", "SquadWin", "TeamEditorWin", "CommandEditorWin"],
        extra: "",
    },
    TypedReader {
        // Core/GateMail/GateMail.lua: headers are 0-based.
        kind: "mail",
        aliases: &["gatemail"],
        roots: &["GateMailInboxWin", "GateMailReadWin", "GateMailCreateWin"],
        extra: "local n = __jcall(mailGetHeaderCount) or 0 \
                extra.count = n \
                local headers = {} \
                for i = 0, math.min(n, 100) - 1 do headers[#headers + 1] = __jcall(mailGetHeaderInfo, i) end \
                extra.headers = headers",
    },
    TypedReader {
        // Core/Loot/Loot.lua: 1-based; type 0 is cash.
        kind: "loot",
        aliases: &[],
        roots: &["LootWin"],
        extra: "local n = __jcall(getLootCount) or 0 \
                extra.count = n \
                local items = {} \
                for i = 1, n do local li = __jcall(getLootInfo, i) \
                  if type(li) == 'table' then li.index = i end items[#items + 1] = li end \
                extra.items = items \
                extra.page = LootMod and LootMod.page",
    },
    TypedReader {
        // Core/DHD/DHD.lua.
        kind: "dhd",
        aliases: &[],
        roots: &["DHDWin"],
        extra: "extra.active = __jcall(isDHDActive)",
    },
    TypedReader {
        // Core/Dialog/Blurb.lua, Dialog.lua.
        kind: "dialog",
        aliases: &["blurb"],
        roots: &["DialogWin", "BlurbWin"],
        extra: "extra.active_text = __jcall(getActiveDialogText)",
    },
    TypedReader {
        // Core/Greet/Greet.lua: topics are 1-based.
        kind: "greet",
        aliases: &[],
        roots: &["GreetWin"],
        extra: "local n = __jcall(getGreetTopicCount) or 0 \
                local topics = {} \
                for i = 1, n do topics[#topics + 1] = { index = i, text = __jcall(getGreetTopicText, i), level = __jcall(getGreetTopicLevel, i) } end \
                extra.topics = topics",
    },
    TypedReader {
        // Core/Vendor/Vendor.lua: items are 0-based; VendorItem.VendorItem
        // is the buy list.
        kind: "vendor",
        aliases: &["merchant"],
        roots: &["VendorWin"],
        extra: "extra.name = __jcall(getVendorName) \
                local n = __jcall(getVendorItemCount) or 0 \
                extra.count = n \
                local items = {} \
                if VendorItem then \
                  for i = 0, math.min(n, 100) - 1 do local vi = __jcall(getVendorItemInfo, i, VendorItem.VendorItem) \
                    if type(vi) == 'table' then vi.index = i end items[#items + 1] = vi end \
                end \
                extra.items = items \
                extra.cash = __jcall(getCash)",
    },
    TypedReader {
        // Core/Trade/Trade.lua.
        kind: "trade",
        aliases: &[],
        roots: &["TradeWin"],
        extra: "",
    },
    TypedReader {
        // Core/Character/Character.lua.
        kind: "character",
        aliases: &["stats", "equipment"],
        roots: &["CharacterWin"],
        extra: "",
    },
];

pub fn find(kind: &str) -> Option<&'static TypedReader> {
    let k = kind.to_ascii_lowercase();
    READERS
        .iter()
        .find(|r| r.kind == k || r.aliases.contains(&k.as_str()))
}

pub fn kinds() -> Vec<&'static str> {
    READERS.iter().map(|r| r.kind).collect()
}

/// The chunk body for a typed read: walk each root, then fill `extra`.
pub fn chunk(r: &TypedReader, opts: WalkOptions) -> String {
    let walks = r
        .roots
        .iter()
        .map(|root| format!("  roots[#roots + 1] = {}", walk_call(root, opts)))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{WALK_FN}{ITEMS_FN}\nlocal roots = {{}}\n{walks}\nlocal extra = {{}}\n\
         local okx, ex = pcall(function()\n{}\nend)\n\
         if not okx then extra.error = tostring(ex) end\n\
         return __jenc({{ roots = roots, extra = extra }})",
        r.extra
    )
}

/// Shape a typed read: which root is open, its summary, the others'
/// visibility, and the module state.
pub fn shape(r: &TypedReader, v: &Value, include_nodes: bool) -> Value {
    let roots = list(&v["roots"]);
    let open: Vec<&Value> = roots
        .iter()
        .filter(|w| w["visible"].as_bool().unwrap_or(false))
        .collect();
    let primary = open.first().map(|w| shape_read(w, include_nodes));
    let others: Vec<Value> = roots
        .iter()
        .map(|w| json!({ "window": w["window"], "found": w["found"], "visible": w["visible"] }))
        .collect();
    json!({
        "kind": r.kind,
        "open": !open.is_empty(),
        "primary": primary,
        "open_windows": open.iter().skip(1).map(|w| shape_read(w, include_nodes)).collect::<Vec<_>>(),
        "roots": others,
        "state": v["extra"],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_uat_window_has_a_reader() {
        for k in [
            "vault",
            "bank",
            "trainer",
            "crafting",
            "pet",
            "organization",
            "mail",
            "loot",
            "dhd",
            "blurb",
            "dialog",
            "vendor",
            "greet",
        ] {
            assert!(find(k).is_some(), "{k}");
        }
        assert_eq!(find("BANK").unwrap().kind, "vault");
        assert!(find("nope").is_none());
    }

    #[test]
    fn kinds_and_aliases_are_unique() {
        let mut all: Vec<&str> = READERS.iter().map(|r| r.kind).collect();
        for r in READERS {
            all.extend(r.aliases);
        }
        let n = all.len();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), n);
    }

    #[test]
    fn chunk_walks_every_root_and_guards_the_extras() {
        let c = chunk(find("crafting").unwrap(), WalkOptions::default());
        for root in ["CraftingWin", "AlloyWin", "ReverseEngWin"] {
            assert!(c.contains(&format!("__lab_walk(\"{root}\"")), "{root}");
        }
        assert!(c.contains("local okx, ex = pcall(function()"));
        assert!(c.contains("pcall(isCraftingAllowed, t)"));
        assert!(c.contains("local function __lab_items"));
    }

    #[test]
    fn shape_reports_the_first_open_root_as_primary() {
        let v = json!({
            "roots": [
                { "window": "DialogWin", "found": true, "visible": false, "nodes": [] },
                { "window": "BlurbWin", "found": true, "visible": true, "nodes": [
                    { "name": "BlurbWin", "type": "DefaultFrameWindow_2", "visible": true, "enabled": true, "text": "Radio" }
                ] }
            ],
            "extra": { "active_text": "Report to the Colonel." }
        });
        let s = shape(find("dialog").unwrap(), &v, false);
        assert_eq!(s["open"], true);
        assert_eq!(s["primary"]["window"], "BlurbWin");
        assert_eq!(s["primary"]["title"], "Radio");
        assert_eq!(s["roots"][0]["visible"], false);
        assert_eq!(s["state"]["active_text"], "Report to the Colonel.");
        let closed = shape(
            find("dialog").unwrap(),
            &json!({ "roots": {}, "extra": {} }),
            false,
        );
        assert_eq!(closed["open"], false);
        assert!(closed["primary"].is_null());
    }
}
