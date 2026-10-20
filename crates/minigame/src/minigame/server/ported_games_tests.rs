//! Every server-validated game, end to end over a real loopback SFS session
//! (MG-71). Each game logs in with its ticket, reads `joinOK` and
//! `onGameBegin`, makes its first golden move from `tests/scripts/`, and then
//! the client sends its own `victory`. The server refuses that victory, so the
//! session reports no victory or defeat; only the canceled result that the
//! socket close produces may follow. The coverage check ties the table to
//! [`GameKind`]: a new server-validated game without a row fails here.

use crate::minigame::games::{GameKind, Validation};

use super::listener_tests::connect_to;
use super::tests::drain_results;
use super::*;

/// The client's own victory command, sent after the first move.
const CLIENT_VICTORY: &str = "<msg t='xt'><body action='xtReq'>\
     <![CDATA[<dataObj><var n='cmd' t='s'>victory</var></dataObj>]]>\
     </body></msg>";

/// The first golden move of one server-validated game: the seed its script
/// starts from, and the first `>` line of that script as a command and
/// parameters.
struct GoldenStart {
    game: &'static str,
    seed: u32,
    command: &'static str,
    params: &'static [(&'static str, &'static str)],
}

/// One row per server-validated game, in [`GameKind::ALL`] order.
const GOLDEN_FIRST_MOVES: [GoldenStart; 5] = [
    GoldenStart {
        game: "Livewire",
        seed: 7,
        command: "processmove",
        params: &[("wirename", "g1")],
    },
    GoldenStart {
        game: "Alignment",
        seed: 7,
        command: "swappieces",
        params: &[
            ("row1", "1"),
            ("col1", "0"),
            ("type1", "2"),
            ("row2", "0"),
            ("col2", "0"),
            ("type2", "2.1"),
            ("row3", "0"),
            ("col3", "1"),
            ("type3", "1"),
            ("xPos", "0"),
            ("yPos", "0"),
            ("orientation", "e"),
        ],
    },
    GoldenStart {
        game: "CrystalGame",
        seed: 15435,
        command: "processmove",
        params: &[("x", "0"), ("y", "0")],
    },
    GoldenStart {
        game: "GoauldCrystals",
        seed: 1,
        command: "playfieldclick",
        params: &[("x", "2"), ("y", "1")],
    },
    GoldenStart {
        game: "ConverseBasicHumanoid",
        seed: 2,
        command: "playermove",
        params: &[("num", "2"), ("suit", "8")],
    },
];

/// An extension request in the shape `protocol.rs` parses: the command, then
/// an `obj` of type `a` named `param`. A value that parses as a number is
/// sent as `t='n'`, anything else as `t='s'`.
fn command_frame(command: &str, params: &[(&str, &str)]) -> String {
    let vars: String = params
        .iter()
        .map(|(name, value)| {
            let kind = if value.parse::<f64>().is_ok() {
                "n"
            } else {
                "s"
            };
            format!("<var n='{name}' t='{kind}'>{value}</var>")
        })
        .collect();
    format!(
        "<msg t='xt'><body action='xtReq'>\
         <![CDATA[<dataObj><var n='cmd' t='s'>{command}</var>\
         <obj o='param' t='a'>{vars}</obj></dataObj>]]></body></msg>"
    )
}

/// The client's half of the handshake: the version check, then a login whose
/// zone names the game, as the SWF sends it.
async fn login(client: &mut TcpStream, game: &str, entity_id: u32, ticket: &str) {
    send_null_terminated(
        client,
        "<msg t='sys'><body action='verChk' r='0'><ver v='154'/></body></msg>",
    )
    .await
    .expect("verChk send");
    send_null_terminated(
        client,
        &format!(
            "<msg t='sys'><body action='login' r='0'><login z='{game}'>\
             <nick><![CDATA[{entity_id}]]></nick><pword><![CDATA[{ticket}]]></pword>\
             </login></body></msg>"
        ),
    )
    .await
    .expect("login send");
}

/// Read frames until `onGameBegin` and return everything read, so the caller
/// can check that `joinOK` came first. Panics on timeout, EOF or I/O error.
async fn read_join_and_game_begin(client: &mut TcpStream) -> String {
    const MILESTONE: &str = "<var n='_cmd' t='s'>onGameBegin</var>";
    let mut seen = String::new();
    let mut chunk = vec![0u8; MAX_MESSAGE_LEN];
    while !seen.contains(MILESTONE) {
        let n = tokio::time::timeout(
            Duration::from_secs(5),
            tokio::io::AsyncReadExt::read(client, &mut chunk),
        )
        .await
        .unwrap_or_else(|_| panic!("no onGameBegin; frames so far:\n{seen}"))
        .expect("read during the join");
        assert!(n > 0, "server closed before onGameBegin: {seen}");
        seen.push_str(&String::from_utf8_lossy(&chunk[..n]).replace('\0', "\n"));
    }
    seen
}

/// Log one game in, make its first golden move, send the client's victory,
/// and check that no result was sent.
async fn refuses_client_victory(golden: &GoldenStart) {
    let registry = SessionRegistry::new();
    let ticket = registry
        .register(
            4400,
            7,
            golden.game.into(),
            1,
            1,
            golden.seed,
            0,
            0,
            1,
            vec![99],
            vec![4243],
            None,
            Some(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
        )
        .await
        .unwrap_or_else(|| panic!("{}: fresh registry must accept the session", golden.game));

    let (mut client, mut rx, handle) = connect_to(registry, ListenerLimits::default()).await;
    login(&mut client, golden.game, 4400, &ticket).await;
    let joined = read_join_and_game_begin(&mut client).await;
    assert!(
        joined.contains("action='joinOK'"),
        "{}: the join must send joinOK before onGameBegin: {joined}",
        golden.game,
    );

    send_null_terminated(&mut client, &command_frame(golden.command, golden.params))
        .await
        .unwrap_or_else(|()| panic!("{}: first move send failed", golden.game));
    send_null_terminated(&mut client, CLIENT_VICTORY)
        .await
        .unwrap_or_else(|()| panic!("{}: victory send failed", golden.game));
    // Frames are read in order, so the server has handled the victory before it
    // sees this EOF and ends the session.
    drop(client);
    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .unwrap_or_else(|_| {
            panic!(
                "{}: session did not end after the client closed",
                golden.game
            )
        })
        .expect("connection task must not panic");

    // The only result allowed is the canceled one that the socket close
    // itself produces (the abandon path). A victory would report its own
    // code and its victory chains, and fails here.
    let results = drain_results(&mut rx);
    assert!(
        results
            .iter()
            .all(|(code, chains)| *code == RESULT_CANCELED && chains.is_empty()),
        "{}: a client victory must be refused; only the canceled result of the close is allowed, got {results:?}",
        golden.game,
    );
}

/// The table covers exactly the games whose `validation()` is `server`, in
/// registry order, and each of them refuses the client's victory.
#[tokio::test]
async fn every_server_validated_game_refuses_a_client_victory() {
    let server: Vec<&str> = GameKind::ALL
        .into_iter()
        .filter(|kind| kind.validation() == Validation::Server)
        .map(GameKind::name)
        .collect();
    let golden: Vec<&str> = GOLDEN_FIRST_MOVES.iter().map(|row| row.game).collect();
    assert_eq!(
        golden, server,
        "the golden table must list every server-validated game",
    );

    for row in &GOLDEN_FIRST_MOVES {
        refuses_client_victory(row).await;
    }
}
