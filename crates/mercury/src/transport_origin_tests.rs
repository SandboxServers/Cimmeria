use std::sync::Mutex;

use tokio::sync::Notify;
use tokio::time::{timeout, Duration};

use super::*;

struct PausedTransport {
    entered: Notify,
    release: Notify,
    sent: Mutex<Vec<(SocketAddr, Vec<u8>)>>,
}

impl PausedTransport {
    fn new() -> Self {
        Self {
            entered: Notify::new(),
            release: Notify::new(),
            sent: Mutex::new(Vec::new()),
        }
    }

    fn sent(&self) -> Vec<(SocketAddr, Vec<u8>)> {
        self.sent.lock().unwrap().clone()
    }
}

#[async_trait]
impl Transport for PausedTransport {
    async fn send_to(&self, bytes: &[u8], addr: SocketAddr) -> io::Result<usize> {
        if bytes == b"old" {
            self.entered.notify_one();
            self.release.notified().await;
        }
        self.sent.lock().unwrap().push((addr, bytes.to_vec()));
        Ok(bytes.len())
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok("127.0.0.1:0".parse().unwrap())
    }
}

fn origin(generation: &str, peer: SocketAddr, connected_at: Instant) -> SendOrigin {
    SendOrigin {
        server_boot_id: Arc::from("boot-1"),
        session_generation: Arc::from(generation),
        peer,
        connected_at,
        player_entity_id: Some(501),
    }
}

#[tokio::test]
async fn send_origin_defaults_delegate_exact_bytes_and_pre_phase3_has_no_tag() {
    let fake = Arc::new(PausedTransport::new());
    let transport: Arc<dyn Transport> = fake.clone();
    let peer: SocketAddr = "127.0.0.1:27000".parse().unwrap();
    let connected_at = Instant::now();
    let old_origin = origin("boot-1:1", peer, connected_at);
    let new_origin = origin("boot-1:2", peer, connected_at);

    let old_send = tokio::spawn({
        let transport = Arc::clone(&transport);
        async move {
            transport
                .send_to_with_origin(b"old", peer, &old_origin)
                .await
        }
    });
    timeout(Duration::from_secs(2), fake.entered.notified())
        .await
        .unwrap();

    assert_eq!(
        transport
            .send_to_with_origin(b"new", peer, &new_origin)
            .await
            .unwrap(),
        3
    );
    assert_eq!(transport.send_to_pre_phase3(b"pre", peer).await.unwrap(), 3);
    fake.release.notify_one();
    assert_eq!(
        timeout(Duration::from_secs(2), old_send)
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
        3
    );

    // The fake sees only the literal datagrams and destination. Neither
    // default method puts diagnostic identity on the wire.
    assert_eq!(
        fake.sent(),
        vec![
            (peer, b"new".to_vec()),
            (peer, b"pre".to_vec()),
            (peer, b"old".to_vec()),
        ]
    );
}

struct OriginRecordingTransport {
    inner: Arc<dyn Transport>,
    completed: Mutex<Vec<(SendOrigin, SocketAddr, Vec<u8>)>>,
}

#[async_trait]
impl Transport for OriginRecordingTransport {
    async fn send_to(&self, bytes: &[u8], addr: SocketAddr) -> io::Result<usize> {
        self.inner.send_to(bytes, addr).await
    }

    async fn send_to_with_origin(
        &self,
        bytes: &[u8],
        addr: SocketAddr,
        origin: &SendOrigin,
    ) -> io::Result<usize> {
        let ticket = origin.clone();
        let sent = self.inner.send_to(bytes, addr).await?;
        self.completed
            .lock()
            .unwrap()
            .push((ticket, addr, bytes.to_vec()));
        Ok(sent)
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.inner.local_addr()
    }
}

#[tokio::test]
async fn send_origin_wrapper_keeps_old_ticket_across_paused_await_without_wire_metadata() {
    let fake = Arc::new(PausedTransport::new());
    let inner: Arc<dyn Transport> = fake.clone();
    let wrapper = Arc::new(OriginRecordingTransport {
        inner,
        completed: Mutex::new(Vec::new()),
    });
    let transport: Arc<dyn Transport> = wrapper.clone();
    let peer: SocketAddr = "127.0.0.1:27000".parse().unwrap();
    let connected_at = Instant::now();
    let old_origin = origin("boot-1:1", peer, connected_at);
    let new_origin = origin("boot-1:2", peer, connected_at);

    let old_send = tokio::spawn({
        let transport = Arc::clone(&transport);
        async move {
            transport
                .send_to_with_origin(b"old", peer, &old_origin)
                .await
        }
    });
    timeout(Duration::from_secs(2), fake.entered.notified())
        .await
        .unwrap();

    transport
        .send_to_with_origin(b"new", peer, &new_origin)
        .await
        .unwrap();
    fake.release.notify_one();
    timeout(Duration::from_secs(2), old_send)
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    let completed = wrapper.completed.lock().unwrap().clone();
    assert_eq!(completed.len(), 2);
    assert_eq!(completed[0], (new_origin, peer, b"new".to_vec()));
    assert_eq!(
        completed[1],
        (
            origin("boot-1:1", peer, connected_at),
            peer,
            b"old".to_vec()
        )
    );
    assert_eq!(
        fake.sent(),
        vec![(peer, b"new".to_vec()), (peer, b"old".to_vec())]
    );
}
