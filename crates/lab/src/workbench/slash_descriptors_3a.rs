//! Source-backed, disabled native slash forms for patch 012 entries 109–135.

use super::commands::{
    ArgDescriptor, ArgKind, ArgRange, Availability, CommandChannel, CommandDescriptor,
    Compensation, Impact, ImpactKind, SelectionRule, TargetKind, TargetPolicy,
    COMMAND_METADATA_VERSION,
};
use super::slash_inventory::{SlashEntry, SlashParameterType};

/// Canonical patch-table words in source order; aliases are not inferred.
pub const NAMES: [&str; 27] = [
    "/gmregeneratecoverlinks",
    "/gmchangecoverweight",
    "/gmchangecoverstanceweight",
    "/gmhelpfull",
    "/gmtestsequence",
    "/gmrepairitem",
    "/gmgiveammo",
    "/gmgivefaction",
    "/gmsetfaction",
    "/gminvokeability",
    "/gmsettechskill",
    "/gmgiveblueprint",
    "/gmgivegearset",
    "/gmgiveinventory",
    "/gmdebugminigameinstance",
    "/gmdebugflash",
    "/gmshowlog",
    "/gmsetinfiniteammo",
    "/gmsetinvulnerable",
    "/gmsetomnipotent",
    "/gmsetignorefocus",
    "/gmsetignorehealth",
    "/gmsetfly",
    "/gmsetghost",
    "/gmsetspectator",
    "/gmsetnotarget",
    "/gmsetpvp",
];

const EVENTS: [&str; 27] = [
    "Event_SlashCmd_RegenerateCoverLinks",
    "Event_SlashCmd_ChangeCoverWeight",
    "Event_SlashCmd_ChangeCoverStanceWeight",
    "Event_SlashCmd_HelpFull",
    "Event_SlashCmd_TestSequence",
    "Event_SlashCmd_RepairItem",
    "Event_SlashCmd_GiveAmmo",
    "Event_SlashCmd_GiveFaction",
    "Event_SlashCmd_SetFaction",
    "Event_SlashCmd_InvokeAbility",
    "Event_SlashCmd_SetTechSkill",
    "Event_SlashCmd_GiveBlueprint",
    "Event_SlashCmd_GiveGearset",
    "Event_SlashCmd_GiveInventory",
    "Event_SlashCmd_DebugMinigameInstance",
    "Event_SlashCmd_DebugFlash",
    "Event_SlashCmd_ShowLog",
    "Event_SlashCmd_SetInfiniteAmmo",
    "Event_SlashCmd_SetInvulnerable",
    "Event_SlashCmd_SetOmnipotent",
    "Event_SlashCmd_SetIgnoreFocus",
    "Event_SlashCmd_SetIgnoreHealth",
    "Event_SlashCmd_SetFly",
    "Event_SlashCmd_SetGhost",
    "Event_SlashCmd_SetSpectator",
    "Event_SlashCmd_SetNoTarget",
    "Event_SlashCmd_SetPVP",
];

const SIGNATURES: [&str; 27] = [
    "normalLimit:Float:true,maxLinks:Integer:true,maxDistance:Float:true",
    "distance:Float:true,defCover:Float:true,offCover:Float:true,move:Float:true,crossPath:Float:true,cover:Float:true",
    "stance:String:true,distance:Float:true,defCover:Float:true,offCover:Float:true,move:Float:true,crossPath:Float:true,cover:Float:true",
    "", "", "ItemId:Integer:true,RepairRatio:Float:true", "", "", "", "", "", "", "", "",
    "instanceId:Integer:true,seed:Integer:true,techcomp:Integer:true", "",
    "on/off:Boolean:false", "on/off:Boolean:false", "on/off:Boolean:false", "",
    "on/off:Boolean:false", "on/off:Boolean:false", "on/off:Boolean:false", "on/off:Boolean:false",
    "on/off:Boolean:false", "on/off:Boolean:false", "on/off:Boolean:true",
];

struct Detail {
    category: &'static str,
    summary: &'static str,
    blocked: &'static str,
    target: TargetKind,
    impact: Option<ImpactKind>,
    precondition: &'static str,
}

macro_rules! detail {
    ($category:expr, $summary:expr, $blocked:expr, $target:expr, $impact:expr, $precondition:expr) => {
        Detail {
            category: $category,
            summary: $summary,
            blocked: $blocked,
            target: $target,
            impact: $impact,
            precondition: $precondition,
        }
    };
}

