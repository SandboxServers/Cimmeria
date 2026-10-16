//! Evidence-backed, disabled native slash forms for patch-012 words 136–162.

use serde::Serialize;

use super::commands::{
    ArgDescriptor, ArgKind, Availability, CommandChannel, CommandDescriptor, Compensation, Impact,
    ImpactKind, SelectionRule, TargetKind, TargetPolicy, COMMAND_METADATA_VERSION,
};
use super::slash_inventory::{SlashEntry, SlashParameterType};

/// Exact final patch-table words, in source order.
pub const NAMES: [&str; 27] = [
    "/gmsetarchetype",
    "/gmtimeofday",
    "/gmtoggleautocycleability",
    "/gmshowcover",
    "/gmshowcommandwaypoints",
    "/gmshowspawnset",
    "/gmshowarea",
    "/gmshowregion",
    "/gmchat",
    "/gmloadinteractionset",
    "/gmpetinvokeability",
    "/gmpetinvokecommand",
    "/gmpetabilitytoggle",
    "/gmchangeammo",
    "/gmforceclientcrash",
    "/gmforcerendercrash",
    "/gmsetringdestination",
    "/gmdumpobjects",
    "/gmspacequeuestatus",
    "/gmspacequeuedresponse",
    "/gmspacequeuereadyresponse",
    "/gmconfirmeffect",
    "/gmshowtriggers",
    "/gmshowwaypoints",
    "/gmshownavmesh",
    "/gmshowmemory",
    "/gmdebugtarget",
];

/// Server or client handler evidence, independent of native slash binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HandlerStatus {
    ImplementedOrdinaryMethod,
    ReceiptOnly,
    Absent,
    ClientEffectUnverified,
}

/// Final-slice form with separately sourced handler and readback limits.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SlashDescriptor3b {
    pub source: SlashEntry,
    pub descriptor: CommandDescriptor,
    pub handler_status: HandlerStatus,
    pub target_scope: String,
    pub readback_note: String,
    pub evidence: String,
}

struct Row {
    event: &'static str,
    signature: &'static str,
    category: &'static str,
    summary: &'static str,
    scope: &'static str,
    handler: HandlerStatus,
    impact: Option<ImpactKind>,
    precondition: &'static str,
    readback: &'static str,
    blocked: &'static str,
}

macro_rules! row {
    ($event:literal, $signature:literal, $category:literal, $summary:literal,
     $scope:literal, $handler:ident, $impact:expr, $precondition:literal,
     $readback:literal, $blocked:literal) => {
        Row {
            event: $event,
            signature: $signature,
            category: $category,
            summary: $summary,
            scope: $scope,
            handler: HandlerStatus::$handler,
            impact: $impact,
            precondition: $precondition,
            readback: $readback,
            blocked: $blocked,
        }
    };
}

