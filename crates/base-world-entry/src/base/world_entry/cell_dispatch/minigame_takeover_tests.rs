//! A changed Base connection must turn a sent URL into failed launch cleanup.

use super::*;
use crate::minigame::SessionRegistry;
use crate::test_support::test_default_connected_client_state;
use std::future::Future;
use std::io;
use std::net::Ipv4Addr;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};

const TEST_PARAMS: MinigamePlayerParams = MinigamePlayerParams {
    player_level: 1,
    intelligence: 0,
    tech_competency: 1,
    abilities_mask: 0,
};

struct TakeoverTransport {
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    registry: SessionRegistry,
    addr: SocketAddr,
    reopen: bool,
    new_ticket: Mutex<Option<String>>,
    sent: Mutex<Vec<Vec<u8>>>,
    first: AtomicBool,
}

impl Transport for TakeoverTransport {
    fn send_to<'a, 'b, 'f>(
        &'a self,
        bytes: &'b [u8],
        _addr: SocketAddr,
    ) -> Pin<Box<dyn Future<Output = io::Result<usize>> + Send + 'f>>
    where
        'a: 'f,
        'b: 'f,
        Self: 'f,
    {
        Box::pin(async move {
            self.sent.lock().unwrap().push(bytes.to_vec());
            if self.first.swap(false, Ordering::Relaxed) {
                let mut replacement = test_default_connected_client_state();
                replacement.player_entity_id = Some(77);
                replacement.active_player_id = Some(1);
                replacement.key = [9; 32];
                replacement.enc =
                    cimmeria_mercury::encryption::MercuryEncryption::from_session_key([9; 32]);
                self.connected
                    .lock()
                    .unwrap()
                    .insert(self.addr, replacement);
                if self.reopen {
                    assert!(self.registry.cancel(77).await);
                    let ticket = self
                        .registry
                        .register(
                            77,
                            1,
                            "Livewire".into(),
                            1,
                            1,
                            0,
                            0,
                            0,
                            1,
                            vec![],
                            vec![],
                            None,
                            Some(self.addr.ip()),
                            1,
                        )
                        .await
                        .expect("replacement board registers");
                    *self.new_ticket.lock().unwrap() = Some(ticket);
                }
            }
            Ok(bytes.len())
        })
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
    }
}

#[tokio::test]
async fn minigame_start_tx_refusal_cancels_only_its_ticket() {
    for reopen in [false, true] {
        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 46000));
        let mut owner = test_default_connected_client_state();
        owner.player_entity_id = Some(77);
        owner.active_player_id = Some(1);
        owner.key = [3; 32];
        owner.enc = cimmeria_mercury::encryption::MercuryEncryption::from_session_key([3; 32]);
        let connected = Arc::new(Mutex::new(HashMap::from([(addr, owner)])));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(77, addr)])));
        let registry = SessionRegistry::new();
        let transport = Arc::new(TakeoverTransport {
            connected: connected.clone(),
            registry: registry.clone(),
            addr,
            reopen,
            new_ticket: Mutex::new(None),
            sent: Mutex::new(Vec::new()),
            first: AtomicBool::new(true),
        });
        start_minigame(
            77,
            1,
            "Livewire".into(),
            1,
            vec![],
            vec![],
            TEST_PARAMS,
            &(transport.clone() as Arc<dyn Transport>),
            &connected,
            &entity_to_addr,
            &Some(registry.clone()),
            "unused",
            9339,
            &None,
        )
        .await;
        assert_eq!(transport.sent.lock().unwrap().len(), 1);
        {
            let clients = connected.lock().unwrap();
            let replacement = &clients[&addr];
            assert_eq!(replacement.next_seq.load(Ordering::Relaxed), 0);
            assert!(replacement.channel.lock().unwrap().tx_window.is_empty());
        }
        assert_eq!(registry.session_count().await, usize::from(reopen));
        if reopen {
            let ticket = transport.new_ticket.lock().unwrap().clone().unwrap();
            assert!(registry
                .authenticate(77, &ticket, "Livewire")
                .await
                .is_some());
        } else {
            assert!(
                registry
                    .register(
                        77,
                        1,
                        "Livewire".into(),
                        1,
                        1,
                        0,
                        0,
                        0,
                        1,
                        vec![],
                        vec![],
                        None,
                        Some(addr.ip()),
                        1
                    )
                    .await
                    .is_some(),
                "a fresh launch may retry immediately"
            );
        }
    }
}
