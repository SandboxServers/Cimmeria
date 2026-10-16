//! Source-backed, disabled native slash forms for patch 012 entries 28–54.

use super::commands::{
    ArgDescriptor, ArgKind, ArgRange, Availability, CommandChannel, CommandDescriptor,
    Compensation, Impact, ImpactKind, SelectionRule, TargetKind, TargetPolicy, VerifyRecipe,
    COMMAND_METADATA_VERSION,
};
use super::slash_inventory::{SlashEntry, SlashParameterType};

/// Canonical patch-table order; changing a word is a contract change, not an alias.
pub const NAMES: [&str; 27] = [
    "/gmgiverespawner",
    "/gmgiveexpertise",
    "/gmgiveappliedsciencepoints",
    "/gmgiveracialparadigmlevels",
    "/gmsetgodmode",
    "/gmsetnoxp",
    "/gmsetnodamage",
    "/gmsetnoaggro",
    "/gmsetspeed",
    "/gmsethealth",
    "/gmsethealthmax",
    "/gmsetfocus",
    "/gmsetfocusmax",
    "/gmsetflag",
    "/gmsetlevel",
    "/gmresetabilities",
    "/gmgiveallabilities",
    "/gmrespec",
    "/gmsettarget",
    "/gmsetmobstance",
    "/gmsetmobabilityset",
    "/gmdhd",
    "/gmgoto",
    "/gmsummon",
    "/gmgotolocation",
    "/gmgotoxyz",
    "/gmreloadorganizations",
];

const EVENTS: [&str; 27] = [
    "Event_SlashCmd_GiveRespawner",
    "Event_SlashCmd_GiveExpertise",
    "Event_SlashCmd_GiveAppliedSciencePoints",
    "Event_SlashCmd_GiveRacialParadigmLevels",
    "Event_SlashCmd_SetGodMode",
    "Event_SlashCmd_SetNoXP",
    "Event_SlashCmd_SetNoDamageTimedMode",
    "Event_SlashCmd_SetNoAggro",
    "Event_SlashCmd_SetSpeed",
    "Event_SlashCmd_SetHealth",
    "Event_SlashCmd_SetHealthMax",
    "Event_SlashCmd_SetFocus",
    "Event_SlashCmd_SetFocusMax",
    "Event_SlashCmd_SetFlag",
    "Event_SlashCmd_SetLevel",
    "Event_SlashCmd_ResetAbilities",
    "Event_SlashCmd_GiveAllAbilities",
    "Event_SlashCmd_Respec",
    "Event_SlashCmd_SetTarget",
    "Event_SlashCmd_SetMobStance",
    "Event_SlashCmd_SetMobAbilitySet",
    "Event_SlashCmd_DHD",
    "Event_SlashCmd_Goto",
    "Event_SlashCmd_Summon",
    "Event_SlashCmd_GotoLocation",
    "Event_SlashCmd_GotoXYZ",
    "Event_SlashCmd_ReloadOrganizations",
];

const SIGNATURES: [&str; 27] = [
    "RespawnerMobId:Integer:true",
    "disciplineId:Integer:true,expertise:Integer:true",
    "points:Integer:true",
    "racialParadigmId:Integer:true,levels:Integer:true",
    "on/off:Boolean:true",
    "",
    "",
    "on/off:Boolean:true",
    "Multiplier:Float:true",
    "Amount:Integer:true,TargetId:Integer:true",
    "Amount:Integer:true,TargetId:Integer:true",
    "Amount:Integer:true,TargetId:Integer:true",
    "Amount:Integer:true,TargetId:Integer:true",
    "aFlagId:Integer:true,aForceVal:Integer:true",
    "Level:Integer:true",
    "",
    "",
    "",
    "NameOrID:String:true",
    "StanceID:Integer:true",
    "abilitySetId:Integer:false",
    "DestinationId:Integer:true",
    "TargetName:String:true",
    "TargetName:String:true",
    "WorldName:String:true,X-Coordinate:Float:true,Y-Coordinate:Float:true,Z-Coordinate:Float:true",
    "X-Coordinate:Float:true,Y-Coordinate:Float:true,Z-Coordinate:Float:true",
    "",
];

struct Detail {
    category: &'static str,
    summary: &'static str,
    blocked: &'static str,
    target: TargetKind,
    selection: SelectionRule,
    impact: Option<ImpactKind>,
    preconditions: &'static str,
    reads: &'static str,
    fields: &'static str,
    recovery: &'static str,
}

