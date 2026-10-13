//! Real SmartFox loop guard for the lab's bounded Livewire activity view.

use tokio::net::TcpListener;

use super::tests::read_until_game_begin;
use super::*;
use crate::minigame::game::create_game;

#[tokio::test]
async fn livewire_commands_and_abort_are_visible_after_teardown() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let mut client = TcpStream::connect(addr).await.unwrap();
    let (server, _) = listener.accept().await.unwrap();
    let registry = SessionRegistry::new();
    let ticket = registry
        .register(
            4321,
            7,
            "Livewire".into(),
            1,
            30,
            17,
            0,
            0,
            1,
            vec![],
            None,
            Some(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
        )
        .await
        .unwrap();
    let session = registry
        .authenticate_and_claim(4321, &ticket, "Livewire")
        .await
        .unwrap();
    let game = create_game(&session).unwrap();
    let (tx, mut rx) = mpsc::channel(16);
    let running_registry = registry.clone();
    let handle = tokio::spawn(async move {
        let end = run_session(
            server,
            &running_registry,
            &tx,
            session,
            game,
            ReadState::empty(),
            Duration::from_secs(30),
        )
        .await;
        running_registry.remove_if_ticket(4321, &ticket).await;
        end
    });
    read_until_game_begin(&mut client).await;
    assert_eq!(registry.board_snapshot(4321).await.unwrap().state, "ready");
    for (cmd, wire) in [
        ("opendoor", None),
        ("processover", Some("g1")),
        ("processmove", Some("g1")),
    ] {
        let vars = match wire {
            Some(wire) => format!("<var n='cmd' t='s'>{cmd}</var><obj o='param' t='a'><var n='wirename' t='s'>{wire}</var></obj>"),
            None => format!("<var n='cmd' t='s'>{cmd}</var>"),
        };
        let frame = format!("<msg t='xt'><body action='xtReq' r='-1'><![CDATA[<dataObj>{vars}</dataObj>]]></body></msg>");
        send_null_terminated(&mut client, &frame).await.unwrap();
    }
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let board = registry.board_snapshot(4321).await.unwrap();
            if board.events.iter().any(|e| e.event == "processmove") {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("game loop must record commands");
    drop(client);
    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .unwrap()
        .unwrap();
    let board = registry.board_snapshot(4321).await.unwrap();
    assert_eq!(board.outcome, Some("canceled"));
    assert!(board
        .events
        .iter()
        .any(|e| e.event == "processover" && e.wire_name.as_deref() == Some("g1")));
    assert!(board
        .events
        .iter()
        .any(|e| e.event == "processmove" && e.wire_name.as_deref() == Some("g1")));
    assert!(board.events.iter().any(|e| e.event == "result_canceled"));
    assert!(board.events.iter().any(|e| e.event == "ended"));
    assert!(matches!(
        rx.try_recv(),
        Ok(CellToBaseMsg::MinigameResult {
            result_code: RESULT_CANCELED,
            ..
        })
    ));
}
