//! ORG-07, D-ORG06: `logOff` drops the Team and Command invites the
//! character holds, on both variants. A return to character select keeps
//! the session, so without this the old character's invites would stay on
//! it (they could not be accepted by the next character, but they would
//! count against its caps). The teardown paths remove the session, and the
//! invites with it.

use std::time::Instant;

use super::super::*;
use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};
use cimmeria_entity::organization::OrgType;

#[tokio::test]
async fn logoff_drops_held_org_invites() {
    for disconnect in [0u8, 1] {
        let addr: SocketAddr = "127.0.0.1:54710".parse().unwrap();
        let entity_id: u32 = 4343;
        let now = Instant::now();
        let mut s = test_default_connected_client_state();
        s.player_entity_id = Some(entity_id);
        s.player_name = Some("Invitee".to_string());
        s.active_player_id = Some(7);
        s.listed_online = true;
        s.org_invites
            .issue(7, 99, "Inviter", 5, OrgType::Command, now)
            .expect("issue");
        assert_eq!(s.org_invites.pending_for(7, now), 1);
        let connected = Arc::new(Mutex::new(HashMap::from([(addr, s)])));
        let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());
        let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
        let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
        let (tx, _rx) = mpsc::channel::<BaseToCellMsg>(8);
        let capture = LogCapture::install();

        dispatch_sgw_player_base_method(
            sgw_player_base::LOG_OFF,
            &[disconnect],
            &Some("Invitee".to_string()),
            addr,
            &transport,
            [0u8; 32],
            &connected,
            &entity_manager,
            &Some(tx),
            &entity_to_addr,
            &None,
        )
        .await
        .expect("logOff must not fail");

        let held = connected.lock().unwrap()[&addr]
            .org_invites
            .pending_for(7, now);
        assert_eq!(held, 0, "logOff({disconnect}) left the invite");
        let row = capture
            .all()
            .into_iter()
            .find(|c| c.has_field("event", "invite_cleared"))
            .expect("invite_cleared");
        assert_eq!(row.target, "org");
        assert!(row.has_field("dropped", "1"), "{:?}", row.fields);
    }
}
