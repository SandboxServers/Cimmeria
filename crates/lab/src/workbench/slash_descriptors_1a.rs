//! Evidence-backed, disabled palette forms for patch-012 slash words 1–27.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::commands::{
    ArgDescriptor, ArgKind, Availability, CommandChannel, CommandDescriptor, Compensation, Impact,
    ImpactKind, SelectionRule, TargetKind, TargetPolicy, VerifyRecipe, COMMAND_METADATA_VERSION,
};
use super::slash_inventory::{parse_patch_table, SlashEntry, SlashParameterType};

const PATCH_TOML: &str =
    include_str!("../../../../data/client-patches/012-gm-slash-commands/commands.toml");
const EVIDENCE: &str = "docs/analysis/dev-workbench/evidence/DW-25A-E.md";
const ARGUMENTS: &[&str] = &[
    "String MissionDesignId;Boolean DisplayAcceptWindow",
    "String DesignID",
    "",
    "",
    "",
    "",
    "String MissionDesignId",
    "String MissionDesignId;Integer StepToAdvanceTo",
    "String MissionDesignId;Integer StepToAdvanceTo",
    "String MissionDesignId;Boolean TurnIn",
    "String MissionDesignId",
    "String MissionDesignId",
    "",
    "",
    "?Integer flagId",
    "",
    "String attribute",
    "Integer SpaceID",
    "",
    "",
    "",
    "Integer XpAmount",
    "String DesignId;Integer Quantity",
    "Integer Amount",
    "String ItemName;Integer Quantity",
    "Integer AbilityId",
    "Integer numPoints",
];
const EVENTS: &[&str] = &[
    "Event_SlashCmd_MissionAssign",
    "Event_SlashCmd_MissionClear",
    "Event_SlashCmd_MissionClearActive",
    "Event_SlashCmd_MissionClearHistory",
    "Event_SlashCmd_MissionList",
    "Event_SlashCmd_MissionListFull",
    "Event_SlashCmd_MissionDetails",
    "Event_SlashCmd_MissionAdvance",
    "Event_SlashCmd_MissionReset",
    "Event_SlashCmd_MissionComplete",
    "Event_SlashCmd_MissionSetAvailable",
    "Event_SlashCmd_MissionAbandon",
    "Event_SlashCmd_ShowTargetLocation",
    "Event_SlashCmd_ListAbilities",
    "Event_SlashCmd_ShowFlag",
    "Event_SlashCmd_ListInteractions",
    "Event_SlashCmd_GetMobAttribute",
    "Event_SlashCmd_ShowMobCount",
    "Event_SlashCmd_ShowIP",
    "Event_SlashCmd_ShowInventory",
    "Event_SlashCmd_ShowPlayer",
    "Event_SlashCmd_GiveXp",
    "Event_SlashCmd_GiveItem",
    "Event_SlashCmd_GiveNaqahdah",
    "Event_SlashCmd_GMDeleteItem",
    "Event_SlashCmd_GiveAbility",
    "Event_SlashCmd_GiveTrainingPoints",
];

/// Server status is distinct from the unproved client event binding and transport.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandlerAvailability {
    Implemented,
    Absent,
    SignatureMismatch,
    OptionalArgumentMismatch,
}

/// One first-slice form, including evidence and diagnostic readback limits.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SlashDescriptor1a {
    pub source: SlashEntry,
    pub descriptor: CommandDescriptor,
    pub handler_availability: HandlerAvailability,
    pub target_scope: String,
    pub readback_note: String,
    pub evidence: String,
}

struct Row {
    word: &'static str,
    category: &'static str,
    help: &'static str,
    target: &'static str,
    handler: HandlerAvailability,
    impact: Option<ImpactKind>,
    effect: &'static str,
    read_tool: &'static str,
    read_fields: &'static str,
    readback: &'static str,
    compensation: &'static str,
    blocker: &'static str,
    arg_help: &'static [&'static str],
}

macro_rules! impact {
    (Unknown) => {
        None
    };
    ($known:ident) => {
        Some(ImpactKind::$known)
    };
}

