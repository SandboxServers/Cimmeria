//! BaseApp service -- Mercury UDP listener for persistent entity state
//! and client connections.
//!
//! See `docs/protocol/login-handshake.md` for the full wire-level spec.

use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cimmeria_common::EntityId;
use cimmeria_mercury::channel::Channel;
use cimmeria_mercury::encryption::{EncryptionVersion, MercuryEncryption};

use serde::Serialize;

use crate::mercury::{PlayerLoadData, WorldEntryInfo};

// ── Submodules ───────────────────────────────────────────────────────────────

pub(crate) mod character;
pub(crate) mod character_create;
pub(crate) mod chardef;
pub(crate) mod connect_loop;
pub(crate) mod console_authoring;
pub(crate) mod contact_list;
pub(crate) mod cooked_data;
pub(crate) mod crafting;
pub(crate) mod deferred_aoi;
pub(crate) mod dialog_overrides;
pub(crate) mod dispatch;
pub(crate) mod gm_feedback;
pub(crate) mod gm_spawn;
pub(crate) mod helpers;
pub(crate) mod item_overrides;
pub(crate) mod login;
pub(crate) mod mission_overrides;
pub(crate) mod organization;
pub(crate) mod outbox;
pub(crate) mod resources;
mod service;
pub(crate) mod tick_sync;
pub(crate) mod world_entry;
pub(crate) mod world_entry_appearance;
pub(crate) mod world_entry_chat;

#[cfg(test)]
mod smoke_tests;

pub use service::BaseService;

// ── Error types ───────────────────────────────────────────────────────────────

/// Errors specific to the base service.
#[derive(Debug, thiserror::Error)]
pub enum BaseError {
    #[error("Entity {0} not found")]
    EntityNotFound(EntityId),

    #[error("Entity creation failed: {0}")]
    CreationFailed(String),

    #[error("Service not running")]
    NotRunning,

    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),
}

// ── Public snapshot for admin API ─────────────────────────────────────────────

/// A snapshot of one connected player, safe to serialize for the admin API.
#[derive(Debug, Clone, Serialize)]
pub struct OnlinePlayer {
    pub id: u32,
    pub name: String,
    pub archetype: &'static str,
    pub level: i32,
    pub zone: String,
    pub ping: Option<u32>,
    pub status: &'static str,
    pub session: String,
}

fn archetype_name(id: i32) -> &'static str {
    match id {
        1 => "Soldier",
        2 => "Commando",
        3 => "Scientist",
        4 => "Archaeologist",
        5 => "Asgard",
        6 => "Goa'uld",
        7 => "Jaffa",
        _ => "Unknown",
    }
}

// ── Per-connection state ──────────────────────────────────────────────────────

/// Deferred world-entry finalization that must wait for `SGWPlayer.onClientReady`.
pub(crate) struct PendingClientReadyInfo {
    pub entity_id: u32,
    pub player_id: i32,
    pub world_name: String,
    pub appearance_args: Vec<u8>,
    pub tint_args: Vec<u8>,
    /// Non-zero if this is the player's first-ever login. Drives the
    /// deferred `onPlayMovie` (intro cinematic) send in
    /// `handle_on_client_ready` — see issue #288. Type mirrors the DB
    /// column (`sgw_player.first_login INT`) and `PlayerLoadData::first_login`.
    pub first_login: i32,
}

