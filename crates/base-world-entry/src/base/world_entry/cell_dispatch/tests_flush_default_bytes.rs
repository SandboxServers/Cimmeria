//! The ordinary deferred-AoI flush's wire output, pinned to bytes captured
//! from the code before the lab chaos flush modes (CH-07) touched it.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::transport::Transport;

use crate::base::deferred_aoi::DeferredAoiMsg;
use crate::test_support::{test_default_connected_client_state, TestTransport};

const WITNESS: u32 = 300;

/// 28 NPC introductions, then one of each tail message. The leave is for an
/// entity the client already had, so lifecycle segmenting keeps one segment.
fn buffer() -> Vec<DeferredAoiMsg> {
    let mut msgs: Vec<DeferredAoiMsg> = (0..28u32)
        .map(|i| DeferredAoiMsg::EnteredAoI {
            entity_id: 2000 + i,
            class_id: (i % 3) as u8,
            position: [i as f32 * 3.5, 12.25, -(i as f32)],
            direction: [0.0, i as f32 * 0.2, 0.0],
            level: 1 + i,
            npc_data: None,
            player_data: None,
        })
        .collect();
    msgs.push(DeferredAoiMsg::EntityMethodCall {
        entity_id: WITNESS,
        method_index: 0x42,
        args: vec![0x01, 0x02],
    });
    msgs.push(DeferredAoiMsg::WitnessEntityMethod {
        entity_id: 2003,
        method_index: 3,
        args: vec![0x00, 0x00, 0x40, 0x00],
        entity_is_player: false,
    });
    msgs.push(DeferredAoiMsg::EntityInvisible { entity_id: 2004 });
    msgs.push(DeferredAoiMsg::LeftAoI { entity_id: 9999 });
    msgs
}

/// FNV-1a, 64-bit: a fixed hash so the pin does not depend on a crate.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

const ADDR: &str = "127.0.0.1:54801";

async fn flush_default() -> Vec<Vec<u8>> {
    let addr: SocketAddr = ADDR.parse().unwrap();
    let typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed.clone();
    let mut state = test_default_connected_client_state();
    state.deferred_aoi_msgs = buffer();
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(WITNESS, addr)])));
    super::deferred_flush::flush_deferred_aoi(
        WITNESS,
        addr,
        "on_client_ready",
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    let session = MercuryEncryption::from_session_key([0u8; 32]);
    typed
        .filter_to(addr)
        .iter()
        .map(|raw| session.decrypt(raw).unwrap())
        .collect()
}

/// `(length, FNV-1a)` of every decrypted packet [`buffer`]'s flush sent at
/// 900aa0751, the commit before CH-07 moved the bundle send into
/// `intro_bundles.rs`: the phase-1 packet, seven phase-2 fragments, then the
/// tail (method call, witness method, invisible, leave). Captured by running
/// this flush on that commit's `deferred_flush.rs`; the CH-07 code matched it.
const PRE_CH07: [(usize, u64); 12] = [
    (1041, 0x7d9e_bf73_0a08_586d),
    (1313, 0x5d6d_2360_86c8_ad23),
    (1313, 0xd94a_8022_4580_d480),
    (1313, 0xf8d7_7bdf_1c0e_de82),
    (1313, 0x9090_4abc_10bc_4b5d),
    (1313, 0xf78a_6796_e608_7dcf),
    (1313, 0x4629_6d4c_506f_ca29),
    (471, 0xa5be_7e43_82c9_4d15),
    (15, 0x0a88_d69d_271b_dee2),
    (16, 0x8abb_6e5a_6fb9_4bdf),
    (11, 0x3b22_86ff_4026_932c),
    (22, 0x2026_9fec_e2e6_b6c4),
];

async fn default_hashes() -> Vec<(usize, u64)> {
    flush_default()
        .await
        .iter()
        .map(|p| (p.len(), fnv1a(p)))
        .collect()
}

/// The ordinary flush, in this build (with or without `lab-chaos`), sends
/// exactly what it sent before CH-07: same packet count, same bytes in every
/// phase-1, phase-2 and tail packet. Fails on any change to the default
/// shape, which the chaos modes must never reach unset.
#[tokio::test]
async fn default_flush_matches_the_pre_ch07_bytes() {
    assert_eq!(default_hashes().await, PRE_CH07);
}

#[cfg(feature = "lab-chaos")]
mod with_registry {
    use std::time::{Duration, Instant};

    use cimmeria_base_session::base::lab_chaos::FlushChaos;
    use cimmeria_base_session::base::lab_chaos::LabChaos;

    use super::super::flush_chaos::lend_registry;
    use super::*;

    fn registry_with(addr: SocketAddr, entity: u32, mode: FlushChaos) -> &'static LabChaos {
        let registry: &'static LabChaos = Box::leak(Box::new(LabChaos::new()));
        registry
            .set(
                addr,
                entity,
                "flush test".into(),
                None,
                Some(mode),
                60,
                Instant::now(),
            )
            .expect("a valid set");
        registry
    }

    async fn hashes_with(registry: Option<&'static LabChaos>) -> Vec<(usize, u64)> {
        lend_registry(registry);
        let hashes = default_hashes().await;
        lend_registry(None);
        hashes
    }

    /// Unset chaos changes no byte against the pre-CH-07 capture: an empty
    /// registry, chaos for another address, and chaos left by the last
    /// character on this address (cleared by the flush). Fails if `apply`
    /// shapes a flush it should leave alone.
    #[tokio::test]
    async fn flush_without_chaos_is_byte_identical() {
        let empty: &'static LabChaos = Box::leak(Box::new(LabChaos::new()));
        assert_eq!(hashes_with(Some(empty)).await, PRE_CH07, "empty");

        let other = "127.0.0.1:1".parse().unwrap();
        let other = registry_with(other, WITNESS, FlushChaos::Shift { len: 5 });
        assert_eq!(hashes_with(Some(other)).await, PRE_CH07, "other address");

        let last_character = registry_with(
            ADDR.parse().unwrap(),
            WITNESS + 1,
            FlushChaos::Shift { len: 5 },
        );
        assert_eq!(
            hashes_with(Some(last_character)).await,
            PRE_CH07,
            "chaos set for the previous character on this address"
        );
        assert_eq!(
            last_character.flush_for(ADDR.parse().unwrap(), Instant::now()),
            None,
            "the stale entry is cleared, so it cannot reach a later flush"
        );
    }

    /// `Delay` set: the flush itself neither waits nor changes a byte. The
    /// delay belongs to the AoI hold, so a hold's release passes are never
    /// delayed again. The old flush slept here and took the whole delay.
    #[tokio::test(start_paused = true)]
    async fn delay_set_flush_sends_at_once_and_unchanged() {
        let delay = registry_with(
            ADDR.parse().unwrap(),
            WITNESS,
            FlushChaos::Delay { ms: 1500 },
        );
        let start = tokio::time::Instant::now();
        assert_eq!(hashes_with(Some(delay)).await, PRE_CH07);
        assert_eq!(start.elapsed(), Duration::ZERO, "the flush never waits");
    }
}