const ROWS: [Row; 27] = [
    row!("Event_SlashCmd_SetArchetype", "", "progression", "Archetype setter has no typed value or handler", "Unknown player; no argument identifies archetype or target", Absent, Some(ImpactKind::Persistent), "A future handler needs GM authority, exact target and coordinated archetype/ability/stat mutation", "No effect now; future proof needs the exact sgw_player.archetype row and reloaded cell state", "TOML has no archetype argument, server handler or native binding"),
    row!("Event_SlashCmd_TimeofDay", "Time:Float:true,Wind:Float:true,Weather:Integer:true", "client view", "Possible caller-local time and weather presentation", "Calling client's display only in legacy console; patch event scope unproved", ClientEffectUnverified, Some(ImpactKind::Transient), "No world-clock mutation or GM-tail handler is proven", "If bound, observe recipient onTimeofDay and rendered state; no durable world readback", "Patch event binding and client presentation effect unproved"),
    row!("Event_SlashCmd_toggleAutoCycleAbility", "", "combat", "Potential caller auto-cycle toggle", "Authenticated caller and their selected target if ordinary method 83 is reached", ImplementedOrdinaryMethod, Some(ImpactKind::Transient), "Method 83 can fire an ability immediately; prefixed word registration and submission unproved", "Server auto_cycle fields are not exposed by current lab snapshots; button light alone is insufficient", "Prefixed native submission unproved despite stock event-to-method trace"),
    row!("Event_SlashCmd_ShowCover", "on/off:Boolean:true", "client view", "Possible local cover-node overlay", "Local client view, no cover graph edit", ClientEffectUnverified, Some(ImpactKind::Transient), "Client handler and Boolean interpretation unproved", "Direct client overlay observation required; no server readback", "Client event handler and native submission unproved"),
    row!("Event_SlashCmd_ShowCommandWaypoints", "on/off:Boolean:true", "client view", "Possible command waypoint overlay", "Local client view, no NPC path authoring", ClientEffectUnverified, Some(ImpactKind::Transient), "Inbound callback does not prove an outbound slash request", "Observe actual local overlay after proven event", "Outbound slash-to-callback binding unproved"),
    row!("Event_SlashCmd_ShowSpawnSet", "", "spawns", "Unknown spawn-set display", "Unknown local selection or visualization; no set ID", Absent, None, "Do not substitute mutating activate/deactivate methods 214/215", "No accepted effect or selected set readback", "No set ID, matching handler or native binding"),
    row!("Event_SlashCmd_ShowArea", "", "client view", "Possible local area overlay", "Local client view only if handler exists", ClientEffectUnverified, Some(ImpactKind::Transient), "No proven client handler or toggle polarity", "Direct visual check required; no server state readback", "Area overlay effect and submission unproved"),
    row!("Event_SlashCmd_ShowRegion", "Type:Text:true", "client view", "Possible region-type overlay filter", "Local client view; accepted Type vocabulary unknown", ClientEffectUnverified, Some(ImpactKind::Transient), "String choices and quoting unproved", "Visual type filter comparison required; no server mutation", "Type semantics, client event behavior and binding unproved"),
    row!("Event_SlashCmd_GMChat", "Text:Text:true", "social", "Unknown GM chat delivery", "Recipient audience unknown; not interchangeable with GM shout", Absent, Some(ImpactKind::Irreversible), "No handler, audience, safe quoting or redacted audit path", "No known recipient or delivery receipt; local echo is insufficient", "GMChat handler, audience and safe text transport unproved"),
    row!("Event_SlashCmd_LoadInteractionSet", "", "reload", "Unknown interaction-set load", "Unknown resource or cache; no ID or reload scope", Absent, None, "No Cimmeria slash reload handler", "No cache generation or handler receipt readback", "Interaction-set handler and scope absent"),
    row!("Event_SlashCmd_PetInvokeAbility", "entityId:Integer:true,abilityId:Integer:true", "pets", "Pet ability command with missing method target", "Caller-owned pet only if ordinary method 88 is reached", ImplementedOrdinaryMethod, Some(ImpactKind::Transient), "Method 88 needs targetId missing from TOML; pet ownership and target validity are server checks", "No slash receipt; separately proven method needs owner feedback, cast and target state", "Missing targetId and native event binding block method 88 mapping"),
    row!("Event_SlashCmd_PetInvokeCommand", "entityId:Integer:true,commandId:Integer:true", "pets", "Unknown pet command", "Unbound entityId and unspecified command ID", Absent, None, "No server method or command ID vocabulary", "No handler response or pet AI state readback", "Pet command handler and command-ID contract absent"),
    row!("Event_SlashCmd_PetAbilityToggle", "entityId:Integer:true,abilityId:Integer:true,toggle:Integer:true", "pets", "Potential caller-owned pet ability toggle", "Caller-owned pet and one ability if ordinary method 89 is reached", ImplementedOrdinaryMethod, Some(ImpactKind::Transient), "Method 89 accepts INT8 toggle 0/1; TOML Integer encoding and binding unproved", "onPetAbilityList only proves resend; it omits toggled_off state", "Integer-to-INT8 serialization and native event binding unproved"),
    row!("Event_SlashCmd_ChangeAmmo", "ammoType:Integer:true", "inventory", "Unknown slash ammo change", "No weapon instance in TOML; ordinary inventory method requires one", Absent, Some(ImpactKind::Persistent), "Do not invent selected weapon; ordinary path separately validates compatibility and persists exact row", "No slash effect; ordinary path requires live slot plus exact sgw_inventory.cur_ammo_type after Base write", "Missing weapon binding and no GM slash handler"),
    row!("Event_SlashCmd_ForceClientCrash", "", "client debug", "Potentially terminate the local client process", "Local process only; not a remote player crash", ClientEffectUnverified, Some(ImpactKind::Irreversible), "Destructive client event is intentionally unavailable", "Process exit would be observable but is not exercised", "Destructive local action and unproved client handler"),
    row!("Event_SlashCmd_ForceRenderThreadCrash", "", "client debug", "Potentially crash the local render thread", "Local client render thread only", ClientEffectUnverified, Some(ImpactKind::Irreversible), "Destructive render event is intentionally unavailable", "Crash or hang is not exercised", "Destructive local action and unproved render handler"),
    row!("Event_SlashCmd_SetRingTransporterDestination", "regionId:Integer:true,destinationId:Integer:true", "travel", "Potential ring destination selection and world transfer", "Caller and explicit source/destination ring IDs if ordinary method 91 is reached", ImplementedOrdinaryMethod, Some(ImpactKind::Transient), "Method 91 lacks a server-held caller/source interaction guard; native binding unproved", "Require exact selection receipt, arrival world/position and transfer packet", "Native binding and server-held source interaction guard missing"),
    row!("Event_SlashCmd_DumpObjects", "", "diagnostics", "Unknown object dump", "Local or server object scope and output destination unknown", Absent, None, "Output could disclose names or addresses; redaction policy absent", "No defined complete inventory or redacted readback", "Object scope, handler and output redaction unresolved"),
    row!("Event_SlashCmd_SpaceQueueStatus", "", "queue", "Unknown space queue query", "Caller queue only if base method 26 is the binding", ReceiptOnly, None, "Base method trace stub is not a proven queue query", "No reliable queue snapshot or caller-bound feedback", "Event-to-base binding and effective handler missing"),
    row!("Event_SlashCmd_SpaceQueuedResponse", "", "queue", "Potential caller queue response", "Caller queue prompt only if base method 28 is the binding", ReceiptOnly, Some(ImpactKind::Transient), "Base method needs INT8 accept absent from TOML; never synthesize a response", "No queue state or transfer outcome readback", "Missing accept parameter and unproved base handler"),
    row!("Event_SlashCmd_SpaceQueueReadyResponse", "", "queue", "Potential caller queue-ready response", "Caller ready prompt only if base method 27 is the binding", ReceiptOnly, Some(ImpactKind::Transient), "Base method needs INT8 accept absent from TOML", "No accepted queue state or transfer readback", "Missing accept parameter and unproved base routing"),
    row!("Event_SlashCmd_ConfirmEffect", "EffectId:Integer:true,Response:Boolean:true", "abilities", "Potential caller effect confirmation receipt", "Caller and one pending effect if ordinary ability-manager method 4 is reached", ReceiptOnly, None, "Stock event-to-method path is traced; prefixed patch registration unproved; server logs only", "Exact caller/effect/accepted-byte log proves receipt, not committed effect state", "Prefixed native submission unproved and server handler is receipt-only"),
    row!("Event_SlashCmd_ShowTriggers", "", "client view", "Possible local trigger overlay", "Local visualization, not content trigger authoring", ClientEffectUnverified, Some(ImpactKind::Transient), "Client event effect and submission unproved", "Direct client overlay observation required", "Trigger event effect and native submission unproved"),
    row!("Event_SlashCmd_ShowWaypoints", "", "client view", "Possible local waypoint display", "Local view; no point set or radius in TOML", ClientEffectUnverified, Some(ImpactKind::Transient), "Inbound onShowWaypoints is not proof of an outbound request", "Match visible points to proven inbound payload", "Waypoints request binding and response source unknown"),
    row!("Event_SlashCmd_ShowNavMesh", "", "client view", "Possible local navmesh display", "Current client chunk if implemented; no navmesh edit", Absent, Some(ImpactKind::Transient), "Candidate method 211 needs INT8 onOff absent from TOML and has no handler", "No current polygon readback or rendered-chunk proof", "Navmesh request handler and Boolean mapping absent"),
    row!("Event_SlashCmd_ShowMemory", "", "client debug", "Possible local memory diagnostics", "Client-local metric source unverified; no server heap request", ClientEffectUnverified, Some(ImpactKind::Transient), "Never export raw addresses or heap content by default", "No metric source, timestamp or redacted display proof", "Memory metric source, client binding and output sensitivity unresolved"),
    row!("Event_SlashCmd_DebugTarget", "", "diagnostics", "Unknown target inspection", "Selected target only if client handler proves it; no target argument", Absent, None, "Do not substitute gmShowPlayer or gmDebugMobData; fresh target and GM gate needed", "No current reply; future proof needs response entity ID matched to fresh server snapshot", "No handler, target argument or native event binding"),
];

