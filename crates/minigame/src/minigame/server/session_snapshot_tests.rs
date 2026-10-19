//! Real SmartFox loop guard for the lab's bounded Livewire activity view.

use tokio::net::TcpListener;

use super::tests::read_until_game_begin;
use super::*;
use crate::minigame::game::create_game;

fn command_frame(cmd: &str, wire: Option<&str>) -> String {
    let vars = match wire {
        Some(wire) => format!("<var n='cmd' t='s'>{cmd}</var><obj o='param' t='a'><var n='wirename' t='s'>{wire}</var></obj>"),
        None => format!("<var n='cmd' t='s'>{cmd}</var>"),
    };
    format!("<msg t='xt'><body action='xtReq' r='-1'><![CDATA[<dataObj>{vars}</dataObj>]]></body></msg>")
}

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
        let frame = command_frame(cmd, wire);
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
    let rows: Vec<_> = board
        .events
        .iter()
        .map(|e| (e.dir, e.cmd.as_str()))
        .collect();
    let positions: Vec<_> = [
        ("in", "login"),
        ("out", "loginSucceeded"),
        ("out", "fullgamestate"),
        ("out", "onGameBegin"),
        ("in", "opendoor"),
        ("out", "opendoor"),
        ("in", "processover"),
        ("out", "over"),
        ("in", "processmove"),
        ("out", "destroy"),
    ]
    .iter()
    .map(|row| {
        rows.iter()
            .position(|actual| actual == row)
            .expect("tap row missing")
    })
    .collect();
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "SmartFox rows out of order: {rows:?}"
    );
    assert!(board
        .events
        .windows(2)
        .all(|pair| pair[0].seq + 1 == pair[1].seq));
    let cut = board
        .events
        .iter()
        .find(|e| e.dir == "out" && e.cmd == "destroy")
        .unwrap();
    assert_eq!(cut.params["wirename"], "g1");
    assert!(matches!(
        rx.try_recv(),
        Ok(CellToBaseMsg::MinigameResult {
            result_code: RESULT_CANCELED,
            ..
        })
    ));
}

#[tokio::test]
async fn livewire_victory_outbound_row_precedes_result() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let mut client = TcpStream::connect(addr).await.unwrap();
    let (server, _) = listener.accept().await.unwrap();
    let registry = SessionRegistry::new();
    let ticket = registry
        .register(
            4322,
            7,
            "Livewire".into(),
            1,
            30,
            17,
            0,
            0,
            1,
            vec![],
            vec![],
            None,
            Some(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
        )
        .await
        .unwrap();
    let session = registry
        .authenticate_and_claim(4322, &ticket, "Livewire")
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
        running_registry.remove_if_ticket(4322, &ticket).await;
        end
    });
    read_until_game_begin(&mut client).await;
    for (cmd, wire) in [
        ("opendoor", None),
        ("processmove", Some("g1")),
        ("processmove", Some("g2")),
    ] {
        send_null_terminated(&mut client, &command_frame(cmd, wire))
            .await
            .unwrap();
    }
    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .unwrap()
        .unwrap();
    let board = registry.board_snapshot(4322).await.unwrap();
    assert_eq!(board.outcome, Some("victory"));
    let rows: Vec<_> = board
        .events
        .iter()
        .map(|e| (e.dir, e.cmd.as_str()))
        .collect();
    let destroy = rows
        .iter()
        .rposition(|row| *row == ("out", "destroy"))
        .unwrap();
    let victory = rows
        .iter()
        .position(|row| *row == ("out", "victory"))
        .unwrap();
    let result = rows
        .iter()
        .position(|row| *row == ("internal", "result_victory"))
        .unwrap();
    assert!(
        destroy < victory && victory < result,
        "victory order: {rows:?}"
    );
    assert_eq!(board.goal_cut, 2);
    assert!(matches!(
        rx.try_recv(),
        Ok(CellToBaseMsg::MinigameResult {
            result_code: RESULT_VICTORY,
            ..
        })
    ));
}