macro_rules! row {
    ($word:literal, $category:literal, $help:literal, $target:literal, $handler:ident,
     $impact:ident, $effect:literal, $tool:literal, $fields:literal, $readback:literal,
     $compensation:literal, $blocker:literal, [$($arg:literal),* $(,)?]) => {
        Row {
            word: $word, category: $category, help: $help, target: $target,
            handler: HandlerAvailability::$handler, impact: impact!($impact),
            effect: $effect, read_tool: $tool, read_fields: $fields,
            readback: $readback, compensation: $compensation, blocker: $blocker,
            arg_help: &[$($arg),*],
        }
    };
}

const ROWS: &[Row] = &[
    row!("/gmmissionassign", "missions", "Accept a mission on your GM character; the popup flag is only a client hint.", "caller player", Implemented,
        Persistent, "Queues mission accept, update, and first-step content events.", "server_db_query", "sgw_mission.player_id;sgw_mission.mission_id;sgw_mission.status;sgw_mission.current_step_id;sgw_mission.active_objective_ids;sgw_mission.completed_objective_ids", "Query the row keyed by authenticated caller player_id and requested mission_id before/after base processing; feedback and UI are separate.", "Manual reassign/clear only after inspecting current state; fired chains remain.", "Event-to-method, Boolean encoding, and native submission unproved.",
        ["Positive numeric mission design ID, entered as TOML String.", "Show accept window; TOML Boolean, server popup byte encoding unproved."]),
    row!("/gmmissionclear", "missions", "Abandon one active mission on your GM character.", "caller player", Implemented,
        Persistent, "Abandons mission and fires mission_abandoned; non-repeat row may be deleted.", "server_db_query", "sgw_mission.player_id;sgw_mission.mission_id;sgw_mission.status;sgw_mission.repeats;row presence", "Query the row keyed by authenticated caller player_id and requested mission_id before/after base processing; deletion can be expected.", "Manual reassign if offerable; progress and fired content are not restored.", "Native event binding and submission unproved.", ["Positive numeric active mission ID, entered as TOML String."]),
    row!("/gmmissionclearactive", "missions", "Clear active missions, according to the patch event name; no server handler exists.", "caller player, inferred event scope only", Absent,
        Unknown, "No Cimmeria mutation through method 111; original effect unknown.", "", "", "Active mission rows establish baseline only, never command success.", "", "Server handler absent; native binding unproved. Never expand into repeated clears.", []),
    row!("/gmmissionclearhistory", "missions", "Clear mission history, according to the patch event name; no server primitive exists.", "unimplemented history scope", Absent,
        Unknown, "No Cimmeria mutation through method 112; original effect unknown.", "", "", "History rows are diagnostic only; no accepted-effect recipe.", "", "Server handler absent; original semantics and native binding unproved.", []),
    row!("/gmmissionlist", "mission inspection", "List your active missions from the live cell tracker.", "caller player", Implemented,
        ReadOnly, "Reports active mission IDs and states; no database write.", "mission X-ray", "mission IDs;status", "Compare GM feedback to the same caller's live mission X-ray.", "", "Native event binding and submission unproved.", []),
    row!("/gmmissionlistfull", "mission inspection", "List your loaded missions, including completed and hidden entries.", "caller player", Implemented,
        ReadOnly, "Reports the caller's loaded tracker, not the full resource catalog.", "mission X-ray", "mission IDs;status", "Compare feedback to live X-ray or caller DB rows, allowing freshness differences.", "", "Native event binding and submission unproved.", []),
    row!("/gmmissiondetails", "mission inspection", "Inspect one mission on your GM character; feedback omits repeat count.", "caller player", Implemented,
        ReadOnly, "Reports mission status, current step, and objective counts.", "mission X-ray", "mission ID;status;current step;objective counts", "Compare feedback to the caller's mission X-ray; read repeats separately from DB.", "", "Native event binding and submission unproved.", ["Positive numeric mission ID, entered as TOML String."]),
    row!("/gmmissionadvance", "missions", "Advance a held mission to a step belonging to that mission.", "caller player", Implemented,
        Persistent, "Queues step/objective transition and step-region content replay.", "server_db_query", "sgw_mission.player_id;sgw_mission.mission_id;sgw_mission.current_step_id;sgw_mission.active_objective_ids;sgw_mission.completed_objective_ids;sgw_mission.failed_objective_ids", "Query the row keyed by authenticated caller player_id and requested mission_id before/after base processing; feedback can precede a failed transition.", "No automatic reset; another advance does not undo fired chains.", "Native event binding and submission unproved.",
        ["Positive numeric held mission ID, entered as TOML String.", "Positive step ID owned by the mission."]),
    row!("/gmmissionreset", "missions", "Reset a mission to a step, according to the patch event name; no handler exists.", "no implemented target", Absent,
        Unknown, "No reset mutation through method 117; original effect unknown.", "", "", "Baseline mission state does not establish reset success.", "", "Handler absent and reset semantics unproved; never substitute advance.",
        ["Mission design ID as TOML String; reset interpretation unproved.", "Step ID as TOML Integer; reset interpretation unproved."]),
    row!("/gmmissioncomplete", "missions", "Complete or turn in a mission, according to the patch event name; no handler exists.", "no implemented target", Absent,
        Unknown, "No completion or reward mutation through method 118; original effect unknown.", "", "", "Mission state and reward balances would need independent future reads.", "", "Handler absent; TurnIn reward semantics and native binding unproved.",
        ["Mission design ID as TOML String.", "Turn-in request as TOML Boolean; no reward guarantee."]),
    row!("/gmmissionsetavailable", "missions", "Set mission availability, according to the patch event name; no handler exists.", "no implemented target", Absent,
        Unknown, "No availability writer through method 119; original effect unknown.", "", "", "Offer visibility alone cannot prove this command acted.", "", "Handler absent; availability is not proven entity state.", ["Mission design ID as TOML String."]),
    row!("/gmmissionabandon", "missions", "Abandon one active mission; server alias of mission clear.", "caller player", Implemented,
        Persistent, "Abandons mission and fires mission_abandoned; repeat history can remain.", "server_db_query", "sgw_mission.player_id;sgw_mission.mission_id;sgw_mission.status;sgw_mission.repeats;row presence", "Query the row keyed by authenticated caller player_id and requested mission_id before/after base processing; compare client removal separately.", "Manual reassign if offerable; progress and content effects are not undone.", "Native alias event binding and submission unproved.", ["Positive numeric active mission ID, entered as TOML String."]),
    row!("/gmshowtargetlocation", "entity inspection", "Show the in-view target's location, or your own if no valid target exists.", "in-view same-space target or caller", Implemented,
        ReadOnly, "Reports live cell position.", "server_entity_get", "entity ID;instance;position", "Compare subject ID and coordinates close in time; moving subjects may differ.", "", "Native event binding and submission unproved.", []),
    row!("/gmlistabilities", "entity inspection", "List known ability IDs of the in-view target, or your own.", "in-view same-space target or caller", Implemented,
        ReadOnly, "Reports live known ability IDs; method name is listAbilities.", "server_db_query", "sgw_player.abilities", "For players compare feedback to persisted known IDs; NPC independent readback unavailable.", "", "Event-to-listAbilities binding and native submission unproved.", []),
    row!("/gmshowflag", "entity inspection", "Show one state-field bit of the in-view target, or your own.", "in-view same-space target or caller", OptionalArgumentMismatch,
        ReadOnly, "Reports bit and raw state field when a valid bit index is provided.", "", "", "Fresh same-entity state-field snapshot if exposed; otherwise independent readback unavailable.", "", "TOML permits omitted flagId but handler requires INT32 0–31; native binding unproved.", ["Optional TOML bit index 0–31; omitted form cannot satisfy server handler."]),
    row!("/gmlistinteractions", "entity inspection", "List interactions, according to the patch event name; no handler exists.", "unimplemented interaction scope", Absent,
        Unknown, "No list handler through method 126; original effect unknown.", "", "", "No accepted-effect readback; another interaction API is separate.", "", "Server handler absent and native binding unproved.", []),
    row!("/gmgetmobattribute", "entity inspection", "Read a mob attribute; the patch lacks the server's target-ID argument.", "explicit same-space target ID in server method only", SignatureMismatch,
        ReadOnly, "A separately proven method could report an attribute; patch form cannot encode it safely.", "", "", "No safe patch-form readback; a proven direct method needs same-target entity comparison.", "", "TOML omits leading INT32 target ID required by server; native binding unproved.", ["Attribute name as TOML String; server also requires an explicit target ID."]),
    row!("/gmshowmobcount", "world inspection", "Count live NPC entities in a loaded space, not database spawns.", "requested loaded space", Implemented,
        ReadOnly, "Reports live NPC count in SpaceID, with 0 meaning caller space.", "server_entity_query", "space_id;class_id=0x04;is_player=false;capped=false;returned;total_matched", "Resolve SpaceID=0 to the caller's actual space; query that exact space with class_id=0x04 and no radius, template or tag filter. Verify only if uncapped, every row is non-player, and returned equals total_matched.", "", "Native event binding and submission unproved.", ["Loaded space ID; 0 requests the caller's space."]),
    row!("/gmshowip", "sensitive inspection", "Show an address, according to the patch event name; no server handler exists.", "unknown", Absent,
        Unknown, "No address read through method 129; original effect unknown.", "", "", "No accepted-effect recipe; never export an IP from this form.", "", "Handler absent; TOML lacks method target ID; native binding unproved.", []),
    row!("/gmshowinventory", "inventory inspection", "Show inventory, according to the patch event name; no server handler exists.", "unknown", Absent,
        Unknown, "No inventory read through method 130; original effect unknown.", "", "", "An authorized inventory DB query is a separate operation, not command success.", "", "Handler absent; TOML lacks target ID; base round-trip and native binding unproved.", []),
    row!("/gmshowplayer", "entity inspection", "Show one player's summary; the patch omits the server method's target-ID argument.", "server explicit same-space ID or in-view target/caller fallback", SignatureMismatch,
        ReadOnly, "Server fallback may report an entity summary; patch encoding is unknown.", "", "", "No safe patch-form readback; a proven call needs same-entity/instance comparison.", "", "TOML omits INT32 target ID; missing-byte fallback does not prove client encoding.", []),
    row!("/gmgivexp", "progression grants", "Grant positive XP to your GM character; level and ASP can change too.", "caller player", Implemented,
        Persistent, "Base persists XP and may update level, training points, and ASP.", "server_db_query", "sgw_player.exp;level;training_points;applied_science_points", "Compare exact caller row after base completion and definitive base feedback.", "Manual correction needs separate approval; levels and fired effects may remain.", "Native event binding and submission unproved.", ["Positive XP amount for your GM character."]),
    row!("/gmgiveitem", "inventory grants", "Grant a numeric item design and quantity to your GM character.", "caller player inventory", Implemented,
        Persistent, "Base grants inventory item; placement may be outside main bag.", "server_db_query", "sgw_inventory.character_id;sgw_inventory.item_id;sgw_inventory.type_id;sgw_inventory.stack_size;sgw_inventory.container_id", "Compare rows for the authenticated caller character_id and requested design type_id before/after base write; item_id is the resulting instance ID, and container_id may differ from the requested bag. Require definitive base feedback.", "Manual removal requires fresh instance ID and approval; no safe automatic inverse.", "Native event binding and submission unproved.",
        ["Positive numeric item design ID, entered as TOML String; names are refused.", "Quantity at least 1; server clamps to 1000."]),
    row!("/gmgivecash", "currency grants", "Add naquadah to your GM character; the client event says GiveNaqahdah.", "caller player", Implemented,
        Persistent, "Base adds persistent sgw_player.naquadah after guarded lookup.", "server_db_query", "sgw_player.naquadah", "Compare exact caller balance after base commit and definitive feedback.", "Manual debit is a separate approved operation; no inverse slash command.", "Semantic event alias and native binding/submission unproved.", ["Positive amount of naquadah."]),
    row!("/gmremoveitem", "inventory mutation", "Remove an inventory item; the patch ItemName conflicts with server instance ID.", "caller player inventory in server method only", SignatureMismatch,
        Persistent, "A separately proven method could remove a persistent instance; patch form is unsafe.", "", "", "No safe patch-form readback; proven method needs exact instance/count/container DB comparison.", "No generic inverse; a grant creates a different instance and can lose state.", "TOML String ItemName vs server INT32 instance ID and INT16 quantity.",
        ["TOML item name; must not silently coerce to inventory instance ID.", "TOML Integer quantity; server wire width is INT16."]),
    row!("/gmgiveability", "ability grants", "Grant a known ability ID to your GM character without a training-point debit.", "caller player", Implemented,
        Persistent, "Base appends persisted ability and updates cell/client ability state.", "server_db_query", "sgw_player.abilities", "Compare exact caller known-ID set after base result; active-effect tool is insufficient.", "No automatic inverse; respec is wider than undo.", "Native event binding and submission unproved.", ["Positive existing ability ID not already known."]),
    row!("/gmgivetrainingpoints", "progression grants", "Add training points to your GM character.", "caller player", Implemented,
        Persistent, "Base guarded UPDATE RETURNING persists additive training points, then mirrors them.", "server_db_query", "sgw_player.training_points", "Compare exact caller points after base completion; cell/client mirrors are separate.", "No inverse; a later spend is a different action.", "Native event binding and submission unproved.", ["Positive point count with no INT32 overflow."]),
];

