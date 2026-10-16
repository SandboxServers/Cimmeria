//! Evidence-backed, disabled native slash forms for patch-012 words 82–108.

use serde::Serialize;
use serde_json::Value;

use super::commands::{
    ArgDescriptor, ArgKind, ArgRange, Availability, CommandChannel, CommandDescriptor,
    Compensation, Impact, ImpactKind, SelectionRule, TargetKind, TargetPolicy, VerifyRecipe,
    COMMAND_METADATA_VERSION,
};
use super::slash_inventory::{parse_patch_table, SlashEntry, SlashParameterType};

const PATCH: &str =
    include_str!("../../../../data/client-patches/012-gm-slash-commands/commands.toml");
const EVIDENCE: &str = "docs/analysis/dev-workbench/evidence/DW-26B-E.md";

/// Server handler status, independent of patch parsing and native binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HandlerStatus {
    Implemented,
    Absent,
}

/// A typed patch form with its separately sourced server and readback claims.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SlashDescriptor2b {
    pub source: SlashEntry,
    pub descriptor: CommandDescriptor,
    pub handler_status: HandlerStatus,
    pub target_scope: String,
    pub precondition_note: String,
    pub readback_note: String,
    pub evidence: String,
}

struct Row {
    word: &'static str,
    event: &'static str,
    signature: &'static str,
    category: &'static str,
    summary: &'static str,
    scope: &'static str,
    handler: HandlerStatus,
    impact: Option<ImpactKind>,
    precondition: &'static str,
    tools: &'static str,
    fields: &'static str,
    readback: &'static str,
    recovery: &'static str,
    blocker: &'static str,
}

macro_rules! row {
    ($word:literal, $event:literal, $signature:literal, $category:literal, $summary:literal,
     $scope:literal, $handler:ident, $impact:expr, $precondition:literal,
     $tools:literal, $fields:literal, $readback:literal, $recovery:literal, $blocker:literal) => {
        Row {
            word: $word,
            event: $event,
            signature: $signature,
            category: $category,
            summary: $summary,
            scope: $scope,
            handler: HandlerStatus::$handler,
            impact: $impact,
            precondition: $precondition,
            tools: $tools,
            fields: $fields,
            readback: $readback,
            recovery: $recovery,
            blocker: $blocker,
        }
    };
}

