//! Filter directives for every log sink, and the routing of OTLP log records
//! between the three SigNoz services.
//!
//! # The parity guarantee (NA25)
//!
//! Every event that reaches an on-disk log file also reaches SigNoz, in
//! exactly one log index:
//!
//! | Level | `service.name` | Filter |
//! |---|---|---|
//! | ERROR, WARN | `cimmeria-server` | [`OTEL_FILTER`] |
//! | INFO, DEBUG | `cimmeria-server`, or `cimmeria-network` for [`otel::is_network_noise_target`] scopes | [`OTEL_FILTER`] |
//! | TRACE | `cimmeria-trace` | [`otel_trace_directives`] |
//!
//! Deliberate exceptions, each pinned by `parity_tests`:
//!
//! - **The `off` targets** in `OTLP_EXCLUDED_TARGETS`, each with its
//!   reason: the exporter's own crates (`hyper`, `h2`, `tonic`, `tower`,
//!   `reqwest`, `opentelemetry`, `tungstenite`), whose export would feed
//!   every batch into the next, and `launcher.key_dump`, which carries a
//!   client session key.
//! - **The per-packet firehoses** (`wire.firehose.*`) reach the files in
//!   full and SigNoz as a counted 1-in-N sample on another target. See
//!   `cimmeria_wire::firehose` (re-exported as `cimmeria_services::firehose`).
//! - Nothing else. A new file layer whose DEBUG rows [`OTEL_FILTER`] does not
//!   cover fails `every_file_directive_reaches_an_otlp_index`.
//!
//! Hand-named `target: "…"` rows match no file layer, so the file guard
//! cannot see them. `target_scan_tests` reads the source of every in-process
//! crate instead and requires each literal target to reach one index at the
//! level it is emitted (round 2 of NA25).
//!
//! The TRACE filter is *derived* from [`FILE_LAYERS`] rather than written out,
//! so a new file layer's TRACE rows are exported without a second edit. DEBUG
//! and above are not derived: [`OTEL_FILTER`] is also the span filter, and
//! whether a new scope belongs in `cimmeria-server` at DEBUG is a decision.

use cimmeria_services::firehose;
use tracing::{Level, Metadata, Subscriber};
use tracing_subscriber::filter::{filter_fn, FilterExt};
use tracing_subscriber::layer::Filter;
use tracing_subscriber::EnvFilter;

use crate::otel;