fn specific_preconditions(word: &str) -> &'static [&'static str] {
    match word {
        "/gmmissionassign" => &["Positive numeric design ID; loaded mission definition and first step; mission offerable and not already held"],
        "/gmmissionclear" | "/gmmissionabandon" => &["Positive numeric ID of an active mission held by the caller"],
        "/gmmissionlist" | "/gmmissionlistfull" => &["Live caller mission tracker; feedback reflects loaded tracker state"],
        "/gmmissiondetails" => &["Positive numeric ID of a mission in the caller's loaded tracker"],
        "/gmmissionadvance" => &["Active held mission and positive step ID owned by that mission"],
        "/gmshowtargetlocation" => &["Valid in-view target in the same space, or caller fallback"],
        "/gmlistabilities" => &["Valid in-view target in the same space, or caller fallback"],
        "/gmshowflag" => &["Explicit bit index 0–31 required by server handler; omitted patch argument cannot satisfy it"],
        "/gmgetmobattribute" => &["Explicit same-space target ID and supported attribute required by server; patch omits target ID"],
        "/gmshowmobcount" => &["Loaded space ID, or 0 for caller's space; count covers live SGWMob NPCs (class 0x04) only"],
        "/gmlistinteractions" | "/gmshowip" | "/gmshowinventory" => &["No server handler; target and effect cannot be established from patch text"],
        "/gmshowplayer" => &["Server expects explicit same-space target ID; patch supplies no target bytes"],
        "/gmgivexp" => &["Positive XP and connected caller character with a matching persistent player row"],
        "/gmgiveitem" => &["Positive numeric item template ID, quantity at least 1 (server clamps above 1000), and base inventory capacity/container acceptance"],
        "/gmgivecash" => &["Positive naquadah amount and matching persistent caller row"],
        "/gmremoveitem" => &["Server requires caller-owned inventory instance ID and positive INT16 quantity; patch supplies ItemName"],
        "/gmgiveability" => &["Existing positive ability ID not already known; matching caller player row"],
        "/gmgivetrainingpoints" => &["Positive points; current matching player session and DB row; INT32 balance headroom"],
        _ => &[], // Unknown-effect mission rows have no impact/precondition badge.
    }
}

