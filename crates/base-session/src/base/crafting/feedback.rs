//! The crafting rejection path (D-CR14): every refused request gets a
//! visible text line, and an `onErrorCode` where a condition code fits.
//!
//! The text is the legacy `feedback()` line: `onPlayerCommunication` from
//! speaker `SYSTEM` on `CHAN_FEEDBACK`, built by the one shared serializer in
//! `cimmeria_wire::cell::chat`. That is the path CR-E1 found reaching the
//! player's chat. Whether the client shows anything for `onErrorCode` is
//! unresolved (CR-E1), so the code is only ever a secondary signal, sent
//! after the text and only for the ASP codes 213/214.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;

use crate::base::helpers::send_to_witness_reliable;
use crate::base::ConnectedClientState;
use crate::cell::messages::CraftVerb;
use crate::mercury::{build_player_entity_method_packet, method_idx};
use cimmeria_cell_catalog::crafting::CONDITION_FEEDBACK_NOT_ENOUGH_APPLIED_SCIENCE_POINTS;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

/// Why a crafting request was refused. Later packets add one variant per
/// reason; each needs a `reason`, a `text` and an `error_code` arm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CraftReject {
    /// The verb has no server implementation yet.
    NotAvailableYet {
        /// What the player tried, as a sentence subject ("Alloying").
        action: &'static str,
    },
    /// The server could not decide the request (no database, the catalog
    /// or the transaction failed). Nothing changed.
    Unavailable {
        /// What the player tried, as a sentence subject.
        action: &'static str,
    },
    /// `spendAppliedSciencePoints` named a discipline the catalog does not
    /// have. The 2009 client only offers catalog disciplines, so this is a
    /// forged or stale request.
    UnknownDiscipline { discipline_id: i32 },
    /// The discipline is already known. Also what a replayed spend gets.
    DisciplineAlreadyKnown { name: String },
    /// No unspent applied science points.
    NoAppliedSciencePoints,
    /// The discipline's racial paradigm is below the level it needs.
    ParadigmTooLow {
        discipline: String,
        paradigm: &'static str,
        required: i32,
        have: i32,
    },
    /// A required discipline is unknown or below expertise 50.
    PrerequisiteMissing {
        discipline: String,
        prerequisite: String,
    },
}

impl CraftReject {
    /// The rejection for a verb whose handler has not landed.
    pub fn not_available(verb: &CraftVerb) -> Self {
        let action = match verb {
            CraftVerb::Spend { .. } => "Learning disciplines",
            CraftVerb::Craft { .. } => "Crafting",
            CraftVerb::Research { .. } => "Research",
            CraftVerb::ReverseEngineer { .. } => "Reverse engineering",
            CraftVerb::Alloy { .. } => "Alloying",
            CraftVerb::Respec => "Crafting respec",
        };
        CraftReject::NotAvailableYet { action }
    }

    /// The `reason` field of the `crafting` `rejected` log event.
    pub fn reason(&self) -> &'static str {
        match self {
            CraftReject::NotAvailableYet { .. } => "not_available_yet",
            CraftReject::Unavailable { .. } => "unavailable",
            CraftReject::UnknownDiscipline { .. } => "unknown_discipline",
            CraftReject::DisciplineAlreadyKnown { .. } => "already_known",
            CraftReject::NoAppliedSciencePoints => "not_enough_asp",
            CraftReject::ParadigmTooLow { .. } => "paradigm_too_low",
            CraftReject::PrerequisiteMissing { .. } => "prerequisite_missing",
        }
    }

    /// The line the player reads.
    pub fn text(&self) -> String {
        match self {
            CraftReject::NotAvailableYet { action } => {
                format!("{action} is not available yet.")
            }
            CraftReject::Unavailable { action } => {
                format!("{action} is unavailable right now. Nothing was changed.")
            }
            CraftReject::UnknownDiscipline { discipline_id } => {
                format!("There is no discipline {discipline_id}.")
            }
            CraftReject::DisciplineAlreadyKnown { name } => format!("You already know {name}."),
            CraftReject::NoAppliedSciencePoints => {
                "You have no applied science points.".to_string()
            }
            CraftReject::ParadigmTooLow {
                discipline,
                paradigm,
                required,
                have,
            } => format!(
                "{discipline} requires {paradigm} paradigm level {required}; yours is {have}."
            ),
            CraftReject::PrerequisiteMissing {
                discipline,
                prerequisite,
            } => format!("{discipline} requires {prerequisite} at expertise 50."),
        }
    }

    /// The `EConditionHandlerFeedback` value sent as a secondary
    /// `onErrorCode`. Only the ASP codes qualify (213/214, the
    /// `CONDITION_FEEDBACK_*` constants in `cimmeria_cell_catalog::crafting`);
    /// every other reason is text only.
    pub fn error_code(&self) -> Option<u16> {
        match self {
            CraftReject::NoAppliedSciencePoints => {
                Some(CONDITION_FEEDBACK_NOT_ENOUGH_APPLIED_SCIENCE_POINTS)
            }
            CraftReject::NotAvailableYet { .. }
            | CraftReject::Unavailable { .. }
            | CraftReject::UnknownDiscipline { .. }
            | CraftReject::DisciplineAlreadyKnown { .. }
            | CraftReject::ParadigmTooLow { .. }
            | CraftReject::PrerequisiteMissing { .. } => None,
        }
    }
}