const ROWS: &[Row] = &[
    row!("/gmgiveminigamecontact", "Event_SlashCmd_GiveMinigameContact", "ContactId:Integer:true,TargetId:Integer:true", "minigame", "Proposed contact grant; no server handler exists", "Proposed other-player contact/target pair; target identity unresolved", Absent, None, "TOML Integer/Integer conflicts with method WSTRING/INT64; contact and target validation absent", "", "", "No accepted grant or target-state readback", "", "Handler absent, signature mismatch and native binding unproved"),
    row!("/gmremoveminigamecontact", "Event_SlashCmd_RemoveMinigameContact", "ContactId:Integer:true,TargetId:Integer:true", "minigame", "Proposed contact removal; no server handler exists", "Proposed other-player contact/target pair; target identity unresolved", Absent, None, "TOML Integer/Integer conflicts with method WSTRING/INT64; removal rules absent", "", "", "No accepted removal or target-state readback", "", "Handler absent, signature mismatch and native binding unproved"),
    row!("/gmgivestargateaddress", "Event_SlashCmd_GiveStargateAddress", "AddressId:Text:true,Hidden:Boolean:true", "stargates", "Proposed address grant; no server handler exists", "Unresolved other-player target and one address", Absent, None, "Method requires INT64 target absent from TOML; never default to caller", "", "", "Startup address catalog cannot establish a player's known addresses", "", "Handler and target argument absent; native binding unproved"),
    row!("/gmremovestargateaddress", "Event_SlashCmd_RemoveStargateAddress", "AddressId:Text:true", "stargates", "Proposed address removal; no server handler exists", "Unresolved other-player target and one address", Absent, None, "Method requires INT64 target absent from TOML", "", "", "No player-address readback or accepted removal", "", "Handler and target argument absent; native binding unproved"),
    row!("/gmloadconstants", "Event_SlashCmd_LoadConstants", "", "reload", "Constants reload is unavailable", "Potential server constants; actual scope undefined", Absent, None, "No reload entrypoint", "", "", "No receipt or definition-version readback", "", "Handler absent; native binding unproved"),
    row!("/gmloadability", "Event_SlashCmd_LoadAbility", "aAbilityId:Integer:true", "reload", "Per-ID ability reload is unavailable", "One proposed ability definition, not a player grant", Absent, None, "Startup full-load primitive is not a per-ID GM handler", "", "", "Catalog presence is baseline, not reload proof", "", "Handler absent; native binding unproved"),
    row!("/gmreloadscripts", "Event_SlashCmd_LoadNACSI", "nacsiId:Integer:true", "reload", "Per-ID NACSI reload is unavailable", "One proposed NACSI ID; broader script scope unproved", Absent, None, "No NACSI loader or storage contract", "", "", "Unrelated content-reload logs are not success", "", "Handler and storage absent; native binding unproved"),
    row!("/gmloadabilityset", "Event_SlashCmd_LoadAbilitySet", "aAbilitySetId:Integer:true", "reload", "Per-ID ability-set reload is unavailable", "One proposed definition, not a selected mob", Absent, None, "Startup full-load primitive is not a per-ID handler", "", "", "No receipt or definition-version readback", "", "Handler absent; native binding unproved"),
    row!("/gmloadbehavior", "Event_SlashCmd_LoadBehavior", "behaviorEventSetId:Integer:true", "reload", "Behavior reload is unavailable", "Proposed behavior event set; no selected NPC", Absent, None, "No loadable behavior GM path", "", "", "No handler feedback or state readback", "", "Handler absent; native binding unproved"),
    row!("/gmloadmob", "Event_SlashCmd_LoadMOB", "mobId:Integer:true", "reload", "Per-mob-definition reload is unavailable", "One proposed mob definition, not a live entity", Absent, None, "No per-definition handler", "", "", "Spawn listing cannot prove a reload", "", "Handler absent; native binding unproved"),
    row!("/gmloaddialogset", "Event_SlashCmd_ShowDialog", "interactionSetId:Integer:true", "reload", "Per-ID dialog-set reload is unavailable", "Proposed interaction set; no player or NPC target", Absent, None, "ShowDialog event does not prove loadDialogSet binding", "", "", "A visible dialog would not establish reload", "", "Event/method ambiguity and handler absence; native binding unproved"),
    row!("/gmloaditem", "Event_SlashCmd_LoadItem", "itemId:Integer:true", "reload", "Per-ID item reload is unavailable", "One proposed item definition, not inventory", Absent, None, "Startup full-load primitive is not a per-ID handler", "", "", "Item catalog presence is baseline data", "", "Handler absent; native binding unproved"),
    row!("/gmloadmission", "Event_SlashCmd_LoadMission", "missionId:Integer:true", "reload", "Per-ID mission reload is unavailable", "One proposed definition, not player progress", Absent, None, "Startup full-load primitive is not a per-ID handler", "", "", "Player mission rows do not prove definition reload", "", "Handler absent; native binding unproved"),
    row!("/gmsetmobvariable", "Event_SlashCmd_SetMobVariable", "variable:Integer:true,value:Integer:true", "npc", "Mob variable writer is unavailable", "No target mob parameter or proven selection", Absent, None, "No generic mob variable store or target resolver", "", "", "NPC snapshot cannot attribute a change to this word", "", "Handler/store absent; target and native binding unproved"),
    row!("/gmentererroraistate", "Event_SlashCmd_EnterErrorAIState", "", "npc", "GM error-AI transition is unavailable", "Unspecified NPC target; selection unproved", Absent, None, "AI Error enum does not provide a GM transition", "", "", "No accepted transition readback", "", "Handler/target absent; native binding unproved"),
    row!("/gmexiterroraistate", "Event_SlashCmd_ExitErrorAIState", "", "npc", "GM error-AI clearance is unavailable", "Unspecified NPC target; selection unproved", Absent, None, "No error-state clearance entrypoint", "", "", "Observed AI recovery needs independent cause", "", "Handler/target absent; native binding unproved"),
    row!("/gmperfstatsbychannel", "Event_SlashCmd_PerfStatsByChannel", "aOnOff:Boolean:true", "diagnostics", "Per-channel performance toggle is unavailable", "Diagnostic scope unspecified; no proven audience", Absent, None, "TOML Boolean to method INT8 encoding unproved", "", "", "No metrics counter or toggle readback", "", "Handler absent; native binding unproved"),
    row!("/gmshowinstanceflag", "Event_SlashCmd_ShowInstanceFlag", "aFlagNumber:Integer:true", "instance", "Instance-flag reader is unavailable", "Proposed current instance; no implemented flag state", Absent, None, "No flag store or reader", "", "", "No response or independent flag readback", "", "Handler/state absent; native binding unproved"),
    row!("/gmsetinstanceflag", "Event_SlashCmd_SetInstanceFlag", "aFlagNumber:Integer:true,aFlagValue:Integer:true", "instance", "Instance-flag writer is unavailable", "Proposed current instance; co-located effect unknown", Absent, None, "TOML Integer flag value versus method INT8; no flag store", "", "", "No receipt, prior value or independent flag readback", "", "Handler/state absent; argument mismatch and native binding unproved"),
    row!("/gmspawnentityloot", "Event_SlashCmd_ShowSpawns", "EntityId:Integer:true,LootTableID:Integer:true", "loot", "GM spawn-loot handler is unavailable", "Proposed entity and loot table; no implemented resolver", Absent, None, "ShowSpawns event does not prove spawnEntityLoot binding", "", "", "Internal loot drop does not prove this command ran", "", "Event/method ambiguity and handler absence; native binding unproved"),
    row!("/gmtestlos", "Event_SlashCmd_TestLOS", "sourceID:Integer:true,targetID:Integer:true", "diagnostics", "Read line of sight between two entities in caller's space", "Two explicit entities in the caller's exact space; caller-only feedback", Implemented, Some(ImpactKind::ReadOnly), "Both signed INT32 IDs must resolve in caller's space; UNKNOWN is not CLEAR", "", "", "Capture caller-only testLOS source to target feedback: CLEAR, BLOCKED, UNKNOWN, or target refusal; compare both IDs and space", "", "native event-to-method and submission proof pending"),
    row!("/gmtogglecombatlos", "Event_SlashCmd_ToggleCombatLOS", "", "combat", "Combat-LOS toggle is unavailable", "Combat scope unspecified; no proven audience", Absent, None, "Def-only bCombatLOS is not a handler", "", "", "Combat outcome cannot prove toggle", "", "Handler absent; native binding unproved"),
    row!("/gmtrackmob", "Event_SlashCmd_ShowMobPaths", "", "npc", "Mob path tracking is unavailable", "Proposed mob target unspecified", Absent, None, "No GM tracking subscription", "", "", "AI snapshot is separate, not tracking output", "", "Event/method ambiguity and handler absence; native binding unproved"),
    row!("/gmonxrayeyes", "Event_SlashCmd_XRayEyes", "on/off:Boolean:true", "client view", "X-ray-eyes client effect is unverified", "Calling GM's client view only", Absent, None, "No server handler or proven client-local presentation", "", "", "Visual client check needed; no server readback", "", "Handler/client effect and native submission unproved"),
    row!("/gmoninvisible", "Event_SlashCmd_Invisible", "on/off:Boolean:true", "visibility", "Invisibility transition is unavailable", "Presumed caller visibility; observer scope unknown", Absent, None, "AoI invisible packet primitive is not a toggle handler", "", "", "No observer-state proof or direct readback", "", "Handler/state absent; native binding unproved"),
    row!("/gmonphysics", "Event_SlashCmd_Physics", "on/off:Boolean:true", "movement", "Toggle only caller's movement-validation bypass if method is reached", "Calling GM server movement validator; no other-player audience", Implemented, Some(ImpactKind::Transient), "Method byte 0 enables bypass, nonzero restores validation; patch Boolean encoding unproved", "", "", "Caller feedback and actor-bound unrestricted log prove a change only; no direct flag read tool, and no-op gives neither", "Restore with verified method byte 1; prior invalid movement cannot be undone", "Patch event-to-method, Boolean encoding and native submission unproved"),
    row!("/gmsendgmshout", "Event_SlashCmd_GMShout", "isGlobal:Boolean:true,Text:Text:true", "announcement", "Announce to exact space or all online in-world players", "isGlobal=0: all player entities in GM's space; nonzero: all online in-world players", Implemented, Some(ImpactKind::Irreversible), "GM gate, live caller and nonblank text of at most 255 UTF-16 units without controls/format characters", "", "", "A submitted line is not acceptance; acceptance chat.gm_broadcast differs from aggregate chat.gm_broadcast_delivered or send_failed; no per-shout correlation when overlapping; check recipient UI. A delivered announcement cannot be unsent; a correction is a new broadcast", "", "native slash split/quoting and submission unproved; preview audience and redact Text"),
];