/// Build the exact 27 patch forms. Every action remains disabled until its
/// native event binding, accepted receipt and effect readback are proven.
pub fn descriptors_3a(entries: &[SlashEntry]) -> Result<Vec<CommandDescriptor>, String> {
    if entries.len() != NAMES.len() || entries.iter().map(|entry| entry.name.as_str()).ne(NAMES) {
        return Err("slash slice 109–135 differs from canonical patch-table order".into());
    }
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            if entry.access != "p" {
                return Err(format!("{} lost patch GM access filter", entry.name));
            }
            let signature = entry
                .parameters
                .iter()
                .map(|parameter| {
                    format!(
                        "{}:{:?}:{}",
                        parameter.name, parameter.parameter_type, parameter.required
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            if entry.event != EVENTS[index] || signature != SIGNATURES[index] {
                return Err(format!(
                    "{} event or parameter contract differs from patch 012",
                    entry.name
                ));
            }
            let detail = detail_for(&entry.name);
            let args = entry
                .parameters
                .iter()
                .map(|parameter| {
                    let (help, min, max) = argument_note(&entry.name, &parameter.name);
                    ArgDescriptor {
                        name: parameter.name.clone(),
                        kind: match parameter.parameter_type {
                            SlashParameterType::Boolean => ArgKind::Boolean,
                            SlashParameterType::Integer => ArgKind::Integer,
                            SlashParameterType::Float => ArgKind::Float,
                            SlashParameterType::String => ArgKind::Text,
                        },
                        required: parameter.required,
                        default: None,
                        range: (min.is_some() || max.is_some()).then_some(ArgRange { min, max }),
                        choices: Vec::new(),
                        help: help.into(),
                        sensitive: false,
                    }
                })
                .collect();
            Ok(CommandDescriptor {
                id: entry.name.clone(),
                channel: CommandChannel::Slash,
                category: detail.category.into(),
                aliases: Vec::new(),
                summary: detail.summary.into(),
                example: None,
                availability: Availability::Unsupported,
                disposition_reason: Some(detail.blocked.into()),
                manual_pending: true,
                args,
                target: TargetPolicy {
                    kind: detail.target,
                    selection_rule: SelectionRule::None,
                    argument_name: None,
                },
                impact: detail.impact.map(|kind| Impact {
                    kind,
                    summary: detail.summary.into(),
                    preconditions: vec![
                        "Server GameMaster authority must be checked if a method is received"
                            .into(),
                        detail.precondition.into(),
                    ],
                }),
                verify: None,
                compensation: Some(Compensation::None),
                metadata_version: COMMAND_METADATA_VERSION,
            })
        })
        .collect()
}

fn argument_note(command: &str, name: &str) -> (&'static str, Option<f64>, Option<f64>) {
    match (command, name) {
        ("/gmregeneratecoverlinks", "maxLinks") => (
            "TOML Integer versus method UINT32; safe work limit and negative conversion unproved",
            None,
            None,
        ),
        ("/gmregeneratecoverlinks", _) | ("/gmchangecoverweight", _) => (
            "Finite float required for a future bounded handler; accepted range unproved",
            None,
            None,
        ),
        ("/gmchangecoverstanceweight", "stance") => (
            "Stance vocabulary and quoting unproved; method declares WSTRING",
            None,
            None,
        ),
        ("/gmchangecoverstanceweight", _) => (
            "TOML Float but method declares WSTRING; conversion and range unproved",
            None,
            None,
        ),
        ("/gmrepairitem", "ItemId") => (
            "Item instance ID, not design ID; method takes signed INT32",
            Some(0.0),
            Some(i32::MAX as f64),
        ),
        ("/gmrepairitem", "RepairRatio") => (
            "Method takes FLOAT; valid repair ratio and vendor context unproved",
            None,
            None,
        ),
        ("/gmdebugminigameinstance", _) => (
            "Method takes signed INT32; accepted value range unproved",
            Some(i32::MIN as f64),
            Some(i32::MAX as f64),
        ),
        (_, "on/off") => (
            "Boolean text, omitted-value behavior and native byte encoding unproved",
            None,
            None,
        ),
        _ => (
            "Patch 012 typed parameter; no unlisted default or bound",
            None,
            None,
        ),
    }
}

fn detail_for(name: &str) -> Detail {
    use ImpactKind::{Irreversible, Transient};
    use TargetKind::{None as NoTarget, Player};
    match name {
        "/gmregeneratecoverlinks" => detail!("cover", "Potentially rebuild cover links in an unproved world/space scope", "No method 223 handler, bounded work contract, graph readback or native binding", NoTarget, Some(Irreversible), "World/space graph scope, finite values and safe maxLinks must be proved"),
        "/gmchangecoverweight" => detail!("cover", "Potentially change world/space cover scoring weights", "No method 224 handler, live weight readback or native binding", NoTarget, Some(Irreversible), "Finite weight range, propagation and prior settings must be established"),
        "/gmchangecoverstanceweight" => detail!("cover", "Potentially change stance-specific cover scoring", "No method 225 handler; TOML Float versus method WSTRING unresolved; no readback", NoTarget, Some(Irreversible), "Stance vocabulary, Float conversion and world/space scope are unproved"),
        "/gmhelpfull" => detail!("help", "Potentially display command help in the client", "Client help effect and native submission unproved; no server method", NoTarget, None, "Observe actual client help output"),
        "/gmtestsequence" => detail!("debug", "Unknown client test-sequence effect", "Client handler, server method, effect and readback unknown", NoTarget, None, "No safe effect contract exists"),
        "/gmrepairitem" => detail!("inventory", "Potential item-instance durability change", "Method 40 is unimplemented; native binding, authorization and ratio range unproved", NoTarget, None, "Exact owned item instance and durability/currency receipt required"),
        "/gmgiveammo" => detail!("inventory", "Slash ammo grant effect unknown", "No native server effect or binding; dot .giveammo is a different executor", NoTarget, None, "Do not borrow dot ammo type, count or readback"),
        "/gmgivefaction" => detail!("faction", "Unknown faction grant effect", "No target, arguments, handler or native binding", NoTarget, None, "Faction and recipient cannot be inferred"),
        "/gmsetfaction" => detail!("faction", "Unknown faction setter effect", "No value, target, handler or native binding", NoTarget, None, "No specific faction state is bound to this event"),
        "/gminvokeability" => detail!("combat", "Unknown ability invocation effect", "No ability, target, handler or native binding", NoTarget, None, "Do not infer a combat recipient from selection"),
        "/gmsettechskill" => detail!("crafting", "Unknown technical-skill mutation", "No skill ID, value, handler or native binding", NoTarget, None, "No persisted skill field is bound to this event"),
        "/gmgiveblueprint" => detail!("crafting", "Unknown blueprint grant", "No blueprint ID, owner, handler or native binding", NoTarget, None, "No recipe row is bound to this event"),
        "/gmgivegearset" => detail!("inventory", "Unknown gearset grant", "No gearset ID, recipient, handler or native binding", NoTarget, None, "Bulk inventory changes need exact instance receipts"),
        "/gmgiveinventory" => detail!("inventory", "Unknown bulk inventory effect", "No contents, owner, handler or native binding; gmGiveItem is distinct", NoTarget, None, "Do not borrow gmGiveItem arguments"),
        "/gmdebugminigameinstance" => detail!("minigame", "Request a caller minigame debug instance; server currently logs a stub", "MinigamePlayer 23 is unimplemented; native binding and accepted instance readback absent", Player, None, "A stub log proves receipt only, never an opened session"),
        "/gmdebugflash" => detail!("debug", "Unknown client Flash debug display", "Client handler effect and native submission unproved", NoTarget, None, "A client UI observation would be required"),
        "/gmshowlog" => detail!("debug", "Potentially change caller client log visibility", "Boolean omission, native submission and observed client result unproved", NoTarget, None, "Client log visibility must be observed; server reads cannot prove it"),
        "/gmsetinfiniteammo" => detail!("combat", "Native slash ammo effect unknown", "No native server effect/readback; dot .infiniteammo is separate", NoTarget, None, "Do not present the dot toggle as a slash inverse"),
        "/gmsetinvulnerable" => detail!("combat", "Unknown invulnerability effect", "No handler, effect or native binding; gmSetGodMode is distinct", NoTarget, None, "Do not infer damage protection from the name"),
        "/gmsetomnipotent" => detail!("combat", "Unknown potentially broad combat effect", "No handler, effect or native binding; TOML has no off argument", NoTarget, None, "No inverse or previous state is known"),
        "/gmsetignorefocus" => detail!("combat", "Unknown focus-cost effect", "No focus-cost handler, local side-effect proof or native binding", NoTarget, None, "Focus remaining stable does not prove a toggle"),
        "/gmsetignorehealth" => detail!("combat", "Unknown health/debug effect", "No health-ignore handler, local side-effect proof or native binding", NoTarget, None, "Do not equate this with god mode"),
        "/gmsetfly" => detail!("movement", "Potential caller fly physics and transient movement-validation bypass", "Method 221 exists, but native fly Boolean-to-wire polarity, binding and flag readback unproved", Player, Some(Transient), "GM gate and live caller required; onPhysics(0) disables validation, (1) restores it"),
        "/gmsetghost" => detail!("movement", "Potential caller ghost physics and the same transient movement-validation bypass", "Method 221 exists, but native ghost Boolean-to-wire polarity, binding and flag readback unproved", Player, Some(Transient), "GM gate and live caller required; fly and ghost share one server flag"),
        "/gmsetspectator" => detail!("debug", "Unknown client spectator effect", "No handler, client camera observation or native submission proof", NoTarget, None, "Do not conflate with minigame spectate"),
        "/gmsetnotarget" => detail!("combat", "Unknown no-target effect", "No handler, observer effect or native submission proof", NoTarget, None, "Do not promise AI, AoI or targetability changes"),
        "/gmsetpvp" => detail!("pvp", "Potential client PvP flag request, never a grant of attack rights", "No accepted slash handler or native Boolean binding; duel registry authority unchanged", Player, None, "Internal setPvPFlag is not GM-callable; engaged DuelRegistry pair still gates combat"),
        _ => unreachable!("slice names checked before detail lookup"),
    }
}