/// Build the first 27 forms in TOML order; changed words, events or arguments fail closed.
pub fn slash_descriptors_1a() -> Result<Vec<SlashDescriptor1a>, String> {
    let entries = parse_patch_table(PATCH_TOML)?;
    descriptors_for(&entries)
}

pub(crate) fn descriptors_for(entries: &[SlashEntry]) -> Result<Vec<SlashDescriptor1a>, String> {
    if ROWS.len() != ARGUMENTS.len() || ROWS.len() != EVENTS.len() {
        return Err("slash evidence table lengths disagree".into());
    }
    let slice = entries
        .get(..ROWS.len())
        .ok_or("patch slash slice is too short")?;
    slice
        .iter()
        .zip(ROWS.iter().zip(ARGUMENTS).zip(EVENTS))
        .map(|(source, ((row, expected_args), expected_event))| {
            if source.name != row.word
                || source.event != *expected_event
                || source.parameters.len() != row.arg_help.len()
            {
                return Err(format!("slash evidence mismatch at {}", row.word));
            }
            let actual_args = source
                .parameters
                .iter()
                .map(|parameter| {
                    let kind = match parameter.parameter_type {
                        SlashParameterType::Boolean => "Boolean",
                        SlashParameterType::Integer => "Integer",
                        SlashParameterType::Float => "Float",
                        SlashParameterType::String => "String",
                    };
                    format!(
                        "{}{kind} {}",
                        if parameter.required { "" } else { "?" },
                        parameter.name
                    )
                })
                .collect::<Vec<_>>()
                .join(";");
            if actual_args != *expected_args {
                return Err(format!("slash parameter mismatch at {}", row.word));
            }
            let args = source
                .parameters
                .iter()
                .zip(row.arg_help)
                .map(|(parameter, help)| ArgDescriptor {
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
                    help: (*help).into(),
                    sensitive: false,
                })
                .collect();
            let verify = (!row.read_tool.is_empty()).then(|| VerifyRecipe {
                read_tools: vec![row.read_tool.into()],
                comparison_fields: row.read_fields.split(';').map(str::to_owned).collect(),
            });
            let descriptor = CommandDescriptor {
                id: source.name.clone(),
                channel: CommandChannel::Slash,
                category: row.category.into(),
                aliases: Vec::new(),
                summary: row.help.into(),
                example: None,
                availability: Availability::Unsupported,
                disposition_reason: Some(format!(
                    "{}{} Native submission unproved. See {EVIDENCE}#{}",
                    if row.impact.is_none() {
                        "Original effect unknown; "
                    } else {
                        ""
                    },
                    row.blocker,
                    &row.word[1..]
                )),
                manual_pending: true,
                args,
                target: TargetPolicy {
                    kind: TargetKind::None,
                    selection_rule: SelectionRule::None,
                    argument_name: None,
                },
                impact: row.impact.map(|kind| Impact {
                    kind,
                    summary: row.effect.into(),
                    preconditions: [
                        "Live server-sourced GameMaster actor in this instance".into(),
                        row.target.into(),
                    ]
                    .into_iter()
                    .chain(specific_preconditions(row.word).iter().map(|p| (*p).into()))
                    .collect(),
                }),
                verify,
                compensation: Some(if row.compensation.is_empty() {
                    Compensation::None
                } else {
                    Compensation::Manual(row.compensation.into())
                }),
                metadata_version: COMMAND_METADATA_VERSION,
            };
            Ok(SlashDescriptor1a {
                source: source.clone(),
                descriptor,
                handler_availability: row.handler,
                target_scope: row.target.into(),
                readback_note: row.readback.into(),
                evidence: format!("{EVIDENCE}#{}", &row.word[1..]),
            })
        })
        .collect()
}

/// Format one UI preview value only. This does not serialize a native slash line.
pub fn display_argument(value: Option<&Value>, kind: ArgKind) -> Option<String> {
    match (value, kind) {
        (None, _) => Some("(omitted)".into()),
        (Some(Value::Bool(value)), ArgKind::Boolean) => Some(value.to_string()),
        (Some(Value::Number(value)), ArgKind::Integer) if value.is_i64() || value.is_u64() => {
            Some(value.to_string())
        }
        (Some(Value::String(value)), ArgKind::Text) => Some(value.clone()),
        _ => None,
    }
}