/// `EnvFilter` directives shared by the OTLP trace layer and the two
/// DEBUG-and-above log layers.
///
/// A custom `target:` that is not named here inherits the leading `info`, so
/// its DEBUG events never reach SigNoz however useful they are.
/// `aoi.create_emit` was built to localize the invisible-static-NPC drop and
/// was absent from the 2026-09-19 repro for exactly that reason.
///
/// A directive's target matches by **string prefix** (`tracing-subscriber`
/// compares `meta.target().starts_with(directive_target)`), and the longest
/// matching directive wins. So `npc_ai=debug` already exports
/// `npc_ai.transition`, `npc_ai.aggro`, `npc_ai.leash`, `npc_ai.tick` and
/// `npc_ai.path_fail`, and `cover=debug` covers every `cover.*` target; but
/// `wire.out=info`
/// needs the more specific `wire.out.avatar_update=debug` beside it to let
/// that one DEBUG sample through. `otel_filter_prefix_matching_exports_npc_ai_children`
/// pins this behaviour, not just the string.
///
/// The `cimmeria_wire::…` rows, `cimmeria_cell_catalog=debug` and
/// `cimmeria_cell_world=debug` keep the code
/// split out of `cimmeria-services` (docs/architecture/services-crate-split.md)
/// at the DEBUG level `cimmeria_services=debug` gave it: a moved module's
/// `module_path!()` target starts with the new crate's name, which the
/// services row no longer matches. Since wave W3a that includes the
/// services-side Mercury glue, `cimmeria_wire::mercury` (the
/// `append_entity_method` appearance diagnostics). Wire's rows name each of
/// its top-level modules rather than the crate: a bare `cimmeria_wire` would
/// prefix-match `cimmeria_wire_log`, so removing wire-log's own row below
/// would change nothing and no guard could tell it was gone (the rule the
/// base and cell rows follow too, which `parity_tests::crate_rows` enforces
/// for every row). The same guard fails when a new top-level wire module has
/// no row. `cimmeria_minigame=debug`
/// does the same for the SmartFoxServer host (wave W3c), which has no file of
/// its own: its session and connection rows reach `server.log` from INFO and
/// SigNoz from DEBUG.
///
/// `cimmeria_wire_log=debug` (wave W3b) does the same for the decoded
/// wire-message stream; no wire row reaches it. The stream's own rows ride
/// the hand-named `wire.in` / `wire.out` directives.
///
/// `cimmeria_base_session=debug` (wave B1) does the same for the BaseApp
/// session layer: the send helpers, tick sync, the outbox, the contact list,
/// the deferred-AoI buffer and cooked-data delivery.
/// `cimmeria_base_methods=debug` (wave B2) does the same for the BaseApp's
/// feature handlers: player load, inventory, vendors, trade, mail, missions
/// and progression. `cimmeria_base_session` is not a prefix of it.
/// `cimmeria_base_world_entry=debug` (wave B3) does the same for world entry:
/// play-character, map load, gate travel, reanchor, teleport, the CellToBase
/// dispatch with its AoI emitters, `onClientReady`, the cinematic AoI hold's
/// release and the character list. Neither base row above is a prefix of it.
/// `cimmeria_base::base=debug` (wave B4) does the same for `BaseService`, the
/// connect loop, login, the SGWPlayer base-method dispatch and the character
/// creator. It names the crate's one top-level module rather than the crate:
/// a bare `cimmeria_base` would prefix-match every `cimmeria_base_*` crate,
/// so removing one of the three rows above would change nothing and no guard
/// could tell it was gone.
///
/// `cimmeria_cell_combat=debug` (wave C2) does the same for combat: ability
/// resolution, damage and death, threat, the effect pulsing, the bandolier
/// and reload handlers, and the NPC AI's behaviour. `cimmeria_cell_cover`
/// shares its `cimmeria_cell_co` prefix but not the name, so neither row
/// matches the other crate.
///
/// `cimmeria_cell_content=debug` (wave C3) does the same for the content
/// layer: the chain executor and its dispatchers, missions, the ring
/// dispatcher and entry points, and the dialog display. Neither
/// `cimmeria_cell_combat` nor `cimmeria_cell_cover` is a prefix of it.
///
/// `cimmeria_cell_console=debug` (wave C5b) does the same for the GM
/// surfaces: the `.`-console with its authoring commands, the chat
/// interceptor and the native `gm*` cell methods. It shares the
/// `cimmeria_cell_co` prefix with the combat, content and cover rows, but no
/// row is a prefix of another.
///
/// `cimmeria_cell_interactions=debug` (wave C4) does the same for the player
/// interactions: the NPC interaction dispatch, stargate travel, the GM space
/// transfer, the respawn fork with its resync, the trade session state and
/// the mail forwarding. No other crate's row is a prefix of it, and it is a
/// prefix of none.
///
/// `cimmeria_cell_methods=debug` (wave C5a) does the same for the
/// client-callable cell methods: the per-interface dispatchers and the
/// SGWPlayer handlers (combat and respawn, interaction, trade, vendors, world,
/// crafting, social). No file layer names them, so they reach `server.log`
/// from INFO and SigNoz from DEBUG, as they did under `cimmeria_services`. No
/// other crate's row is a prefix of it, and it is a prefix of none.
///
/// `cimmeria_cell_pets=debug` (#962, the plugin pilot) does the same for the
/// pets plugin crate: the pet cell methods (88-90) and `PetsPlugin`. Their
/// rows name `pets.*` targets, which the `pets=debug` row exports; this row
/// covers any untargeted row. No other crate's row is a prefix of it, and it
/// is a prefix of none. `cell.plugin=info` exports the plugin table's
/// startup rows (installed, incomplete, invalid).
///
/// `cimmeria_cell_duel=debug` (#962 step 2) does the same for the duel plugin
/// crate: the duel answer and forfeit (cell methods 102-103), the duel tick
/// and engage, and `DuelPlugin`. Their rows name the `duel` target, which the
/// `duel=debug` row exports; this row covers any untargeted row. No other
/// crate's row is a prefix of it, and it is a prefix of none.
///
/// `cimmeria_cell::cell=debug` (wave C6) does the same for the cell service:
/// `CellService`, the cell loop, the base-message handlers, the ticks and the
/// cell-method router. Like `cimmeria_base::base` it names the crate's one
/// top-level module rather than the crate: a bare `cimmeria_cell` would
/// prefix-match every `cimmeria_cell_*` crate (world, combat, content, cover,
/// catalog, interactions, methods, console), so removing any of their rows
/// above would change nothing and no guard could tell it was gone.
///
/// `crafting` (CR-01, `docs/analysis/crafting/`) is the crafting campaign's
/// event target on both halves of the split: the cell's malformed-request
/// drops and forwards, the catalog load, and the base's `request` and
/// `rejected` events (later `induction_started`, `completed`,
/// `persist_failed`). It is `debug` so a later DEBUG row reaches SigNoz
/// without a filter change.
///
/// `mercury.backpressure` is `info`, not `warn` (NA25). Its one emitter is a
/// WARN today, but `server.log` keeps the target from INFO, and the parity
/// rule is that nothing a file keeps is missing from SigNoz; `warn` here
/// would silently drop the first INFO row anyone adds.
///
/// `wire.firehose=debug` is for the same reason: the firehose rows are
/// TRACE, and the TRACE filter turns them off, but a DEBUG row ever emitted
/// on one of those targets reaches its file and so must reach SigNoz.
///
/// `mercury.lossy_transport` (NA37 round 2) is the network-chaos test
/// apparatus's own drop/latency/jitter log, behind `cimmeria-mercury`'s
/// `test-support` feature — never compiled into a release build, but
/// `target_scan_tests` reads source text regardless of feature gating, so it
/// still needs an explicit level here.
///
/// `mercury.rx_order` (NA38) is the reliable receive gate on each client
/// session: DEBUG rows for a packet held behind a gap or dropped as a
/// retransmitted duplicate, a WARN for one beyond the receive window. It
/// only speaks when the client's packets arrive lost or reordered, so it is
/// quiet on a healthy link and exactly the evidence a lossy one needs.
///
/// `mercury.tx_hole` is the same gap seen from the sending side: the client
/// has acked a reliable packet sent after one it never acked, so it is
/// holding every reliable message behind the missing one (no entity
/// creates, leaves or method calls reach it) until a resend lands. DEBUG
/// `tx_hole_open` per loss, WARN `tx_hole_stall` past 2 s, INFO
/// `tx_hole_closed` when a reported stall ends. `debug` so the per-loss rows
/// reach SigNoz: they are the server-to-client loss rate per peer.
///
/// `org` and `squad` (organizations campaign, ORG-01) are the Team/Command
/// and Squad targets. Both are `debug`: the per-call "no handler yet" rows
/// and the later routing decisions are DEBUG, and the coordinator reads
/// SigNoz for these two targets after the two-client UAT.
///
/// `chat` and `rate_limit` (social-systems campaign, SS-00) are the chat
/// path's decisions and the per-player flood limits. `rate_limit` logs
/// `rate_limit.exceeded` at WARN for a drop that notifies the player (at
/// most one per category per player per 5 s) and at DEBUG for the silent
/// drops between; `chat` logs the D-SS12 text-rule refusals and (SS-C2)
/// the GM broadcast: `chat.gm_broadcast` (INFO audit row),
/// `chat.gm_broadcast_delivered` and `chat.gm_broadcast_rejected`, and
/// (SS-C3) the channel allowlist, the GM mutes and the unsupported
/// Communicator methods: `chat.channel_rejected`, `chat.gm_mute` /
/// `chat.gm_unmute` (INFO audit rows), their `_refused` rows,
/// `chat.method_unsupported`, and the DEBUG `chat.muted_refused`. Both are
/// `debug` so the suppressed drops and the muted lines reach SigNoz.
/// `online_index` (SS-00) is the online name index: DEBUG `insert` /
/// `remove` rows with the teardown `path`, and a DEBUG `lookup` row with
/// `reason = missing | ambiguous` for every lookup that does not resolve.
///
/// `mail` (social-systems SS-M1, raised from `info`) is gate mail: INFO
/// `mail.sent` with the recipients and mail ids, WARN `mail.send_refused`
/// with `reason` and `result` for every refused send and the read-side
/// owner misses, and DEBUG rows for the cell's decode verdict
/// (`mail.send_decoded`, `mail.send_decode_rejected`), each failed
/// recipient (`mail.recipient_failed`), an attachment seen
/// (`mail.attachment_seen`) and each header list sent (`mail.headers_sent`).
/// `duel` (SS-D1) is the duel challenge and response path on both the base
/// (`sendDuelChallenge`) and the cell (the registry, the response, the
/// tick): DEBUG rows for every refusal (`reason=`) and state transition,
/// WARN for a payload that does not decode.
/// `bank` (bank-vault campaign, `docs/analysis/bank-vault/`, telemetry
/// contract D-BV19) is the vault target on both halves of the split: the
/// cell's `vault_session_opened` / `vault_session_closed` (DEBUG) and
/// `vault_open_rejected` (WARN), and the base's `move_rejected` /
/// `grant_rejected` (WARN). It is `debug` because the session transitions
/// are DEBUG and the owner debugs bank issues from SigNoz alone.
///
/// `ammo` (ammo campaign, `docs/analysis/ammo/`, telemetry contract in
/// `work-packets.md`) is the special-ammo target on both halves: the
/// startup flag and catalog rows (INFO, WARN), then the packets' reserve
/// draws and returns, damage modifiers and loot drops (DEBUG) and their
/// refusals (WARN). AM-F adds the row before any DEBUG emitter exists so no
/// Wave-1 packet has to edit this file.
pub(crate) const OTEL_FILTER: &str = "info,\
                cimmeria_services=debug,\
                cimmeria_resources=debug,\
                cimmeria_auth=debug,\
                cimmeria_wire::ability_tree=debug,\
                cimmeria_wire::base=debug,\
                cimmeria_wire::black_market=debug,\
                cimmeria_wire::cell=debug,\
                cimmeria_wire::containers=debug,\
                cimmeria_wire::crafting=debug,\
                cimmeria_wire::firehose=debug,\
                cimmeria_wire::hex=debug,\
                cimmeria_wire::mercury=debug,\
                cimmeria_wire::state_field=debug,\
                cimmeria_wire_log=debug,\
                cimmeria_cell_cover=debug,\
                cimmeria_cell_catalog=debug,\
                cimmeria_minigame=debug,\
                cimmeria_base_session=debug,\
                cimmeria_cell_world=debug,\
                cimmeria_base_methods=debug,\
                cimmeria_base_world_entry=debug,\
                cimmeria_cell_combat=debug,\
                cimmeria_base::base=debug,\
                cimmeria_cell_content=debug,\
                cimmeria_cell_console=debug,\
                cimmeria_cell_interactions=debug,\
                cimmeria_cell_methods=debug,\
                cimmeria_cell_pets=debug,\
                cimmeria_cell_duel=debug,\
                cimmeria_cell::cell=debug,\
                cimmeria_mercury=debug,\
                mercury.packet=info,\
                mercury.retransmit=info,\
                mercury.backpressure=info,\
                mercury.lossy_transport=debug,\
                mercury.rx_order=debug,\
                mercury.tx_hole=debug,\
                wire.in=info,wire.out=info,\
                wire.out.avatar_update=debug,\
                wire.out.forced_position=debug,\
                wire.firehose=debug,\
                aoi.entity_enter=debug,aoi.entity_leave=debug,\
                aoi.create_emit=debug,\
                aoi.introduce=debug,\
                movement.npc=debug,movement.player=debug,\
                movement.navmesh=debug,\
                npc_ai=debug,\
                pets=debug,\
                cell.plugin=info,\
                deployables=debug,\
                cover=debug,\
                spawner=debug,\
                content=info,\
                threat=info,\
                auth=info,\
                world_entry=info,\
                vendor=info,progression=info,inventory=info,mission=info,\
                abilities=debug,\
                crafting=debug,\
                content.resolve=debug,\
                dialog.display=debug,\
                mission.step_context=debug,\
                movement.movement_type=debug,\
                movement.position_sample=debug,\
                movement.validation=debug,\
                player.journal=debug,\
                trade.atomic_swap=debug,\
                org=debug,squad=debug,\
                chat=debug,rate_limit=debug,online_index=debug,\
                mail=debug,\
                duel=debug,\
                bank=debug,\
                ammo=debug,\
                console.feedback=debug,\
                client.native=debug,\
                launcher=debug,\
                launcher.key_dump=off,\
                cimmeria_discord=debug,\
                sqlx::query=debug,\
                tungstenite=off,tokio_tungstenite=off,hyper=off,\
                h2=off,tower=off,tonic=off,reqwest=off,opentelemetry=off";

