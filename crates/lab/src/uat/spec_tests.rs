//! Spec parsing and validation tests: the schema, the per-source rules
//! and the two-player constraints.

use super::*;

const MINI: &str = r#"
schema = 1
[section]
id = "gm-parity"
system = "GM console command parity"
guide = "unified-uat.md#gm-console-command-parity"
ledger = "legacy-command-parity/README.md"

[[row]]
id = "M1-1"
title = "help answers"
expected = "Each answers in chat."
step = [{ chat = ".help", label = "help" }]

[[row.expect]]
id = "help"
text = ".help lists commands"
source = "chat"
since = "help"
contains = "help"
"#;

#[test]
fn a_minimal_section_parses_with_defaults() {
    let s = parse(MINI).unwrap();
    let row = &s.rows[0];
    assert_eq!(row.required_native, Tier::N1);
    assert_eq!(row.state, "in_world");
    assert_eq!(row.players, 1);
    assert_eq!(s.section.character, "lab");
    assert_eq!(row.steps[0].kind().unwrap(), ActionKind::Chat);
}

#[test]
fn a_dangling_label_and_a_bare_clause_are_rejected() {
    let bad = MINI.replace("since = \"help\"", "since = \"nope\"");
    let e = parse(&bad).unwrap_err();
    assert!(e.contains("M1-1/help: no action labelled \"nope\""), "{e}");
    let bad = MINI.replace("contains = \"help\"", "");
    assert!(parse(&bad).unwrap_err().contains("contains or matches"));
}

#[test]
fn an_action_with_two_kinds_is_rejected() {
    let bad = MINI.replace(
        "{ chat = \".help\", label = \"help\" }",
        "{ chat = \".help\", wait_ms = 5, label = \"help\" }",
    );
    assert!(parse(&bad).unwrap_err().contains("more than one"));
}

#[test]
fn unknown_fields_are_errors_not_silently_ignored() {
    let bad = MINI.replace("title = \"help answers\"", "title = \"x\"\ntypo = 1");
    assert!(parse(&bad).is_err());
}

const PACKET: &str = r#"
[[row.expect]]
id = "timer"
text = "a 15 s timer"
source = "packet"
message = "onTimerUpdate"
direction = "to_client"
entity = "${player_entity_id}"
field = "complete_in_s"
op = "approx"
value = 15
tolerance = 1
"#;

#[test]
fn the_second_player_needs_a_two_player_row() {
    let two = MINI.replace(
        "title = \"help answers\"",
        "title = \"help answers\"\nplayers = 2",
    );
    let p2_step = "{ chat = \".help\", label = \"help\", client = \"p2\" }";
    let ok = two.replace("{ chat = \".help\", label = \"help\" }", p2_step);
    parse(&ok).unwrap();
    // The same step on a one-player row, an unknown client, and
    // @target_player without a second player are all spec errors.
    let one = MINI.replace("{ chat = \".help\", label = \"help\" }", p2_step);
    assert!(parse(&one).unwrap_err().contains("players = 2"));
    let p3 = ok.replace("client = \"p2\"", "client = \"p3\"");
    assert!(parse(&p3).unwrap_err().contains("p1 or p2"));
    let target = MINI.replace(
        "step = [{ chat = \".help\", label = \"help\" }]",
        "step = [{ tool = \"@target_player\" }, { chat = \".help\", label = \"help\" }]",
    );
    assert!(parse(&target).unwrap_err().contains("players = 2"));
    let resolved =
        parse(&target.replace("title = \"help answers\"", "title = \"x\"\nplayers = 2")).unwrap();
    assert_eq!(
        resolved.rows[0].steps[0].tool.as_deref(),
        Some("uat_target_player")
    );
    // A fallback runs on its action's client: it may not name another
    // one, an unknown one, or p2 on a one-player row, at any depth.
    let fb = |client: &str| {
        ok.replace(
            p2_step,
            &format!(
                "{{ tool = \"lab_x\", tier = \"N1\", label = \"help\", fallback = [{{ chat = \".help\", fallback = [{{ chat = \".h\", client = \"{client}\" }}] }}] }}"
            ),
        )
    };
    parse(&fb("p1")).unwrap();
    assert!(parse(&fb("p2"))
        .unwrap_err()
        .contains("runs on its action's client"));
    assert!(parse(&fb("p3")).unwrap_err().contains("p1 or p2"));
    let one = fb("p2").replace("players = 2", "players = 1");
    assert!(parse(&one).unwrap_err().contains("players = 2"));
    // Only clauses that read a client take one.
    let signoz = format!(
        "{two}\n[[row.expect]]\nid = \"s\"\ntext = \"t\"\nsource = \"signoz\"\nfilter = \"x\"\nclient = \"p2\"\n"
    );
    assert!(parse(&signoz).unwrap_err().contains("client applies to"));
}