/// Load this exact source slice; all forms remain disabled until native proof.
pub fn slash_descriptors_2b() -> Result<Vec<SlashDescriptor2b>, String> {
    let entries = parse_patch_table(PATCH)?;
    descriptors_for(&entries)
}

/// Build the source-backed forms from the full patch inventory.
pub fn descriptors_for(entries: &[SlashEntry]) -> Result<Vec<SlashDescriptor2b>, String> {
    let slice = entries
        .get(81..108)
        .ok_or("patch slash slice 82–108 is too short")?;
    slice
        .iter()
        .zip(ROWS)
        .map(|(source, row)| {
            let signature = source
                .parameters
                .iter()
                .map(|p| {
                    let kind = match p.parameter_type {
                        SlashParameterType::Boolean => "Boolean",
                        SlashParameterType::Integer => "Integer",
                        SlashParameterType::Float => "Float",
                        SlashParameterType::String => "Text",
                    };
                    format!("{}:{kind}:{}", p.name, p.required)
                })
                .collect::<Vec<_>>()
                .join(",");
            if source.name != row.word
                || source.event != row.event
                || source.access != "p"
                || signature != row.signature
            {
                return Err(format!("slash evidence mismatch at {}", row.word));
            }
            let args = source
                .parameters
                .iter()
                .map(|p| ArgDescriptor {
                    name: p.name.clone(),
                    kind: match p.parameter_type {
                        SlashParameterType::Boolean => ArgKind::Boolean,
                        SlashParameterType::Integer => ArgKind::Integer,
                        SlashParameterType::Float => ArgKind::Float,
                        SlashParameterType::String => ArgKind::Text,
                    },
                    required: p.required,
                    default: None,
                    range: integer_range(row.word, &p.name),
                    choices: Vec::new(),
                    help: argument_help(row.word, &p.name).into(),
                    sensitive: row.word == "/gmsendgmshout" && p.name == "Text",
                })
                .collect();
            let target = target_policy(row.word);
            let descriptor = CommandDescriptor {
                id: source.name.clone(),
                channel: CommandChannel::Slash,
                category: row.category.into(),
                aliases: Vec::new(),
                summary: row.summary.into(),
                example: None,
                availability: Availability::Unsupported,
                disposition_reason: Some(format!(
                    "{}; see {EVIDENCE}#{}",
                    row.blocker,
                    &row.word[1..]
                )),
                manual_pending: true,
                args,
                target,
                impact: row.impact.map(|kind| Impact {
                    kind,
                    summary: row.summary.into(),
                    preconditions: vec![
                        "Live server-sourced GameMaster actor".into(),
                        row.precondition.into(),
                    ],
                }),
                verify: (!row.tools.is_empty()).then(|| VerifyRecipe {
                    read_tools: row.tools.split('|').map(str::to_owned).collect(),
                    comparison_fields: row.fields.split('|').map(str::to_owned).collect(),
                }),
                compensation: Some(if row.recovery.is_empty() {
                    Compensation::None
                } else {
                    Compensation::Manual(row.recovery.into())
                }),
                metadata_version: COMMAND_METADATA_VERSION,
            };
            Ok(SlashDescriptor2b {
                source: source.clone(),
                descriptor,
                handler_status: row.handler,
                target_scope: row.scope.into(),
                precondition_note: row.precondition.into(),
                readback_note: row.readback.into(),
                evidence: format!("{EVIDENCE}#{}", &row.word[1..]),
            })
        })
        .collect()
}