/// Every target [`OTEL_FILTER`] turns `off`, with the reason. These are the
/// only rows the server logs that SigNoz never receives.
///
/// Test-only: nothing routes on it. `parity_tests` checks it equals the `off`
/// set in [`OTEL_FILTER`], and exempts exactly these from the file-parity
/// and source-target guards.
#[cfg(test)]
pub(crate) const OTLP_EXCLUDED_TARGETS: &[(&str, &str)] = &[
    ("tungstenite", EXPORTER_TRANSPORT),
    ("tokio_tungstenite", EXPORTER_TRANSPORT),
    ("hyper", EXPORTER_TRANSPORT),
    ("h2", EXPORTER_TRANSPORT),
    ("tower", EXPORTER_TRANSPORT),
    ("tonic", EXPORTER_TRANSPORT),
    ("reqwest", EXPORTER_TRANSPORT),
    ("opentelemetry", EXPORTER_TRANSPORT),
    (
        "launcher.key_dump",
        "carries the client's session key (`key_b64`); logged at DEBUG only so \
         the default sinks keep it off disk, and it must not leave the host \
         through the exporter either",
    ),
];

/// Reason shared by the exporter's own crates in [`OTLP_EXCLUDED_TARGETS`].
#[cfg(test)]
const EXPORTER_TRANSPORT: &str = "the OTLP exporter's own transport (or the admin \
     WebSocket's): exporting it loops every batch's HTTP/gRPC chatter into the next batch";

