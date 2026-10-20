//! Every server-validated game, end to end over a real loopback SFS session
//! (MG-71). Each game logs in with its ticket, reads `joinOK` and
//! `onGameBegin`, makes its first move, and waits for the reply that move
//! gets. Then the client sends its own `victory`. The server refuses that
//! victory, so the session reports no victory or defeat; only the canceled
//! result that the socket close produces may follow. The coverage check ties
//! the table to [`GameKind`]: a new server-validated game without a row fails
//! here.

use crate::minigame::games::{GameKind, Validation};

use super::listener_tests::connect_to;
use super::result_dispatch::RESULT_CANCELED;
use super::tests::drain_redeemed;
use super::*;

/// The client's own victory command, sent after the first move.
const CLIENT_VICTORY: &str = "<msg t='xt'><body action='xtReq'>\
     <![CDATA[<dataObj><var n='cmd' t='s'>victory</var></dataObj>]]>\
     </body></msg>";

/// The first move of one server-validated game: the seed its script starts
/// from, the command and parameters of the first client message, and the
/// `_cmd` of the first reply that message gets. The command is the first `>`
/// line of the golden script, with two exceptions. Livewire's script has no
/// `>` line (only `start` and `abort`), so its row is `opendoor`, the first
/// client message the SWF sends after `start`. Alignment's board is random in
/// a live session (the OS-entropy path, never the golden seed), so a
/// `swappieces` cannot be planned; its row is `callhint`, whose reply
/// (`timerupdate`) does not depend on the board.
struct GoldenStart {
    game: &'static str,
    seed: u32,
    command: &'static str,
    params: &'static [(&'static str, &'static str)],
    reply: &'static str,
}

/// One row per server-validated game, in [`GameKind::ALL`] order.
const GOLDEN_FIRST_MOVES: [GoldenStart; 5] = [
    GoldenStart {
        game: "Livewire",
        seed: 7,
        command: "opendoor",
        params: &[],
        reply: "opendoor",
    },
    GoldenStart {
        game: "Alignment",
        seed: 7,
        command: "callhint",
        params: &[],
        reply: "timerupdate",
    },
    GoldenStart {
        game: "CrystalGame",
        seed: 15435,
        command: "processmove",
        params: &[("x", "0"), ("y", "0")],
        reply: "timerupdate",
    },
    GoldenStart {
        game: "GoauldCrystals",
        seed: 1,
        command: "playfieldclick",
        params: &[("x", "2"), ("y", "1")],
        reply: "timerupdate",
    },
    GoldenStart {
        game: "ConverseBasicHumanoid",
        seed: 2,
        command: "playermove",
        params: &[("num", "2"), ("suit", "8")],
        reply: "updatecards",
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

/// Read frames until one carries `_cmd` = `reply` and return everything read,
/// so the caller can check the frames before it. Panics on timeout, EOF or
/// I/O error.
async fn read_until(client: &mut TcpStream, reply: &str) -> String {
    let milestone = format!("<var n='_cmd' t='s'>{reply}</var>");
    let mut seen = String::new();
    let mut chunk = vec![0u8; MAX_MESSAGE_LEN];
    while !seen.contains(&milestone) {
        let n = tokio::time::timeout(
            Duration::from_secs(5),
            tokio::io::AsyncReadExt::read(client, &mut chunk),
        )
        .await
        .unwrap_or_else(|_| panic!("no {reply}; frames so far:\n{seen}"))
        .expect("read while waiting for a reply");
        assert!(n > 0, "server closed before {reply}: {seen}");
        seen.push_str(&String::from_utf8_lossy(&chunk[..n]).replace('\0', "\n"));
    }
    seen
}

/// Log one game in, make its first move and wait for its reply, send the
/// client's victory, and check that no result was sent.
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
            1,
        )
        .await
        .unwrap_or_else(|| panic!("{}: fresh registry must accept the session", golden.game));

    let (mut client, mut rx, handle) =
        connect_to(registry.clone(), ListenerLimits::default()).await;
    login(&mut client, golden.game, 4400, &ticket).await;
    let joined = read_until(&mut client, "onGameBegin").await;
    assert!(
        joined.contains("action='joinOK'"),
        "{}: the join must send joinOK before onGameBegin: {joined}",
        golden.game,
    );

    send_null_terminated(&mut client, &command_frame(golden.command, golden.params))
        .await
        .unwrap_or_else(|()| panic!("{}: first move send failed", golden.game));
    // The move must be answered before the victory goes out, so a move the
    // server silently drops fails here rather than passing the test.
    read_until(&mut client, golden.reply).await;
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
    let results = drain_redeemed(&mut rx, &registry).await;
    assert!(
        results
            .iter()
            .all(|(code, chains, _)| *code == RESULT_CANCELED && chains.is_empty()),
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
