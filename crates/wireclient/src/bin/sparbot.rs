//! `sparbot`: log a second account into the world as a duel partner that
//! accepts every challenge, stands still, and forfeits after a while
//! (SS-U2). See `docs/architecture/wireclient.md` § "sparbot".
//!
//! ```text
//! SPARBOT_USER=spar SPARBOT_PASSWORD=... \
//!   sparbot --player-id 1234 [--auth-url http://127.0.0.1:8081] [--shard Test]
//!           [--forfeit-after 30] [--run-for 0]
//! ```
//!
//! Every option also reads an environment variable (`SPARBOT_AUTH_URL`,
//! `SPARBOT_USER`, `SPARBOT_PASSWORD`, `SPARBOT_SHARD`, `SPARBOT_PLAYER_ID`,
//! `SPARBOT_FORFEIT_AFTER`, `SPARBOT_RUN_FOR`); a flag wins. Credentials are
//! never built in. `--forfeit-after 0` never forfeits; `--run-for 0` runs
//! until Ctrl-C.

use std::process::ExitCode;
use std::time::Duration;

use sha1::{Digest, Sha1};

use cimmeria_wireclient::auth::Credentials;
use cimmeria_wireclient::sparbot::{self, Sparbot, SparbotConfig};
use cimmeria_wireclient::GameSession;

const USAGE: &str = "usage: sparbot --player-id <id> [--user <name>] [--password <pw>] \
[--auth-url <url>] [--shard <name>] [--forfeit-after <secs, 0 = never>] [--run-for <secs, 0 = until Ctrl-C>]
Every option can come from SPARBOT_<OPTION> instead (e.g. SPARBOT_PASSWORD).";

#[derive(Debug)]
struct Args {
    auth_url: String,
    user: String,
    password: String,
    shard: String,
    player_id: i32,
    forfeit_after: Option<Duration>,
    run_for: Option<Duration>,
}

/// A flag's value, else the `SPARBOT_<NAME>` environment variable.
fn option(argv: &[String], flag: &str) -> Option<String> {
    let from_flag = argv
        .iter()
        .position(|a| a == flag)
        .and_then(|i| argv.get(i + 1).cloned());
    from_flag.or_else(|| {
        let var = format!(
            "SPARBOT_{}",
            flag.trim_start_matches("--")
                .replace('-', "_")
                .to_uppercase()
        );
        std::env::var(var).ok().filter(|v| !v.is_empty())
    })
}

/// Seconds, where 0 means "no limit".
fn secs(argv: &[String], flag: &str, default: u64) -> Result<Option<Duration>, String> {
    let n = match option(argv, flag) {
        Some(v) => v
            .parse::<u64>()
            .map_err(|_| format!("{flag}: not a whole number of seconds: {v}"))?,
        None => default,
    };
    Ok((n > 0).then(|| Duration::from_secs(n)))
}

fn parse_args(argv: &[String]) -> Result<Args, String> {
    let required = |flag: &str| option(argv, flag).ok_or_else(|| format!("{flag} is required"));
    let player_id = required("--player-id")?;
    Ok(Args {
        auth_url: option(argv, "--auth-url").unwrap_or_else(|| "http://127.0.0.1:8081".into()),
        user: required("--user")?,
        password: required("--password")?,
        shard: option(argv, "--shard").unwrap_or_else(|| "Test".into()),
        player_id: player_id
            .parse()
            .map_err(|_| format!("--player-id: not a number: {player_id}"))?,
        forfeit_after: secs(argv, "--forfeit-after", 30)?,
        run_for: secs(argv, "--run-for", 0)?,
    })
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "sparbot=info,warn".to_string()),
        )
        .init();

    let argv: Vec<String> = std::env::args().skip(1).collect();
    if argv.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let args = match parse_args(&argv) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("sparbot: {e}\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    let creds = Credentials {
        username: args.user.clone(),
        password_sha1_hex: hex::encode_upper(Sha1::digest(args.password.as_bytes())),
        ..Credentials::test_account()
    };
    let mut session = match GameSession::connect(&args.auth_url, &creds, &args.shard, 1).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("sparbot: login as {} failed: {e}", args.user);
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = session
        .enter_world(args.player_id, Duration::from_secs(15))
        .await
    {
        eprintln!(
            "sparbot: world entry as player {} failed: {e}",
            args.player_id
        );
        return ExitCode::FAILURE;
    }
    let own = session
        .player_entity_id
        .expect("enter_world sets the entity id");
    tracing::info!(
        target: "sparbot",
        event = "sparbot.in_world",
        account_id = session.account_id,
        player_id = args.player_id,
        entity_id = own,
        forfeit_after_secs = args.forfeit_after.map(|d| d.as_secs()),
        "in the world; waiting for duel challenges (Ctrl-C to log out)"
    );

    let mut bot = Sparbot::new(
        own,
        SparbotConfig {
            forfeit_after: args.forfeit_after,
        },
    );
    tokio::select! {
        () = sparbot::run(&session, &mut bot, args.run_for) => {}
        _ = tokio::signal::ctrl_c() => {
            tracing::info!(target: "sparbot", event = "sparbot.interrupted", "Ctrl-C");
        }
    }
    // Log out cleanly so the character does not linger for the 60 s reap.
    let _ = session.send_bundle(&GameSession::disconnect(0), true).await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    let r = &bot.report;
    println!(
        "sparbot: accepted {} challenge(s), sent {} forfeit(s), stopped: {:?}",
        r.challenges_accepted,
        r.forfeits_sent,
        r.stopped
            .map_or("interrupted".to_string(), |s| format!("{s:?}"))
    );
    match r.stopped {
        Some(sparbot::StopReason::ServerSilent | sparbot::StopReason::SendFailed) => {
            ExitCode::FAILURE
        }
        _ => ExitCode::SUCCESS,
    }
}
