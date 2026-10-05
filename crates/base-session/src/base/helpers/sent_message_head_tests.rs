//! The `mercury.tx_hole` stall WARN names the message a stalled packet
//! starts in (`msg_id`, `msg_name`, plus `method_name` for an entity method).
//! The send path records nothing for it except the head of a bundle fragment
//! past the first, which the fragment plan's own walk already found; every
//! other packet is identified at stall time from its retained bytes.
//!
//! The bug shape: a pre-composed blob of several messages (`createEntity`
//! plus its avatar update, a player-ghost cascade) was recorded as one
//! message, so every later packet of a fragmented introduction reported the
//! blob's first message instead of the one it starts in.

use super::reliable_send::{name_stalled_entry, name_stalled_message};
use super::*;
use cimmeria_mercury::channel::MessageNames;
use cimmeria_mercury::channel_bundle::{ChannelBundle, IDBASE_SGW_PLAYER};
use cimmeria_mercury::packet::MessageHead;

use crate::test_support::{test_default_connected_client_state, TestTransport};

const WITNESS: u32 = 0x5E17_0001;
const OTHER_PLAYER: u32 = 0x5E17_0002;
const MOB: u32 = 0x5E17_0003;
/// `BeingAppearance`, the same method on every being.
const BEING_APPEARANCE: u8 = 26;

/// A session table with the witness's session and another player's.
fn sessions() -> (SocketAddr, HashMap<SocketAddr, ConnectedClientState>) {
    let addr: SocketAddr = "127.0.0.1:55301".parse().unwrap();
    let other: SocketAddr = "127.0.0.1:55302".parse().unwrap();
    let mut witness = test_default_connected_client_state();
    witness.player_entity_id = Some(WITNESS);
    let mut player = test_default_connected_client_state();
    player.player_entity_id = Some(OTHER_PLAYER);
    (addr, HashMap::from([(addr, witness), (other, player)]))
}

/// Send `bundle` to the witness, then name each outstanding packet the way
/// the stall WARN would: what was recorded at send time, and the names.
async fn stall_names_of(bundle: ChannelBundle) -> Vec<(Option<MessageHead>, MessageNames)> {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());
    let (addr, table) = sessions();
    let connected = Arc::new(Mutex::new(table));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(WITNESS, addr)])));

    let outcome =
        send_bundle_to_witness_reliable(&transport, &connected, &entity_to_addr, WITNESS, bundle)
            .await;
    assert!(
        matches!(outcome, BundleSendOutcome::Sent { .. }),
        "{outcome:?}"
    );
    let clients = connected.lock().unwrap();
    let state = &clients[&addr];
    let channel = state.channel.lock().unwrap();
    channel
        .tx_window
        .iter()
        .map(|e| (e.first_message, name_stalled_entry(&clients, &state.enc, e)))
        .collect()
}

/// An entity method message: `[0x80 | index][len][entity_id][args]`.
fn entity_method(index: u8, entity_id: u32, args: usize) -> Vec<u8> {
    let len = u16::try_from(4 + args).unwrap();
    let mut msg = vec![0x80 | index];
    msg.extend_from_slice(&len.to_le_bytes());
    msg.extend_from_slice(&entity_id.to_le_bytes());
    msg.resize(msg.len() + args, 0);
    msg
}

/// One raw append holding `createEntity`, its avatar update and a run of
/// entity methods, large enough to fragment: the second packet starts inside
/// the run and must report a method of the run, not `createEntity`.
#[tokio::test]
async fn a_fragment_inside_a_multi_message_blob_reports_its_own_message() {
    let mut blob = vec![0x09];
    blob.extend_from_slice(&1000u16.to_le_bytes());
    blob.resize(blob.len() + 1000, 0);
    blob.push(0x10);
    blob.resize(blob.len() + 25, 0);
    for _ in 0..20 {
        blob.extend(entity_method(BEING_APPEARANCE, MOB, 60));
    }
    let mut bundle = ChannelBundle::new(true);
    bundle.append_raw_message(&blob);

    let packets = stall_names_of(bundle).await;
    assert!(packets.len() >= 2, "the blob must fragment: {packets:?}");

    // The first fragment records nothing; its own bytes name it.
    let (recorded, first) = packets[0];
    assert_eq!(
        recorded, None,
        "the first packet costs the send path nothing"
    );
    assert_eq!(first.msg_id, Some(0x09));
    assert_eq!(first.msg_name, Some("createEntity"));

    // The second starts inside the method run.
    let (recorded, second) = packets[1];
    let head = recorded.expect("a later fragment carries the plan's head");
    assert_eq!(
        (head.msg_id, head.method_index, head.entity_id),
        (
            0x80 | BEING_APPEARANCE,
            Some(u16::from(BEING_APPEARANCE)),
            Some(MOB)
        ),
        "the second packet starts inside the method run, not in createEntity"
    );
    assert_eq!(second.msg_name, Some("entityMethod"));
    assert_eq!(second.method_name, Some("BeingAppearance"));
}

/// A one-packet send records nothing; the stall decrypts the retained bytes
/// and names its first message.
#[tokio::test]
async fn a_single_packet_is_named_from_its_retained_bytes() {
    let mut bundle = ChannelBundle::new(true);
    bundle.append_entity_method(27, IDBASE_SGW_PLAYER, WITNESS, &[0; 4]);
    let packets = stall_names_of(bundle).await;
    assert_eq!(packets.len(), 1);
    assert_eq!(
        packets[0],
        (
            None,
            MessageNames {
                msg_id: Some(0x80 | 27),
                msg_name: Some("entityMethod"),
                method_name: Some("onSystemCommunication"),
            }
        )
    );
}

/// The stall names the method for the entity's kind: a player (the witness
/// or another session's entity) reads the player table; a mob's 27 is left
/// unnamed, since `onSystemCommunication` is only a player's 27.
#[test]
fn a_stalled_message_is_named_for_its_entity() {
    let (_, clients) = sessions();
    let head = |entity_id: u32, index: u16| MessageHead {
        offset: 0,
        msg_id: 0x80 | u8::try_from(index).unwrap(),
        entity_id: Some(entity_id),
        method_index: Some(index),
    };
    let named = |method_name| MessageNames {
        msg_id: Some(0x80 | 27),
        msg_name: Some("entityMethod"),
        method_name,
    };
    let player_27 = named(Some("onSystemCommunication"));
    assert_eq!(
        name_stalled_message(&clients, &head(WITNESS, 27)),
        player_27
    );
    assert_eq!(
        name_stalled_message(&clients, &head(OTHER_PLAYER, 27)),
        player_27
    );
    assert_eq!(name_stalled_message(&clients, &head(MOB, 27)), named(None));
    let leave = MessageHead {
        offset: 0,
        msg_id: 0x0C,
        entity_id: None,
        method_index: None,
    };
    assert_eq!(
        name_stalled_message(&clients, &leave),
        MessageNames {
            msg_id: Some(0x0C),
            msg_name: Some("leaveAoI"),
            method_name: None,
        }
    );
}
