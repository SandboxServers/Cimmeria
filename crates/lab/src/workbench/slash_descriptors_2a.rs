//! Evidence-backed disabled palette forms for patch-012 slash words 55–81.

use serde::Serialize;
use serde_json::Value;

use super::commands::{
    ArgDescriptor, ArgKind, Availability, CommandChannel, CommandDescriptor, Compensation, Impact,
    ImpactKind, SelectionRule, TargetKind, TargetPolicy, VerifyRecipe, COMMAND_METADATA_VERSION,
};
use super::slash_inventory::{parse_patch_table, SlashEntry, SlashParameterType};

const PATCH_TOML: &str =
    include_str!("../../../../data/client-patches/012-gm-slash-commands/commands.toml");
const EVIDENCE: &str = "docs/analysis/dev-workbench/evidence/DW-26A-E.md";
const START: usize = 54;

/// Server handler status is independent of the unproved client binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HandlerAvailability {
    Implemented,
    Absent,
}

/// One third-slice form with its source, target scope and readback limits.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SlashDescriptor2a {
    pub source: SlashEntry,
    pub descriptor: CommandDescriptor,
    pub handler_availability: HandlerAvailability,
    pub target_scope: String,
    pub readback_note: String,
    pub evidence: String,
}

struct Row {
    word: &'static str,
    event: &'static str,
    arguments: &'static str,
    category: &'static str,
    summary: &'static str,
    target: &'static str,
    handler: HandlerAvailability,
    impact: Option<ImpactKind>,
    effect: &'static str,
    precondition: &'static str,
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
    ($kind:ident) => {
        Some(ImpactKind::$kind)
    };
}

macro_rules! row {
    ($word:literal, $event:literal, $arguments:literal, $category:literal,
     $summary:literal, $target:literal, $handler:ident, $impact:ident,
     $effect:literal, $precondition:literal, $tool:literal, $fields:literal,
     $readback:literal, $compensation:literal, $blocker:literal, [$($arg:literal),* $(,)?]) => {
        Row { word: $word, event: $event, arguments: $arguments, category: $category,
            summary: $summary, target: $target, handler: HandlerAvailability::$handler,
            impact: impact!($impact), effect: $effect, precondition: $precondition,
            read_tool: $tool, read_fields: $fields, readback: $readback,
            compensation: $compensation, blocker: $blocker, arg_help: &[$($arg),*] }
    };
}