macro_rules! detail {
    ($category:expr, $summary:expr, $blocked:expr, $target:expr, $selection:expr,
     $impact:expr, $preconditions:expr, $reads:expr, $fields:expr, $recovery:expr) => {
        Detail {
            category: $category,
            summary: $summary,
            blocked: $blocked,
            target: $target,
            selection: $selection,
            impact: $impact,
            preconditions: $preconditions,
            reads: $reads,
            fields: $fields,
            recovery: $recovery,
        }
    };
}

/// Build this exact 27-word slice from parsed patch TOML, preserving each
/// parameter's spelling, type, required flag and order. No form is executable.
pub fn descriptors_1b(entries: &[SlashEntry]) -> Result<Vec<CommandDescriptor>, String> {
    if entries.len() != NAMES.len() || entries.iter().map(|entry| entry.name.as_str()).ne(NAMES) {
        return Err("slash slice 28–54 differs from canonical patch-table order".into());
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
                    selection_rule: detail.selection,
                    argument_name: match entry.name.as_str() {
                        "/gmsethealth" | "/gmsethealthmax" | "/gmsetfocus" | "/gmsetfocusmax" => {
                            Some("TargetId".into())
                        }
                        "/gmgoto" | "/gmsummon" => Some("TargetName".into()),
                        _ => None,
                    },
                },
                impact: detail.impact.map(|kind| Impact {
                    kind,
                    summary: detail.summary.into(),
                    preconditions: vec![
                        "Server GameMaster authority is required".into(),
                        detail.preconditions.into(),
                    ],
                }),
                verify: (!detail.reads.is_empty()).then(|| VerifyRecipe {
                    read_tools: detail.reads.split('|').map(str::to_owned).collect(),
                    comparison_fields: detail.fields.split('|').map(str::to_owned).collect(),
                }),
                compensation: Some(if detail.recovery.is_empty() {
                    Compensation::None
                } else {
                    Compensation::Manual(detail.recovery.into())
                }),
                metadata_version: COMMAND_METADATA_VERSION,
            })
        })
        .collect()
}

fn argument_note(command: &str, name: &str) -> (&'static str, Option<f64>, Option<f64>) {
    match (command, name) {
        ("/gmgiveexpertise", "disciplineId" | "expertise")
        | ("/gmgiveappliedsciencepoints", "points") =>
            ("Positive signed 32-bit value; Base may refuse or cap the grant", Some(1.0), Some(i32::MAX as f64)),
        ("/gmsethealth" | "/gmsethealthmax" | "/gmsetfocus" | "/gmsetfocusmax", "Amount") =>
            ("Nonnegative signed 32-bit value; stat bounds may clamp it", Some(0.0), Some(i32::MAX as f64)),
        ("/gmsethealth" | "/gmsethealthmax" | "/gmsetfocus" | "/gmsetfocusmax", "TargetId") =>
            ("Explicit same-space entity ID; 0 means the caller. Cell wire is INT64", Some(0.0), None),
        ("/gmsetgodmode", _) =>
            ("Boolean spelling and native UINT8 serialization remain unproved", None, None),
        ("/gmsetnoaggro", _) =>
            ("Boolean spelling and effect are unproved", None, None),
        ("/gmsetmobabilityset", _) =>
            ("Optional in TOML with no default; candidate server method requires INT32 bytes", Some(i32::MIN as f64), Some(i32::MAX as f64)),
        ("/gmdhd", _) =>
            ("Positive destination ID; candidate cell wire is signed INT8, encoding unproved", Some(1.0), Some(127.0)),
        ("/gmgoto" | "/gmsummon", _) =>
            ("TOML says TargetName; candidate server method accepts numeric entity ID only. Quoting unproved", None, None),
        ("/gmsettarget", _) =>
            ("TOML says NameOrID; candidate server method accepts numeric text only; 0 clears", None, None),
        ("/gmgotolocation", "WorldName") =>
            ("Destination world name; native space/quote handling unproved", None, None),
        ("/gmgotolocation" | "/gmgotoxyz", _) =>
            ("Finite coordinate; all-zero location chooses authored entry point for /gmgotolocation", None, None),
        ("/gmsetflag", _) =>
            ("Flag IDs, force values and ref-count behavior have no safe command contract", None, None),
        _ => ("Patch 012 typed parameter; no unlisted default or bound", None, None),
    }
}