/// Build the exact final 27 forms; no descriptor is runnable without native proof.
pub fn descriptors_3b(entries: &[SlashEntry]) -> Result<Vec<SlashDescriptor3b>, String> {
    if entries.len() != NAMES.len() || entries.iter().map(|entry| entry.name.as_str()).ne(NAMES) {
        return Err("slash slice 136–162 differs from canonical patch-table order".into());
    }
    entries
        .iter()
        .zip(ROWS.iter())
        .map(|(entry, row)| {
            let signature = entry
                .parameters
                .iter()
                .map(|parameter| {
                    let kind = match parameter.parameter_type {
                        SlashParameterType::Boolean => "Boolean",
                        SlashParameterType::Integer => "Integer",
                        SlashParameterType::Float => "Float",
                        SlashParameterType::String => "Text",
                    };
                    format!("{}:{kind}:{}", parameter.name, parameter.required)
                })
                .collect::<Vec<_>>()
                .join(",");
            if entry.access != "p" || entry.event != row.event || signature != row.signature {
                return Err(format!(
                    "{} event or parameter contract differs from patch 012",
                    entry.name
                ));
            }
            let args = entry
                .parameters
                .iter()
                .map(|parameter| ArgDescriptor {
                    name: parameter.name.clone(),
                    kind: match parameter.parameter_type {
                        SlashParameterType::Boolean => ArgKind::Boolean,
                        SlashParameterType::Integer => ArgKind::Integer,
                        SlashParameterType::Float => ArgKind::Float,
                        SlashParameterType::String => ArgKind::Text,
                    },
                    required: parameter.required,
                    default: None,
                    range: None,
                    choices: Vec::new(),
                    help: argument_help(&entry.name, &parameter.name).into(),
                    sensitive: entry.name == "/gmchat" && parameter.name == "Text",
                })
                .collect();
            let target_kind = match entry.name.as_str() {
                "/gmtoggleautocycleability" | "/gmsetringdestination" | "/gmconfirmeffect" => {
                    TargetKind::Player
                }
                _ => TargetKind::None,
            };
            let descriptor = CommandDescriptor {
                id: entry.name.clone(),
                channel: CommandChannel::Slash,
                category: row.category.into(),
                aliases: Vec::new(),
                summary: row.summary.into(),
                example: None,
                availability: Availability::Unsupported,
                disposition_reason: Some(row.blocked.into()),
                manual_pending: true,
                args,
                target: TargetPolicy {
                    kind: target_kind,
                    selection_rule: SelectionRule::None,
                    argument_name: None,
                },
                impact: row.impact.map(|kind| Impact {
                    kind,
                    summary: row.summary.into(),
                    preconditions: vec![row.precondition.into()],
                }),
                verify: None,
                compensation: Some(Compensation::None),
                metadata_version: COMMAND_METADATA_VERSION,
            };
            Ok(SlashDescriptor3b {
                source: entry.clone(),
                descriptor,
                handler_status: row.handler,
                target_scope: row.scope.into(),
                readback_note: row.readback.into(),
                evidence: "docs/analysis/dev-workbench/evidence/DW-27B-E.md".into(),
            })
        })
        .collect()
}

fn argument_help(command: &str, name: &str) -> &'static str {
    match (command, name) {
        ("/gmchat", "Text") => "Sensitive text; audience, quoting and redacted audit path unproved",
        ("/gmpetinvokeability", _) => {
            "Ordinary method 88 also needs targetId, absent from patch TOML"
        }
        ("/gmpetabilitytoggle", "toggle") => {
            "Patch Integer; ordinary method requires INT8 0/1; encoding unproved"
        }
        ("/gmsetringdestination", "regionId") => {
            "Source ring ID; server-held caller interaction guard missing"
        }
        ("/gmshowregion", "Type") => "Accepted region type vocabulary and quoting unproved",
        ("/gmconfirmeffect", "Response") => {
            "Stock handler maps Boolean to accepted byte; prefixed registration unproved"
        }
        _ => "Patch 012 typed parameter; no unlisted default, range or target binding",
    }
}