const ROWS: &[Row] = &[
    row!("/gmreloadinventory", "Event_SlashCmd_ReloadInventory", "", "inventory", "Reload inventory definitions or state (intent unverified).", "inventory definitions versus caller inventory unknown", Absent, Unknown,
        "No inventory reload is implemented.", "No reload primitive follows the GM gate.", "", "", "Inventory snapshots establish baseline only; no accepted reload readback.", "", "Handler and target semantics absent; native binding/submission unproved.", []),
    row!("/gmusers", "Event_SlashCmd_Users", "", "players", "List players in the caller's current space.", "players in caller's current space, not shard-wide", Implemented, ReadOnly,
        "Reports sorted player entity and optional character IDs to the caller.", "Caller must be in a live space.", "server_entity_query", "caller space_id;is_player=true;capped=false;entity_id;total_matched", "Compare gmUsers feedback with an uncapped contemporaneous player-entity query in the same caller space; server_sessions has no space ID.", "", "Native event binding and submission unproved.", []),
    row!("/gmsethidegm", "Event_SlashCmd_SetHideGM", "Boolean on/off", "visibility", "Set GM hide state (unimplemented).", "presumed caller visibility; server target unknown", Absent, Unknown,
        "No hide/show state is implemented.", "No server bHideGM state or handler exists.", "", "", "Client appearance cannot prove an absent server state mutation.", "", "Handler absent; Boolean encoding and native binding/submission unproved.", ["Patch Boolean on/off; native encoding unproved."]),
    row!("/gmprintstats", "Event_SlashCmd_PrintStats", "String Stat", "inspection", "Print a named stat (unimplemented).", "self, selection or server-wide counters unknown", Absent, Unknown,
        "No index-168 print handler is routed.", "Stat names and scope are not validated by a handler.", "", "", "Stat snapshots are baseline only, not method-specific feedback.", "", "Handler, stat scope, quoting and native binding/submission unproved.", ["Stat name; allowed values and quoting unknown."]),
    row!("/gmdebugability", "Event_SlashCmd_AbilityDebug", "Integer AbilityId", "combat debug", "Toggle debug lines for a caller ability; zero clears.", "caller casts and debug subscription", Implemented, Transient,
        "Toggles caller debugAbilityList, or clears it on zero; grants no ability.", "Nonzero INT32 ability ID must exist in cell definitions.", "client_chat_log", "channel=Feedback;prefix=gmDebugAbility:;on/off/clear;caller identity;later combat cast identity", "First-press Feedback-channel exact gmDebugAbility: prefix and later caller/ability-matched combat lines corroborate a transient toggle; there is no stable query or DB row.", "Repeat toggle or send zero to clear after verifying current state.", "Native binding, integer formatting and submission unproved.", ["INT32 candidate: zero clears, nonzero must name a known ability; client bounds unproved."]),
    row!("/gmdebugcombat", "Event_SlashCmd_CombatDebug", "", "combat debug", "Toggle caller combat-debug lines.", "caller combat-debug stream", Implemented, Transient,
        "Toggles hit-roll and damage debug output without combat mutation.", "Live caller required.", "client_chat_log", "channel=Feedback;prefix=gmDebugCombat:;on/off;caller identity;later combat cast identity", "First-press Feedback-channel exact gmDebugCombat: on/off prefix and later caller/cast-matched lines corroborate active state, not persistence.", "Repeat toggle after reading current feedback.", "Native event binding/submission unproved.", []),
    row!("/gmdebugcombatverbose", "Event_SlashCmd_CombatDebugVerbose", "", "combat debug", "Toggle verbose caller combat-debug lines.", "caller verbose combat-debug stream", Implemented, Transient,
        "Toggles effect-plan, NVP, ledger and pulse detail on later hits.", "Live caller required.", "client_chat_log", "channel=Feedback;prefix=gmDebugCombatVerbose:;on/off;caller identity;later verbose combat cast identity", "First-press Feedback-channel exact gmDebugCombatVerbose: on/off prefix and later caller/cast-matched verbose lines; no persistent DB state.", "Repeat toggle after reading current feedback.", "Native event binding/submission unproved.", []),
    row!("/gmdebugheal", "Event_SlashCmd_HealDebug", "", "combat debug", "Toggle caller heal/buff debug lines.", "caller heal/buff debug stream", Implemented, Transient,
        "Toggles later heal/buff detail; does not heal the caller.", "Live caller required.", "client_chat_log", "channel=Feedback;prefix=gmDebugHeal:;on/off;caller identity;later heal cast identity", "First-press Feedback-channel exact gmDebugHeal: on/off prefix and later caller/cast-matched heal lines; no persistent readback.", "Repeat toggle after reading current feedback.", "Native event binding/submission unproved.", []),
    row!("/gmdebugstartminigame", "Event_SlashCmd_DebugStartMinigame", "Integer gameId", "minigame", "Start a debug minigame (unimplemented tail method).", "caller minigame session intent", Absent, Unknown,
        "Index 173 does not start a game; inherited index 20 is a separate logging stub.", "A real game starts through a separate ticketed content path.", "", "", "Minigame state/tap can show an independent session, not acceptance of this method.", "", "Tail handler absent; game ID range and native binding/submission unproved.", ["Game ID; client range and encoding unproved."]),
    row!("/gmdebugspectateminigame", "Event_SlashCmd_DebugSpectateMinigame", "", "minigame", "Spectate a minigame (unimplemented tail method).", "spectator/session target unknown", Absent, Unknown,
        "Index 174 has no spectate transition; inherited index 21 is a different logging stub.", "No working spectator routing exists.", "", "", "A minigame snapshot cannot attribute spectate success to this call.", "", "Tail handler and native binding/submission unproved.", []),
    row!("/gmdebugjoinminigame", "Event_SlashCmd_DebugJoinMinigame", "", "minigame", "Join a minigame (unimplemented tail method).", "session target unknown", Absent, Unknown,
        "Index 175 has no join transition; inherited index 22 is a different logging stub.", "No working join routing exists.", "", "", "Minigame presence alone cannot prove this call.", "", "Tail handler and native binding/submission unproved.", []),
    row!("/gmdebugabilityonmob", "Event_SlashCmd_DebugAbilityOnMob", "Integer AbilityID", "combat debug", "Toggle debug lines for a selected NPC ability; zero means all.", "selected NPC in caller's space", Implemented, Transient,
        "Toggles a caller debug subscription to selected mob casts; no ability grant.", "Selected same-space NPC and known nonzero INT32 ability ID required.", "client_chat_log", "channel=Feedback;prefix=gmDebugAbilityOnMob:;selected mob entity_id;ability ID;later mob cast identity", "First-press Feedback-channel exact gmDebugAbilityOnMob: prefix plus later selected-mob/ability-matched cast lines; no DB state.", "Repeat on the same mob/ability; gmDebugAbility 0 clears caller lists.", "Native selection, argument encoding and submission unproved.", ["Zero means all selected mob abilities; nonzero ID must exist."]),
    row!("/gmdebugbehaviorsonmob", "Event_SlashCmd_DebugBehaviorsOnMob", "", "NPC debug", "Show NPC behavior debug output (unimplemented).", "selection implied by word, unverified", Absent, Unknown,
        "No behavior-debug stream is routed from index 177.", "No handler-side target check exists.", "", "", "Entity AI/threat reads are baseline, not debug-stream acceptance.", "", "Handler, selection and native binding/submission unproved.", []),
    row!("/gmdebugpathsonmob", "Event_SlashCmd_DebugPathsOnMob", "", "NPC debug", "Show NPC path debug output (unimplemented).", "selected mob implied, unverified", Absent, Unknown,
        "No path stream or onShowPath callback is routed from index 178.", "No path-debug handler exists.", "", "", "server_entity_get gives position/AI, not nav_path or a path-stream receipt.", "", "Handler, selection and native binding/submission unproved.", []),
    row!("/gmdebugevents", "Event_SlashCmd_DebugEvents", "Integer TargetId;Integer InformLevel", "event debug", "Subscribe to target event debug output (intent unverified).", "numeric target type and level semantics unknown", Absent, Unknown,
        "No event-debug subscription is routed from index 179.", "No target or InformLevel validation exists.", "", "", "Event logs alone cannot prove an absent subscription.", "", "Handler, target/level semantics and native binding/submission unproved.", ["Numeric TargetId; entity/player/event-source meaning unknown.", "InformLevel enum and numeric bounds unknown."]),
    row!("/gmmobdata", "Event_SlashCmd_MobData", "Integer SpaceId;Integer TargetId", "NPC inspection", "Print data for an explicit same-space entity.", "explicit entity TargetId in caller space; not mob-only", Implemented, ReadOnly,
        "Reports ID, template, AI, faction, level, health, threat count and tag.", "Valid INT32 TargetId in caller space; SpaceId is not used for authority.", "server_entity_get", "caller space_id;entity_id;template_id;ai_state;faction;level;health;tag", "Compare feedback with exact same-space entity at capture time; threat count has no independent LabEntitySnapshot readback and ephemeral fields may age.", "", "Native binding, two-integer formatting and submission unproved.", ["SpaceId is supplied but server ignores it for authority.", "TargetId must identify an entity in caller's current space."]),
    row!("/gmdebuginteract", "Event_SlashCmd_DebugInteract", "", "interaction debug", "Debug interaction handling (unimplemented).", "interaction subject unknown", Absent, Unknown,
        "No interaction call is routed from index 181.", "No handler-side target check exists.", "", "", "No method-specific readback exists.", "", "Handler, target and native binding/submission unproved.", []),
    row!("/gmemitbehavioreventonmob", "Event_SlashCmd_EmitBehaviorEventOnMob", "Integer EventID", "NPC behavior", "Emit an NPC behavior event (unimplemented).", "mob selection implied but unverified", Absent, Unknown,
        "No event emission is implemented; future AI/content effects unknown.", "No validated NPC target or event catalog exists for this method.", "", "", "AI/log baseline cannot prove event emission.", "", "Handler, event catalog, target and native binding/submission unproved.", ["EventID catalog and valid range unknown."]),
    row!("/gmaddbehavioreventset", "Event_SlashCmd_AddBehaviorEventSet", "", "NPC behavior", "Attach a behavior event set (unimplemented).", "event set and mob target unknown", Absent, Unknown,
        "No set attachment is implemented; persistence intent unknown.", "Canonical later QA duplicate has no parameters or target validation.", "", "", "No accepted-effect readback exists.", "", "Handler, set/target semantics and native binding/submission unproved.", []),
    row!("/gmremovebehavioreventset", "Event_SlashCmd_RemoveBehaviorEventSet", "", "NPC behavior", "Remove a behavior event set (unimplemented).", "event set and mob target unknown", Absent, Unknown,
        "No set removal is implemented; future effect unknown.", "Canonical later QA duplicate has no parameters or target validation.", "", "", "No accepted-effect readback exists.", "", "Handler, set/target semantics and native binding/submission unproved.", []),
    row!("/gmspawnbycmd", "Event_SlashCmd_Spawn", "String DesignId;Float X-Offset;Float Z-Offset", "world spawn", "Spawn a numeric NPC template near the caller.", "new NPC at caller X/Z offsets in current world and space", Implemented, Transient,
        "Queues GmSpawnNpc; base resolves template and Cell confirms spawn; queueing alone is not success.", "Live caller/world, positive numeric WSTRING template ID and finite FLOAT offsets required.", "client_chat_log;server_log_tail;server_entity_get", "caller Feedback completion;GmSpawnNpc completion result;returned entity_id;template_id;space_id;position", "Require actor-bound base/cell completion in logs and Feedback, then inspect its returned NPC ID, template and position in caller space; a queued send or entity snapshot alone is not success.", "Manual despawn by verified ID; triggered behavior is not rolled back.", "Event_SlashCmd_Spawn is only a semantic candidate; native binding, quoting and submission unproved.", ["Positive numeric template ID encoded as patch String; names refused.", "Finite X offset; client bounds unknown.", "Finite Z offset; client bounds unknown."]),
    row!("/gmdespawn", "Event_SlashCmd_Despawn", "", "world spawn", "Despawn a selected NPC, if the native sentinel is proven.", "selected same-space NPC only if stock sentinel zero is sent", Implemented, Transient,
        "Removes NPC from spatial grid, sends LeftAoI and releases combat.", "Server requires INT32 TargetID; zero selects current NPC; rejects players and other-space targets.", "client_chat_log;server_entity_query;server_packet_tap_read", "caller Feedback success;target entity_id;space_id;capped=false;target absent;LeftAoI target and witness IDs", "Require actor-bound success feedback, uncapped same-space roster absence and exact LeftAoI witness packet evidence; refusal or absence alone is not success.", "Manual respawn from known template/position is not exact restoration.", "Patch has no TargetID; zero-sentinel encoding, native binding and submission unproved.", []),
    row!("/gmrechargeitem", "Event_SlashCmd_RechargeItem", "Integer ItemId", "inventory", "Recharge an item through a GM method (unimplemented).", "item instance versus design ID unknown", Absent, Unknown,
        "No free GM recharge exists; paid vendor recharge is a separate player method.", "No index-187 GM cell-to-base handler; vendor flow requires session, owned item and payment.", "", "", "Vendor charges/naquadah DB changes prove only the separate paid path, never this slash call.", "", "GM handler absent; item ID meaning and native binding/submission unproved.", ["ItemId is not proven to be inventory instance ID or design ID."]),
    row!("/gmsetmobattribute", "Event_SlashCmd_SetMobAttribute", "String attribute;String attributeType;Integer value", "NPC attributes", "Set an NPC attribute (unimplemented and mismatched).", "mob target absent from patch arguments", Absent, Unknown,
        "No attribute setter is routed; read-only getter is a separate method.", "Candidate expects INT32 target before WSTRING attr/type and INT32 val.", "", "", "Entity/stat reads are baseline only, not setter acceptance.", "", "Handler absent; missing target/signature, quoting and native binding/submission unproved.", ["Attribute name; no validated whitelist or quoting.", "Attribute type; allowed values unknown.", "Integer value; client bounds unknown."]),
    row!("/gmrespawn", "Event_SlashCmd_Respawn", "", "player state", "Respawn the calling GM player.", "caller GM player, not selected target", Implemented, Transient,
        "May refill health/focus, clear death flags and re-anchor/broadcast pawn state.", "Caller needs player_id and a current-world respawner; cross-world transfer can fail.", "server_entity_get;client_player_state;server_packet_tap_read;server_log_tail", "actor entity_id;position;health;focus;death flags;authoritative transfer or position snap completion", "gmRespawn feedback may print even after early exit; require actor-bound before/after life and position state plus authoritative transfer or snap evidence in packet/log trail; missing transfer evidence leaves verification unknown.", "No exact inverse for life, position or combat state; manual restore needs prior state.", "Native event binding and submission unproved.", []),
    row!("/gmkilltarget", "Event_SlashCmd_Kill", "Integer TargetId", "combat", "Kill a same-space NPC through the GM method.", "explicit valid NPC ID; stock -1 sentinel selects current NPC", Implemented, Irreversible,
        "Zeros NPC health and runs death, loot, threat and witness effects; grants no GM XP.", "Live NPC target, not already dead; server rejects players and other-space IDs.", "client_chat_log;server_entity_get;server_log_tail;server_packet_tap_read", "caller Feedback success;target entity_id;space_id;health=0;dead state;death event/loot chain;witness death update", "Require actor-bound success feedback, exact NPC dead/health state and death/loot chain plus witness packet evidence; a snapshot or submitted slash line alone cannot verify encounter effects.", "No inverse for death, loot or content; a fresh spawn is another entity.", "Patch Integer versus candidate INT64 and -1 sentinel conversion; native binding/submission unproved.", ["Patch Integer TargetId has no numeric bound; candidate expects INT64 and stock -1 sentinel."]),
    row!("/debugminigamecomplete", "Event_SlashCmd_MinigameComplete", "Integer ResultCode", "minigame debug", "Complete a minigame through a debug method (unimplemented).", "non-gm slash word; candidate GM-tail method, caller minigame only", Absent, Unknown,
        "No method-191 handler; separate authenticated SmartFox outcomes may fire content chains.", "Server GM gate applies even to this non-/gm word; never send client ResultCode to victory chains.", "", "", "Minigame tap and chain logs concern the separate game path, not this command; ticket attribution across replacement is incomplete.", "", "GM handler, result-code parsing and native binding/submission unproved; no dot or cancel substitute.", ["Exactly one patch Integer ResultCode; no game/winner/loser ID; SmartFox 0/1/2 is separate."]),
];