/// `EErrorCodeSystem::ERRORCODE_SYSTEM_Ability`, the only system the enum
/// defines.
const ERRORCODE_SYSTEM_ABILITY: u8 = 0;

/// The seven `onErrorCode` argument bytes: `UINT8 SystemID = 0,
/// INT32 InstanceID = 0, UINT16 ErrorCodeID`, little-endian. A crafting
/// rejection names no ability, so `InstanceID` is 0.
pub fn error_code_args(code: u16) -> Vec<u8> {
    let mut args = Vec::with_capacity(7);
    args.push(ERRORCODE_SYSTEM_ABILITY);
    args.extend_from_slice(&0i32.to_le_bytes());
    args.extend_from_slice(&code.to_le_bytes());
    args
}

/// The `onPlayerCommunication` argument bytes of a crafting feedback line:
/// speaker `SYSTEM`, flags 0, channel `CHAN_FEEDBACK`, then `text`.
pub fn feedback_text_args(text: &str) -> Vec<u8> {
    serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text)
}

/// Refuse a crafting request: log `crafting` `rejected`, send the
/// `CHAN_FEEDBACK` text line to the player's own client and, where
/// [`CraftReject::error_code`] maps one, `onErrorCode` after it.
pub async fn reject(
    entity_id: u32,
    player_id: i32,
    reject: &CraftReject,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    tracing::info!(
        target: "crafting",
        event = "rejected",
        entity_id,
        player_id,
        reason = reject.reason(),
        "crafting request rejected"
    );
    let text_args = feedback_text_args(&reject.text());
    send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        entity_id,
        |key, version, seq, acks| {
            build_player_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_idx::ON_PLAYER_COMMUNICATION,
                &text_args,
                version,
            )
        },
    )
    .await;
    if let Some(code) = reject.error_code() {
        let args = error_code_args(code);
        send_to_witness_reliable(
            transport,
            connected,
            entity_to_addr,
            entity_id,
            |key, version, seq, acks| {
                build_player_entity_method_packet(
                    key,
                    seq,
                    acks,
                    entity_id,
                    method_idx::ON_ERROR_CODE,
                    &args,
                    version,
                )
            },
        )
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};
    use cimmeria_mercury::encryption::EncryptionVersion;

    const ENTITY: u32 = 4260;
    const PLAYER_ID: i32 = 4261;

    fn one_session() -> (
        SocketAddr,
        Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
        Arc<Mutex<HashMap<u32, SocketAddr>>>,
    ) {
        let addr: SocketAddr = "127.0.0.1:55720".parse().unwrap();
        let connected = Arc::new(Mutex::new(HashMap::from([(
            addr,
            test_default_connected_client_state(),
        )])));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(ENTITY, addr)])));
        (addr, connected, entity_to_addr)
    }

    /// `reject` sends exactly one packet to the player's own client: the
    /// `CHAN_FEEDBACK` line carrying the rejection text, byte for byte. It
    /// also logs `crafting` `rejected` with the reason. Removing the send (a
    /// silent rejection, the D-CR14 bug shape) fails the packet count; a
    /// line on any other channel fails the byte comparison.
    #[tokio::test]
    async fn reject_sends_the_text_line_to_the_players_client() {
        let capture = LogCapture::install();
        let typed = Arc::new(TestTransport::new());
        let transport: Arc<dyn Transport> = typed.clone();
        let (addr, connected, entity_to_addr) = one_session();
        let why = CraftReject::not_available(&CraftVerb::Respec);

        reject(
            ENTITY,
            PLAYER_ID,
            &why,
            &transport,
            &connected,
            &entity_to_addr,
        )
        .await;

        let sent = typed.filter_to(addr);
        assert_eq!(sent.len(), 1, "exactly the text line, no onErrorCode");
        assert_eq!(typed.len(), 1, "nothing to any other address");
        let expected = build_player_entity_method_packet(
            &[0u8; 32],
            0,
            &[],
            ENTITY,
            method_idx::ON_PLAYER_COMMUNICATION,
            &feedback_text_args("Crafting respec is not available yet."),
            EncryptionVersion::V1,
        );
        assert_eq!(sent[0], expected);

        let event = capture
            .find_event(
                tracing::Level::INFO,
                "crafting request rejected",
                "not_available_yet",
            )
            .expect("rejected event");
        assert_eq!(event.target, "crafting");
        assert!(
            event.has_field("player_id", &PLAYER_ID.to_string()),
            "{event:#?}"
        );
    }

    #[test]
    fn not_available_text_names_the_action() {
        let text = |verb| CraftReject::not_available(&verb).text();
        assert_eq!(
            text(CraftVerb::Spend { discipline_id: 21 }),
            "Learning disciplines is not available yet."
        );
        assert_eq!(
            text(CraftVerb::Craft {
                blueprint_id: 412,
                items: vec![],
                quantity: 1
            }),
            "Crafting is not available yet."
        );
        assert_eq!(
            text(CraftVerb::Research {
                item_id: 1,
                kickers: vec![]
            }),
            "Research is not available yet."
        );
        assert_eq!(
            text(CraftVerb::ReverseEngineer { item_id: 1 }),
            "Reverse engineering is not available yet."
        );
        assert_eq!(
            text(CraftVerb::Alloy {
                blueprint_id: 42,
                current_tier_item_id: 1,
                lower_tier_items: vec![]
            }),
            "Alloying is not available yet."
        );
        assert_eq!(
            text(CraftVerb::Respec),
            "Crafting respec is not available yet."
        );
    }

    /// The line is `SYSTEM`, flags 0, on `CHAN_FEEDBACK` (9), then the text.
    #[test]
    fn feedback_text_rides_chan_feedback() {
        let args = feedback_text_args("hi");
        let speaker_end = 4 + "SYSTEM".len() * 2;
        assert_eq!(u32::from_le_bytes(args[0..4].try_into().unwrap()), 6);
        assert_eq!(args[speaker_end], 0, "speaker flags");
        assert_eq!(args[speaker_end + 1], CHAN_FEEDBACK, "channel");
        assert_eq!(
            CHAN_FEEDBACK, 9,
            "CHAN_feedback rides the registered tell channel"
        );
        assert_eq!(&args[speaker_end + 2..speaker_end + 6], &2u32.to_le_bytes());
        assert_eq!(&args[speaker_end + 6..], &[b'h', 0, b'i', 0]);
    }

    /// `onErrorCode`: system 0, instance 0, then the code as UINT16.
    #[test]
    fn error_code_args_are_system_instance_code() {
        assert_eq!(error_code_args(214), [0, 0, 0, 0, 0, 0xD6, 0x00]);
    }

    /// A reason that maps a condition code (no ASP -> 214) sends the text
    /// line first, then `onErrorCode(0, 0, 214)`: two packets, in that
    /// order, both to the player. Unmapping the code, or sending the code
    /// instead of the text, fails the comparison.
    #[tokio::test]
    async fn coded_reject_sends_the_text_then_on_error_code() {
        let typed = Arc::new(TestTransport::new());
        let transport: Arc<dyn Transport> = typed.clone();
        let (addr, connected, entity_to_addr) = one_session();

        reject(
            ENTITY,
            PLAYER_ID,
            &CraftReject::NoAppliedSciencePoints,
            &transport,
            &connected,
            &entity_to_addr,
        )
        .await;

        let packet = |seq, method, args: &[u8]| {
            build_player_entity_method_packet(
                &[0u8; 32],
                seq,
                &[],
                ENTITY,
                method,
                args,
                EncryptionVersion::V1,
            )
        };
        assert_eq!(
            typed.filter_to(addr),
            vec![
                packet(
                    0,
                    method_idx::ON_PLAYER_COMMUNICATION,
                    &feedback_text_args("You have no applied science points."),
                ),
                packet(1, method_idx::ON_ERROR_CODE, &[0, 0, 0, 0, 0, 0xD6, 0x00]),
            ]
        );
    }

    /// Only the ASP reason carries a code; every other spend reason is
    /// text only (D-CR14).
    #[test]
    fn only_not_enough_asp_maps_a_condition_code() {
        let texts_only = [
            CraftReject::Unavailable {
                action: "Learning disciplines",
            },
            CraftReject::UnknownDiscipline { discipline_id: 9 },
            CraftReject::DisciplineAlreadyKnown { name: "X".into() },
            CraftReject::ParadigmTooLow {
                discipline: "X".into(),
                paradigm: "Common",
                required: 5,
                have: 1,
            },
            CraftReject::PrerequisiteMissing {
                discipline: "X".into(),
                prerequisite: "Y".into(),
            },
        ];
        for why in texts_only {
            assert_eq!(why.error_code(), None, "{why:?}");
        }
        assert_eq!(CraftReject::NoAppliedSciencePoints.error_code(), Some(214));
    }
}