/// The per-packet wire stream belongs in protocol.log + OTLP at full
/// fidelity, not in human-facing sinks (console, server.log, admin WS). Mute
/// it in each.
pub(crate) const WIRE_FIREHOSE_MUTED: &str =
    "mercury.packet=warn,wire.in=warn,wire.out=warn,mercury.retransmit=warn";

/// `server.log` (JSON, every module, INFO and above).
pub(crate) fn server_log_directives() -> String {
    format!("info,{WIRE_FIREHOSE_MUTED}")
}

/// One per-system `logs/<file>` layer.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FileLayer {
    pub(crate) file: &'static str,
    pub(crate) directives: &'static str,
}

/// Every per-system log file and what it receives. `init_logging` builds one
/// layer per row and the parity test walks the same rows, so a file cannot be
/// added without the test seeing it.
///
/// Each row starts `off` and names module paths, which is why the
/// `wire.firehose.*` targets are listed explicitly: moving a row off its
/// module-path target would otherwise drop it from its file.
///
/// A module path here must name a module that exists: when a module moves
/// crate its `module_path!()` changes and the row silently stops matching.
/// `stale_target_tests` fails on a path that no longer resolves.
///
/// A row matches by string prefix, so `world_entry.log`'s
/// `cimmeria_base_session::base::world_entry` keeps every `world_entry*`
/// module the session crate took from `cimmeria-services` (wave B1):
/// `world_entry::space_registry`, `world_entry_appearance::builders` and
/// `world_entry_chat`, which the old `cimmeria_services::base::world_entry` row
/// matched the same way. The feature handlers under it, `world_entry::methods`,
/// are `cimmeria_base_methods::base::world_entry::methods` since wave B2, and
/// the rest of world entry, `world_entry_appearance` and `character` are
/// `cimmeria_base_world_entry::base::…` since wave B3; that crate's
/// `world_entry` row keeps `world_entry_appearance` by the same prefix match.
///
/// The cell modules the C4-C6 preparation moved inside `cimmeria-services`
/// keep their files the same way: `interactions.log` names chat at
/// `cell::console::chat`, and `aoi.log` names `cell::respawn::resync`, the
/// client-cache resync and hotbar seed that sat under `cell::service`'s
/// `player_init` before. The trade session state (`cell::trade`), the respawn
/// fork and the GM handlers (`cell::console::gm`) had no file and have none.
/// Since wave C5b the console, chat and the GM handlers are
/// `cimmeria_cell_console::cell::console::…`; chat keeps `interactions.log`.
/// Since wave C4 the interaction handlers, mail, gate travel and the resync
/// are `cimmeria_cell_interactions::cell::…`, beside the content crate's
/// dialog display in `interactions.log`. Since wave C6 the cell service (the
/// loop, the base-message handlers and the ticks) and the cell-method router
/// are `cimmeria_cell::cell::{service, dispatch}`, which `aoi.log` and
/// `dispatch.log` name in place of the old `cimmeria_services::cell::…` rows.
pub(crate) const FILE_LAYERS: &[FileLayer] = &[
    FileLayer {
        file: "auth.log",
        directives: "off,cimmeria_auth::auth=trace",
    },
    FileLayer {
        file: "base.log",
        directives: "off,\
             cimmeria_base::base::service=trace,\
             cimmeria_base::base::connect_loop=trace,\
             cimmeria_base::base::login=trace,\
             cimmeria_base_session::base::tick_sync=trace,\
             cimmeria_base_session::base::helpers=trace,\
             wire.firehose.decrypt=trace,\
             wire.firehose.udp_in=trace",
    },
    FileLayer {
        file: "world_entry.log",
        directives: "off,\
             cimmeria_base_world_entry::base::world_entry=trace,\
             cimmeria_base_session::base::world_entry=trace,\
             cimmeria_base_methods::base::world_entry::methods=trace,\
             wire.firehose.aoi_position=trace",
    },
    FileLayer {
        file: "character.log",
        directives: "off,\
             cimmeria_base_world_entry::base::character=trace,\
             cimmeria_base::base::character_create=trace,\
             cimmeria_resources::base::chardef=trace,\
             cimmeria_base_session::base::cooked_data=trace,\
             cimmeria_resources::base::resources=trace",
    },
    FileLayer {
        file: "protocol.log",
        directives: "off,\
             cimmeria_wire::mercury=trace,\
             cimmeria_mercury=trace,\
             mercury.packet=info,\
             wire.in=info,wire.out=info,\
             mercury.retransmit=info",
    },
    FileLayer {
        file: "aoi.log",
        directives: "off,\
             cimmeria_cell::cell::service=trace,\
             cimmeria_cell_interactions::cell::respawn::resync=trace,\
             cimmeria_cell_world::cell::service=trace,\
             cimmeria_cell_combat::cell::service=trace,\
             cimmeria_cell_world::cell::space_manager=trace,\
             cimmeria_cell_world::cell::space_manager::npc_population=off",
    },
    FileLayer {
        file: "combat.log",
        directives: "off,\
             cimmeria_cell_combat::cell::combat=trace,\
             cimmeria_cell_world::cell::combat=trace,\
             cimmeria_cell_combat::cell::abilities=trace",
    },
    FileLayer {
        file: "content.log",
        directives: "off,cimmeria_cell_content::cell::content=trace",
    },
    FileLayer {
        file: "missions.log",
        directives: "off,cimmeria_cell_content::cell::missions=trace",
    },
    FileLayer {
        file: "interactions.log",
        directives: "off,\
             cimmeria_cell_interactions::cell::interactions=trace,\
             cimmeria_cell_content::cell::interactions=trace,\
             cimmeria_cell_console::cell::console::chat=trace,\
             cimmeria_cell_interactions::cell::mail=trace",
    },
    FileLayer {
        file: "spawner.log",
        directives: "off,\
             cimmeria_cell_catalog::cell::spawner=trace,\
             cimmeria_cell_world::cell::space_manager::npc_population=trace,\
             cimmeria_cell_interactions::cell::gate_travel=trace,\
             cimmeria_cell_content::cell::ring_transport=trace,\
             cimmeria_cell_world::cell::ring_transport=trace",
    },
    FileLayer {
        file: "dispatch.log",
        directives: "off,\
             cimmeria_cell::cell::dispatch=trace,\
             cimmeria_cell_world::cell::dispatch=trace,\
             cimmeria_base::base::dispatch=trace",
    },
];