fn detail_for(name: &str) -> Detail {
    use ImpactKind::{Persistent, Transient};
    use SelectionRule::{
        ArgumentRequired, ArgumentRequiredOther, ArgumentRequiredSelfZero, None as NoSelection,
        Required,
    };
    use TargetKind::{Being, Mob, None as NoTarget, Player};
    match name {
        "/gmgiverespawner" => detail!("grant", "Respawner grant effect is unknown", "No server handler or native event binding", NoTarget, NoSelection, None, "Respawner validity and target path unknown", "", "", ""),
        "/gmgiveexpertise" => detail!("crafting grant", "Add caller discipline expertise, capped at 100; Base commits later", "Native binding and accepted Base receipt unproved", Player, NoSelection, Some(Persistent), "Caller character and crafting DB pool; both values positive", "server_db_query", "sgw_player.player_id|sgw_player.discipline_ids|sgw_player_discipline_expertise.player_id|sgw_player_discipline_expertise.discipline_id|sgw_player_discipline_expertise.expertise", "Reconcile exact old expertise and discipline membership; clamp loses delta"),
        "/gmgiveappliedsciencepoints" => detail!("crafting grant", "Add caller applied-science points, saturating at i32 maximum", "Native binding and accepted Base receipt unproved", Player, NoSelection, Some(Persistent), "Caller character and DB pool; points positive", "server_db_query", "sgw_player.player_id|sgw_player.applied_science_points", "Manually reconcile old balance and intervening spending; saturation loses delta"),
        "/gmgiveracialparadigmlevels" => detail!("grant", "Racial-paradigm grant effect is unknown", "No server handler or native binding", NoTarget, NoSelection, None, "Paradigm ID, level bounds and target path unknown", "", "", ""),
        "/gmsetgodmode" => detail!("combat", "Toggle caller's transient god-mode damage protection", "Native Boolean serialization and binding unproved", Player, NoSelection, Some(Transient), "Caller entity must exist", "", "", "Switch off in same session or relog; missed combat damage is not restored"),
        "/gmsetnoxp" => detail!("progression", "XP suppression effect is unknown", "No server handler or native binding", NoTarget, NoSelection, None, "Toggle semantics unknown; TOML has no argument", "", "", ""),
        "/gmsetnodamage" => detail!("combat", "Timed damage immunity effect is unknown", "No handler, duration contract or native binding", NoTarget, NoSelection, None, "TOML has no Boolean argument", "", "", ""),
        "/gmsetnoaggro" => detail!("combat", "Aggro suppression effect is unknown", "No server handler or native Boolean binding", NoTarget, NoSelection, None, "Target and lifetime unknown", "", "", ""),
        "/gmsetspeed" => detail!("movement", "Speed multiplier effect is unknown", "No handler, safe range or native binding", NoTarget, NoSelection, None, "Target and multiplier policy unknown", "", "", ""),
        "/gmsethealth" => detail!("stats", "Set same-space target's current Health, subject to stat clamp", "Native binding and Integer-to-INT64 target encoding unproved", Being, ArgumentRequiredSelfZero, Some(Transient), "Explicit TargetId, 0=self; nonnegative Amount; target has Health stat", "server_entity_get", "health_cur", "Manually restore captured current Health after accounting for combat"),
        "/gmsethealthmax" => detail!("stats", "Set same-space target's maximum Health; current may clamp", "Native binding and Integer-to-INT64 target encoding unproved", Being, ArgumentRequiredSelfZero, Some(Transient), "Explicit TargetId, 0=self; nonnegative Amount; target has Health stat", "server_entity_get", "health_max|health_cur", "Manually restore captured max and current Health after combat"),
        "/gmsetfocus" => detail!("stats", "Set same-space target's current Focus, subject to stat clamp", "Native binding and Integer-to-INT64 target encoding unproved", Being, ArgumentRequiredSelfZero, Some(Transient), "Explicit TargetId, 0=self; nonnegative Amount; target has Focus stat", "server_entity_get", "focus_cur", "Manually restore captured current Focus after spend or regen"),
        "/gmsetfocusmax" => detail!("stats", "Set same-space target's maximum Focus; current may clamp", "Native binding and Integer-to-INT64 target encoding unproved", Being, ArgumentRequiredSelfZero, Some(Transient), "Explicit TargetId, 0=self; nonnegative Amount; target has Focus stat", "server_entity_get", "focus_max|focus_cur", "Manually restore captured max and current Focus"),
        "/gmsetflag" => detail!("state", "Force-set flag effect is unknown", "No handler, ref-count-safe flag contract or native binding", NoTarget, NoSelection, None, "Flag ID, force value and target policy unknown", "", "", ""),
        "/gmsetlevel" => detail!("progression", "Level change effect is unknown", "No server handler or native binding", NoTarget, NoSelection, None, "Level range, rescale and persistence unknown", "", "", ""),
        "/gmresetabilities" => detail!("abilities", "Persistently reset caller ability grants and refund spent tree points", "Native binding and accepted Base receipt unproved", Player, NoSelection, Some(Persistent), "Caller character, DB row and start profile required", "server_db_query", "sgw_player.player_id|sgw_player.abilities|sgw_player.training_points|sgw_player.trained_abilities|sgw_player.tree_points_spent|sgw_player_ability_grants.player_id|sgw_player_ability_grants.ability_id|sgw_player_ability_grants.source_kind|sgw_player_ability_grants.source_id", "Manually reconstruct captured abilities, provenance and tree state; no inverse"),
        "/gmgiveallabilities" => detail!("abilities", "Persistently append missing archetype-tree abilities with GM provenance", "Native binding and accepted Base receipt unproved", Player, NoSelection, Some(Persistent), "Caller character, archetype tree and DB row required", "server_db_query", "sgw_player.player_id|sgw_player.abilities|sgw_player_ability_grants.player_id|sgw_player_ability_grants.ability_id|sgw_player_ability_grants.source_kind|sgw_player_ability_grants.source_id", "Manually restore prior ability and provenance rows; reset is not a precise inverse"),
        "/gmrespec" => detail!("abilities", "GM respec effect is unknown", "No GM handler or native binding; trainer reset is different", NoTarget, NoSelection, None, "Target and cost policy unknown", "", "", ""),
        "/gmsettarget" => detail!("target", "Store caller current target ID and notify witnesses; 0 clears", "Native string quoting and binding unproved", Player, NoSelection, Some(Transient), "Caller exists; NameOrID must be nonnegative numeric text", "server_entity_get", "current_target_id", "Set captured prior ID or 0 in same session; target may have vanished"),
        "/gmsetmobstance" => detail!("npc", "Mob stance effect is unknown", "No handler, stance enum or native binding", NoTarget, NoSelection, None, "Target class and selection semantics unknown", "", "", ""),
        "/gmsetmobabilityset" => detail!("npc", "Replace selected mob's transient known abilities; may interrupt cast", "Optional TOML argument has no default but server requires INT32; native binding unproved", Mob, Required, Some(Transient), "Selected same-space live mob and loaded ability set required", "", "", "Prior live ability list needs a supported read and restore path; respawn may differ"),
        "/gmdhd" => detail!("travel", "Dial another world; arrival may persist world, position and learned gates", "Native binding and Integer-to-signed-INT8 encoding unproved", Player, NoSelection, Some(Persistent), "Known dialable destination, standable arrival and accepted GateTravel", "server_sessions|server_entity_get|server_db_query", "space_id|position|sgw_player.world_location|sgw_player.pos_x/y/z|sgw_player.known_stargates", "Manual return travel cannot remove learned gates or undo duel effects"),
        "/gmgoto" => detail!("travel", "Move caller to numeric same-space entity ID; saved caller position may change", "TOML TargetName is not resolved as a name; native string binding unproved", Being, ArgumentRequired, Some(Persistent), "Positive numeric TargetName and same-space entity", "server_entity_get|server_db_query", "caller.position|sgw_player.pos_x/y/z", "Manually return caller to captured live and saved position; duel effects remain"),
        "/gmsummon" => detail!("travel", "Move numeric same-space target to caller; player position may persist", "TOML TargetName is not resolved as a name; native string binding unproved", Being, ArgumentRequiredOther, Some(Persistent), "Positive numeric TargetName, not self, same-space entity", "server_entity_get|server_db_query", "target.position|target_player.sgw_player.pos_x/y/z", "Manually return target to captured live and saved position; duel effects remain"),
        "/gmgotolocation" => detail!("travel", "Reload into named world; arrival may persist world, position and gate book", "Native world-name quoting, binding and accepted travel receipt unproved", Player, NoSelection, Some(Persistent), "Nonempty world, finite coordinates; zero triple requests authored entry", "server_sessions|server_entity_get|server_db_query", "space_id|position|sgw_player.world_location|sgw_player.world_id|sgw_player.pos_x/y/z|sgw_player.known_stargates", "Manual return travel may restore location, not new gate IDs or duel state"),
        "/gmgotoxyz" => detail!("travel", "Snap caller within current space; saved position may change", "Native binding and accepted teleport receipt unproved", Player, NoSelection, Some(Persistent), "Caller entity and finite coordinates", "server_entity_get|server_db_query", "space_id|position|sgw_player.pos_x/y/z", "Manually return to captured live and saved position; duel effects remain"),
        "/gmreloadorganizations" => detail!("organizations", "Re-push caller Team and Command client state; partial delivery possible", "Native binding and received-state proof unproved; client organization state capture is pending", Player, NoSelection, Some(Transient), "Caller character, GM re-authorization and Base DB pool", "", "", "Repeat authoritative push or relog if needed; no DB rollback applies"),
        _ => unreachable!("slice names checked before detail lookup"),
    }
}