#[test]
fn a_packet_clause_parses_and_its_rules_hold() {
    let s = parse(&format!("{MINI}{PACKET}")).unwrap();
    let c = &s.rows[0].expect[1];
    assert_eq!(c.source, Source::Packet);
    assert_eq!(c.message.as_deref(), Some("onTimerUpdate"));
    assert_eq!(c.op, Some(Op::Approx));
    assert_eq!(c.tolerance, Some(1.0));

    let bad = format!("{MINI}{}", PACKET.replace("to_client", "outbound"));
    assert!(parse(&bad).unwrap_err().contains("to_client"));
    let bad = format!(
        "{MINI}{}",
        PACKET.replace("message = \"onTimerUpdate\"\n", "")
    );
    assert!(parse(&bad).unwrap_err().contains("needs message"));
    // One tap per row, read at teardown: a step-anchored clause is wrong.
    let bad = format!(
        "{MINI}{}",
        PACKET.replace("source = \"packet\"", "source = \"packet\"\nat = \"help\"")
    );
    assert!(parse(&bad).unwrap_err().contains("no at or since"));
    let bad = format!("{MINI}{}", PACKET.replace("tolerance = 1\n", ""));
    assert!(parse(&bad).unwrap_err().contains("tolerance"));
    // Non-finite and negative tolerances would pass anything or nothing.
    for t in ["inf", "+inf", "nan", "-1"] {
        let bad = format!(
            "{MINI}{}",
            PACKET.replace("tolerance = 1", &format!("tolerance = {t}"))
        );
        assert!(parse(&bad).unwrap_err().contains("finite tolerance"), "{t}");
    }
    // op/value without field would pass on any matching message.
    let bad = format!(
        "{MINI}{}",
        PACKET.replace("field = \"complete_in_s\"\n", "")
    );
    assert!(parse(&bad).unwrap_err().contains("need a field"));
}

const EVENT: &str = r#"
[[row.expect]]
id = "sent"
text = "the press leaves the client"
source = "client_event"
event = "client.ability.sent"
match_fields = { ability_id = 597 }
since = "help"
field = "target_id"
op = "eq"
value = 0
"#;

#[test]
fn a_client_event_clause_parses_and_its_rules_hold() {
    let s = parse(&format!("{MINI}{EVENT}")).unwrap();
    let c = &s.rows[0].expect[1];
    assert_eq!(c.source, Source::ClientEvent);
    assert_eq!(c.match_fields.as_ref().unwrap()["ability_id"], 597);
    // The target is the telemetry name, `client.` and all.
    let bad = format!(
        "{MINI}{}",
        EVENT.replace("client.ability.sent", "ability.sent")
    );
    assert!(parse(&bad).unwrap_err().contains("client.<kind>"));
    let bad = format!("{MINI}{}", EVENT.replace("field = \"target_id\"\n", ""));
    assert!(parse(&bad).unwrap_err().contains("need a field"));
    // event and match_fields belong to client_event clauses only.
    let bad = format!(
        "{MINI}{}",
        EVENT
            .replace(
                "source = \"client_event\"",
                "source = \"chat\"\ncontains = \"x\""
            )
            .replace("field = \"target_id\"\nop = \"eq\"\nvalue = 0\n", "")
    );
    assert!(parse(&bad).unwrap_err().contains("belongs to client_event"));
}

#[test]
fn the_ability_lab_capabilities_resolve_and_check_their_args() {
    let row = MINI.replace(
        "step = [{ chat = \".help\", label = \"help\" }]",
        "setup = [{ tool = \"@dummy\", args = { disposition = \"friendly\" } }]\nstep = [{ chat = \".help\", label = \"help\" }]\nteardown = [{ tool = \"@clear_effects\" }, { tool = \"@cooldowns_reset\", args = { ability_id = 597 } }]",
    );
    let s = parse(&row).unwrap();
    assert_eq!(s.rows[0].setup[0].tool.as_deref(), Some("uat_dummy"));
    assert_eq!(
        s.rows[0].teardown[1].tool.as_deref(),
        Some("uat_cooldowns_reset")
    );
    let bad = row.replace("\"friendly\"", "\"angry\"");
    assert!(parse(&bad).unwrap_err().contains("disposition"));
}