/// Targets exported at TRACE that no file names: the firehose samples.
pub(crate) const TRACE_ONLY_TARGETS: &[&str] = &[firehose::SAMPLED_TARGET_PREFIX];

/// `(target, level)` for every `target=level` directive; bare levels such as
/// the leading `off` are skipped.
pub(crate) fn directive_pairs(directives: &str) -> impl Iterator<Item = (&str, &str)> {
    directives
        .split(',')
        .map(str::trim)
        .filter_map(|d| d.split_once('='))
}

/// A hand-named target (`npc_ai`, `wire.out`, `movement.navmesh`) rather than
/// a Rust module path. For module paths the file layers are the authority on
/// what is kept at TRACE, so [`OTEL_FILTER`]'s blanket `cimmeria_services=debug`
/// does not become `cimmeria_services=trace`.
///
/// Every workspace crate linked into the server is named `cimmeria_*`, so a
/// target this returns `false` for is a module path or an external crate's
/// path; `stale_target_tests` checks the former exist.
pub(super) fn is_custom_target(target: &str) -> bool {
    !target.contains("::") && !target.starts_with("cimmeria_")
}

/// Directives for the `cimmeria-trace` log index: every target a file layer
/// keeps at TRACE, every custom target [`OTEL_FILTER`] names, and the firehose
/// samples; with `wire.firehose` off.
///
/// Every directive is `=trace` (or `off`), so the longest-match rule reduces
/// to "any of them matches", which is the OR over the file layers this
/// stands for. The firehose rows are skipped rather than listed and
/// overridden, because `wire.firehose.decrypt=trace` is longer than, and
/// would beat, `wire.firehose=off`.
pub(crate) fn otel_trace_directives() -> String {
    otel_trace_directives_for(FILE_LAYERS)
}