/// State held for each client that has completed the Phase 3 handshake.
pub(crate) struct ConnectedClientState {
    pub enc: MercuryEncryption,
    pub key: [u8; 32],
    /// Wire-encryption version this session speaks, for BOTH directions and
    /// every handshake/outbound builder. Pinned at login from server config;
    /// `enc` is constructed with this version, so it never disagrees with the
    /// version the outbound builders use. Server-wide today — no per-client
    /// negotiation yet.
    pub enc_version: EncryptionVersion,
    pub account_id: u32,
    /// Human-readable account name (login username), threaded from the
    /// login ticket. Surfaced in Discord notifications alongside the
    /// numeric `account_id`. `None` only if a future login path forgets
    /// to populate it.
    pub account_name: Option<String>,
    /// Account access level, populated from the login row. Used by
    /// chat dispatch to set the `SPEAKER_GM` bit on `speaker_flags`
    /// when `access_level > 0` (matches
    /// `python/base/Chat.py::getSpeakerFlags`). 0=Player, 1=Moderator,
    /// 2=GameMaster per `crates/commands/src/permissions.rs::AccessLevel`.
    pub access_level: u32,
    /// DND auto-reply message. `Some(_)` means DND is active and the
    /// chat dispatch sets `SPEAKER_DND` (0x04) on outgoing
    /// `onPlayerCommunication`; `None` means not in DND. Set/cleared
    /// by the `chatSetDNDMessage` (CHAT_SET_DND, 0xC4) handler. A
    /// payload whose decoded message is empty or 1-char clears the
    /// field, matching `python/base/SGWPlayer.py::chatSetDNDMessage`.
    ///
    /// AFK (`chatSetAFKMessage`) is a separate auto-reply-tells feature
    /// and does NOT contribute to `ESpeakerFlags` — the enum
    /// (`entities/defs/enumerations.xml`) has no `SPEAKER_AFK` token,
    /// and the Python reference (`python/base/Chat.py::getSpeakerFlags`)
    /// only checks `accessLevel > 0` and `dndMessage is not None`.
    pub dnd_message: Option<String>,
    pub char_list_sent: bool,
    pub world_entry_sent: bool,
    pub pending_player_entity_id: Option<u32>,
    pub player_entity_id: Option<u32>,
    pub next_seq: Arc<AtomicU32>,
    /// Sequence counter for **unreliable** outbound packets — kept separate
    /// from `next_seq` to preserve the contiguous reliable seq stream the
    /// SGW BigWorld client's `UnAckedHandler::queueAckForPacket`
    /// (`ghidra://SGW.exe@0x0158cba0`) requires. The receiver's `inSeqAt`
    /// advances by exactly 1 per reliable arrival; sharing the counter
    /// with unreliable emissions creates gaps the receiver cannot fill,
    /// stalling delivery of every subsequent reliable packet. The receiver
    /// has separate dedup state at `+0x128` for unreliable packets, so two
    /// independent monotonic streams (one reliable, one unreliable) are
    /// the wire-format-correct shape. See `spec.protocol.mercury-wire-format`
    /// §1.7 + the `FUN_0158bb50` decompile.
    pub next_seq_unreliable: Arc<AtomicU32>,
    pub pending_acks: Arc<Mutex<Vec<u32>>>,
    pub last_recv: Arc<Mutex<Instant>>,
    /// Wall-clock instant the session completed Phase 3 (channel registered).
    /// Distinct from [`last_recv`], which slides forward on every packet —
    /// this one is fixed at connect so logout/disconnect emits can report a
    /// true `session_secs` rather than idle time.
    ///
    /// [`last_recv`]: Self::last_recv
    pub connected_at: Instant,
    pub account_entity_id: u32,
    pub next_data_id: u16,
    pub pending_world_entry: Option<WorldEntryInfo>,
    pub pending_player_load_data: Option<PlayerLoadData>,
    pub pending_map_loaded: Option<WorldEntryInfo>,
    pub pending_client_ready: Option<PendingClientReadyInfo>,
    /// Buffer of AoI-class cell→base messages held back while the client
    /// is still in the pre-`onClientReady` world-entry window. Flushed
    /// from [`crate::base::world_entry_appearance::handle_on_client_ready`]
    /// once the client signals it's ready to receive entity data.
    ///
    /// Without this gate, the cell would fire CREATE_ENTITY + property
    /// cascade for every NPC in the spawned space the moment the player
    /// is added as a witness — that's ~33+ reliable packets sent during
    /// the 3.5s window while the client is loading terrain and cannot
    /// ACK. The TX window then fills before `mapLoaded`'s own burst can
    /// run. See [`deferred_aoi`] for the buffer semantics.
    pub deferred_aoi_msgs: Vec<deferred_aoi::DeferredAoiMsg>,
    pub cached_appearance_args: Option<Vec<u8>>,
    pub cached_tint_args: Option<Vec<u8>>,
    /// Tracks whether the player is currently rendering holstered (no
    /// weapon visual in the wire `ComponentList`) or drawn (weapon visible).
    /// Mirrored from the cell's `CellEntity::weapon_holstered` via
    /// [`crate::cell::messages::CellToBaseMsg::RefreshAppearance`]. All
    /// `BeingAppearance`-emit sites read this so they keep the holster
    /// state consistent across spawn, inventory refresh, AoI rebroadcast,
    /// and combat enter/exit.
    ///
    /// Defaults to `true` so a freshly-connected client spawns weapon-down
    /// (matches the Phase 1 design and
    /// `docs/architecture/state-field-bits.md`).
    pub weapon_holstered: bool,
    pub cancelled: Arc<AtomicBool>,
    /// Cancellation flag for the post-cinematic appearance-spam guard
    /// (issue #288). `world_entry_appearance::send_cinematic` resets this
    /// to `false` and spawns a spam loop that polls it each iteration;
    /// `handle_cancel_movie` flips it to `true` when the client emits a
    /// real cancelMovie so the spam stops short of its full duration.
    pub cinematic_spam_cancel: Arc<AtomicBool>,
    pub player_name: Option<String>,
    pub player_level: Option<i32>,
    pub player_archetype: Option<i32>,
    pub world_name: Option<String>,
    pub player_xp: Option<u64>,
    pub player_training_points: Option<u32>,
    /// The DB `player_id` of the character the client selected via
    /// `playCharacter`. Set during the play-character flow and used by
    /// gate-travel / respawn so they target the active character instead
    /// of falling back to "lowest player_id for the account" — which is
    /// wrong on multi-character accounts.
    pub active_player_id: Option<i32>,
    /// Cross-world ring transport carry-through. Set in
    /// `handle_gate_travel` when the cell `Effect::TeleportCrossWorld`
    /// passes a ring id; consumed in
    /// `world_entry_appearance::handle_client_ready` once the destination
    /// world's `onClientReady` arrives, then forwarded to the cell as a
    /// `BaseToCellMsg::AdvanceRingDestination` so the destination ring's
    /// FSM can leave `RemoteLoadWait`. Stays `None` for stargate dial
    /// gate-travel.
    pub pending_destination_ring_id: Option<i32>,