fn target_policy(word: &str) -> TargetPolicy {
    if word == "/gmtestlos" {
        TargetPolicy {
            kind: TargetKind::Being,
            selection_rule: SelectionRule::ArgumentRequired,
            argument_name: Some("targetID".into()),
        }
    } else {
        // Unimplemented target words do not have a trusted resolver. Never
        // silently turn an omitted target into the caller or current selection.
        TargetPolicy {
            kind: TargetKind::None,
            selection_rule: SelectionRule::None,
            argument_name: None,
        }
    }
}

fn integer_range(word: &str, name: &str) -> Option<ArgRange> {
    if word == "/gmtestlos" && matches!(name, "sourceID" | "targetID") {
        Some(ArgRange {
            min: Some(0.0),
            max: Some(i32::MAX as f64),
        })
    } else {
        None // No bounds in TOML; absent handlers have no safe server range.
    }
}

fn argument_help(word: &str, name: &str) -> &'static str {
    match (word, name) {
        ("/gmgiveminigamecontact" | "/gmremoveminigamecontact", "TargetId") =>
            "Proposed other-player ID; TOML Integer conflicts with method INT64 and no resolver exists",
        ("/gmgiveminigamecontact" | "/gmremoveminigamecontact", "ContactId") =>
            "TOML Integer conflicts with method WSTRING contact ID",
        ("/gmgivestargateaddress" | "/gmremovestargateaddress", _) =>
            "Patch omits required other-player target; no caller or selection default",
        ("/gmsetinstanceflag", "aFlagValue") =>
            "TOML Integer, method INT8; value range and semantics unproved",
        ("/gmtestlos", _) => "Explicit nonnegative signed INT32 entity ID in caller's space",
        ("/gmonphysics", _) =>
            "TOML Boolean text encoding unproved; method byte 0 enables bypass and nonzero restores",
        ("/gmsendgmshout", "isGlobal") =>
            "Scope preview: false/zero exact space; true/nonzero all online in-world; native Boolean encoding unproved",
        ("/gmsendgmshout", "Text") =>
            "Public announcement; redact in companion history, audit, trace and screenshots by default",
        _ => "Required patch-012 typed parameter; no TOML default or bound",
    }
}

/// Format a preview or stored value without leaking sensitive announcement text.
/// Callers must use this for history, audit, trace and screenshot surfaces.
pub fn display_argument(arg: &ArgDescriptor, value: &Value, reveal: bool) -> String {
    if arg.sensitive && !reveal {
        "[redacted]".into()
    } else {
        value.to_string()
    }
}

/// Scope preview for a GM shout. Boolean is only a typed preview, not native encoding.
pub fn shout_audience_preview(global: bool) -> &'static str {
    if global {
        "All online in-world players across instances"
    } else {
        "All player entities in the GM's exact space instance"
    }
}