/// [`otel_trace_directives`] over an arbitrary file-layer table, so the
/// parity test can prove the guard catches a row the real table lacks.
pub(crate) fn otel_trace_directives_for(file_layers: &[FileLayer]) -> String {
    let file_targets = file_layers
        .iter()
        .flat_map(|l| directive_pairs(l.directives))
        .filter(|(_, level)| level.eq_ignore_ascii_case("trace"))
        .map(|(target, _)| target);
    let custom_targets = directive_pairs(OTEL_FILTER)
        .filter(|(target, level)| !level.eq_ignore_ascii_case("off") && is_custom_target(target))
        .map(|(target, _)| target);

    let mut targets: Vec<&str> = file_targets
        .chain(custom_targets)
        .chain(TRACE_ONLY_TARGETS.iter().copied())
        .filter(|t| !t.starts_with(firehose::FIREHOSE_TARGET_PREFIX))
        .collect();
    targets.sort_unstable();
    targets.dedup();

    let mut out = String::from("off");
    for t in targets {
        out.push(',');
        out.push_str(t);
        out.push_str("=trace");
    }
    out.push(',');
    out.push_str(firehose::FIREHOSE_TARGET_PREFIX);
    out.push_str("=off");
    // A child `OTEL_FILTER` turns off (`launcher.key_dump`) stays off here
    // too, although its parent (`launcher`) was raised to TRACE above.
    for (target, _) in
        directive_pairs(OTEL_FILTER).filter(|(_, level)| level.eq_ignore_ascii_case("off"))
    {
        out.push(',');
        out.push_str(target);
        out.push_str("=off");
    }
    out
}