    /// Reliable-UDP channel state. Tracks the TX window of in-flight
    /// reliable packets, processes incoming ACKs from the client,
    /// maintains the per-peer adaptive RTO, and drives retransmits.
    ///
    /// The legacy send path still assigns sequences via [`next_seq`]
    /// and calls `socket.send_to` directly; reliable sends mirror their
    /// encrypted bytes into this `Channel` via `register_sent_packet`
    /// after the socket send succeeds. ACK consumption + RTO sampling
    /// happen on every received packet (`connect_loop/encrypted.rs`);
    /// retransmits fire from the per-session `tick_sync` loop every
    /// 100 ms, capped at `RETRANSMIT_BUDGET_PER_TICK` entries per scan.
    ///
    /// When a send arrives via `register_sent_packet` while the TX window
    /// is at its `TX_WINDOW_SIZE` cap, the Channel queues the entry in its
    /// `unsent_packets` deque rather than rejecting it; queued entries are
    /// promoted into the window FIFO as ACKs free slots. This replaces
    /// the prior downgrade-reliable-send-to-best-effort path. (The other
    /// reliable entry point, `Channel::send_packet`, still errors on
    /// overflow — it's used only by tests and unmigrated paths and so
    /// never hits the saturating bursts that motivated the queue.)
    ///
    /// Wrapped in `Mutex` because `process_acks`, `register_sent_packet`,
    /// and `check_timeouts` all need `&mut self` and run from different
    /// code paths (receive loop, per-send-site call sites, retransmit tick).
    pub channel: Mutex<Channel>,
}

impl ConnectedClientState {
    /// Next sequence number for an **unreliable** outbound packet —
    /// fetch-add on the unreliable counter, masked to the 28-bit Mercury
    /// sequence space. Use this from any code path that sends a packet
    /// without `FLAG_RELIABLE`; the reliable path uses `next_seq` directly
    /// because reliable seqs are also tracked by the per-session Channel's
    /// TX window. Encapsulated so a future caller can't accidentally drop
    /// the `SEQUENCE_MASK` clamp and overflow into the 4-bit reserved
    /// flag space (see issue #292 for the previous instance of that
    /// class of bug).
    pub fn next_unreliable_seq(&self) -> u32 {
        self.next_seq_unreliable
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            & cimmeria_mercury::packet::SEQUENCE_MASK
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mercury::SKIN_TINTS;
    use cimmeria_common::ServerConfig;

    #[test]
    fn new_service_is_not_running() {
        let config = ServerConfig::default();
        let svc = BaseService::new(&config);
        assert!(!svc.is_running);
        assert_eq!(svc.listener_addr.port(), 32832);
    }

    #[tokio::test]
    async fn start_sets_running() {
        let config = ServerConfig {
            base_port: 0,
            ..ServerConfig::default()
        };
        let mut svc = BaseService::new(&config);
        svc.start().await.unwrap();
        assert!(svc.is_running);
    }

    #[tokio::test]
    async fn create_entity_fails_when_not_running() {
        let config = ServerConfig::default();
        let svc = BaseService::new(&config);
        let result = svc.create_base_entity().await;
        assert!(result.is_err());
    }

    #[test]
    fn skin_tints_array_length() {
        assert_eq!(SKIN_TINTS.len(), 16);
    }

    #[test]
    fn skin_tints_all_nonzero() {
        for (i, &tint) in SKIN_TINTS.iter().enumerate() {
            assert_ne!(tint, 0, "SKIN_TINTS[{i}] should not be zero");
        }
    }

    #[test]
    fn skin_tints_index_0_matches_python() {
        assert_eq!(SKIN_TINTS[0], 0x2F1308FF);
    }
}