/// Build 27 disabled forms in canonical patch order, rejecting source drift.
pub fn slash_descriptors_2a() -> Result<Vec<SlashDescriptor2a>, String> {
    descriptors_for(&parse_patch_table(PATCH_TOML)?)
}

pub(crate) fn descriptors_for(entries: &[SlashEntry]) -> Result<Vec<SlashDescriptor2a>, String> {
    let slice = entries
        .get(START..START + ROWS.len())
        .ok_or("patch slash slice 55–81 is missing")?;
    slice
        .iter()
        .zip(ROWS)
        .map(|(source, row)| {
            let actual = source
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
            if source.name != row.word
                || source.event != row.event
                || source.access != "p"
                || actual != row.arguments
                || source.parameters.len() != row.arg_help.len()
            {
                return Err(format!("slash evidence mismatch at {}", row.word));
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
                    sensitive: parameter.parameter_type == SlashParameterType::String,
                })
                .collect();
            let verify = (!row.read_tool.is_empty()).then(|| VerifyRecipe {
                read_tools: row.read_tool.split(';').map(str::to_owned).collect(),
                comparison_fields: row.read_fields.split(';').map(str::to_owned).collect(),
            });
            let descriptor = CommandDescriptor {
                id: source.name.clone(),
                channel: CommandChannel::Slash,
                category: row.category.into(),
                aliases: Vec::new(),
                summary: row.summary.into(),
                example: None,
                availability: Availability::Unsupported,
                disposition_reason: Some(format!(
                    "{} Native submission unproved. See {EVIDENCE}#{}",
                    row.blocker,
                    &row.word[1..]
                )),
                manual_pending: true,
                args,
                target: target_policy(row.word),
                impact: row.impact.map(|kind| Impact {
                    kind,
                    summary: row.effect.into(),
                    preconditions: vec![
                        "Live server-sourced GameMaster actor in this instance".into(),
                        row.target.into(),
                        row.precondition.into(),
                    ],
                }),
                verify,
                compensation: Some(if row.compensation.is_empty() {
                    Compensation::None
                } else {
                    Compensation::Manual(row.compensation.into())
                }),
                metadata_version: COMMAND_METADATA_VERSION,
            };
            Ok(SlashDescriptor2a {
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

fn target_policy(word: &str) -> TargetPolicy {
    match word {
        "/gmdebugabilityonmob" | "/gmdespawn" => TargetPolicy {
            kind: TargetKind::Mob,
            selection_rule: SelectionRule::Required,
            argument_name: None,
        },
        "/gmkilltarget" => TargetPolicy {
            kind: TargetKind::MobOrId,
            selection_rule: SelectionRule::SelectedOrArgument,
            argument_name: Some("TargetId".into()),
        },
        "/gmmobdata" => TargetPolicy {
            kind: TargetKind::AnyEntity,
            selection_rule: SelectionRule::SelectedOrArgument,
            argument_name: Some("TargetId".into()),
        },
        _ => TargetPolicy {
            kind: TargetKind::None,
            selection_rule: SelectionRule::None,
            argument_name: None,
        },
    }
}

/// Format a redacted audit preview, never a native slash line.
pub fn audit_argument(value: &Value, arg: &ArgDescriptor) -> Option<String> {
    if arg.sensitive && matches!(value, Value::String(_)) {
        return Some("(redacted)".into());
    }
    match (value, arg.kind) {
        (Value::Bool(value), ArgKind::Boolean) => Some(value.to_string()),
        (Value::Number(value), ArgKind::Integer) if value.is_i64() || value.is_u64() => {
            Some(value.to_string())
        }
        (Value::Number(value), ArgKind::Float) if value.as_f64().is_some_and(f64::is_finite) => {
            Some(value.to_string())
        }
        _ => None,
    }
}