/// `cimmeria-server`: DEBUG and above, except DEBUG/INFO from network-noise
/// scopes. WARN+ from those scopes still lands here so a real wire problem
/// shows in the primary view.
pub(crate) fn routes_to_server(meta: &Metadata<'_>) -> bool {
    let level = *meta.level();
    // `Level` orders by verbosity: `<= DEBUG` is DEBUG or more severe.
    level <= Level::DEBUG && (!otel::is_network_noise_target(meta.target()) || level <= Level::WARN)
}

/// `cimmeria-network`: INFO and DEBUG from network-noise scopes.
pub(crate) fn routes_to_network(meta: &Metadata<'_>) -> bool {
    let level = *meta.level();
    level <= Level::DEBUG && level > Level::WARN && otel::is_network_noise_target(meta.target())
}

/// `cimmeria-trace`: TRACE only, from every scope. The other two indexes
/// reject TRACE, so no record is indexed twice.
pub(crate) fn routes_to_trace(meta: &Metadata<'_>) -> bool {
    *meta.level() == Level::TRACE
}

/// Filter for the `cimmeria-server` log layer.
pub(crate) fn otel_server_log_filter<S: Subscriber>() -> impl Filter<S> + Send + Sync + 'static {
    EnvFilter::new(OTEL_FILTER).and(filter_fn(routes_to_server))
}

/// Filter for the `cimmeria-network` log layer.
pub(crate) fn otel_network_log_filter<S: Subscriber>() -> impl Filter<S> + Send + Sync + 'static {
    EnvFilter::new(OTEL_FILTER).and(filter_fn(routes_to_network))
}

/// Filter for the `cimmeria-trace` log layer.
pub(crate) fn otel_trace_log_filter<S: Subscriber>() -> impl Filter<S> + Send + Sync + 'static {
    EnvFilter::new(otel_trace_directives()).and(filter_fn(routes_to_trace))
}

/// [`otel_trace_log_filter`] over an arbitrary file-layer table.
#[cfg(test)]
pub(crate) fn otel_trace_log_filter_for<S: Subscriber>(
    file_layers: &[FileLayer],
) -> impl Filter<S> + Send + Sync + 'static {
    EnvFilter::new(otel_trace_directives_for(file_layers)).and(filter_fn(routes_to_trace))
}
