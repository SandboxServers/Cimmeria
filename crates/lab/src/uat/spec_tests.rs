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

/// FS-P3 from the lab-spec-vocab contract: one frost pose, aimed at the entity.
const FS_P3: &str = r#"
[[row]]
id = "FS-P3"
title = "frost pose"
expected = "The camera aims at the frost body."
pose = [
  { id = "frost", tag = "ArmYourself_FrostBody", radius_m = 4.16, bearing_deg = 322.5, dy_m = 0.13, pitch_deg = -28.8, yaw_offset_deg = 25.3, zoom = 250.0, calibrated = "2026-10-10 hand" },
]
setup = [
  { tool = "@finish_dialog", args = {} },
  { chat = "/gmsetgodmode 1", tier = "G" },
  { tool = "@stand", args = { pose = "frost" }, label = "stand" },
]
step = [
  { tool = "@camera", args = { pose = "frost" }, label = "aim" },
  { tool = "@world_click", args = { pose = "frost", button = "right", expect = "window" }, label = "frost" },
  { tool = "@finish_dialog", args = {}, label = "close" },
]
[[row.expect]]
id = "step-80623"
text = "sgw_mission: 622 on step 80623"
source = "server"
tool = "server_db_query"
args = { sql = "SELECT ..." }
pointer = "/rows/0/current_step_id"
op = "eq"
value = 80623
timeout_ms = 5000
"#;

/// The contract's frost pose line (FS-P3).
const FROST_LINE: &str = r#"{ id = "frost", tag = "ArmYourself_FrostBody", radius_m = 4.16, bearing_deg = 322.5, dy_m = 0.13, pitch_deg = -28.8, yaw_offset_deg = 25.3, zoom = 250.0, calibrated = "2026-10-10 hand" }"#;

/// The contract's point-aimed guard pose line (FS-P4).
const GUARD_LINE: &str = r#"{ id = "guard", tag = "ArmYourself_GuardBody", radius_m = 4.41, bearing_deg = 322.7, dy_m = 0.13, pitch_deg = -28.8, yaw_offset_deg = 25.3, zoom = 250.0, aim = "point", aim_dy_m = 0.28 }"#;

/// The one pose a one-row section with `pose = [line]` declares.
fn one_pose(line: &str) -> PoseSpec {
    let text = format!(
        "{MINI}\n[[row]]\nid = \"P\"\ntitle = \"t\"\nexpected = \"e\"\npose = [{line}]\n\
         step = [{{ chat = \".help\", label = \"help\" }}]\n\
         [[row.expect]]\nid = \"help\"\ntext = \"t\"\nsource = \"chat\"\nsince = \"help\"\ncontains = \"help\"\n"
    );
    parse(&text).unwrap().rows.remove(1).pose.remove(0)
}

#[test]
fn a_pose_row_parses_and_its_stand_point_follows_the_bearing() {
    let s = parse(&format!("{MINI}{FS_P3}")).unwrap();
    let pose = &s.rows[1].pose[0];
    assert_eq!(pose.id, "frost");
    assert_eq!(pose.aim, AimMode::Entity);
    // README F20: the stand point is 4.16 m out at bearing 322.5 from the
    // target, which puts it at x = -325.0 and z = -212.8.
    let stand = pose.stand_point([-328.30, 73.472, -210.27]);
    assert!((stand[0] + 325.0).abs() < 0.02, "{stand:?}");
    assert!((stand[1] - 73.6).abs() < 0.02, "{stand:?}");
    assert!((stand[2] + 212.8).abs() < 0.02, "{stand:?}");
    // Only a point-aimed pose has an aim point, `aim_dy_m` above the target.
    assert_eq!(pose.aim_point([1.0, 2.0, 3.0]), None);
    let aim = one_pose(GUARD_LINE).aim_point([1.0, 2.0, 3.0]).unwrap();
    assert!(
        (aim[0] - 1.0).abs() < 1e-9 && (aim[1] - 2.28).abs() < 1e-9 && (aim[2] - 3.0).abs() < 1e-9,
        "{aim:?}"
    );
}

#[test]
fn pose_lines_round_trip() {
    for line in [FROST_LINE, GUARD_LINE] {
        let pose = one_pose(line);
        // The line written back is the line read, and it parses to the same pose.
        assert_eq!(pose.to_inline_toml(), line);
        assert_eq!(one_pose(&pose.to_inline_toml()), pose);
    }
    // The frost pose's line is the contract's, byte for byte, through the section parser.
    let s = parse(&format!("{MINI}{FS_P3}")).unwrap();
    assert_eq!(s.rows[1].pose[0].to_inline_toml(), FROST_LINE);
}

#[test]
fn pose_rules_name_the_row() {
    let bad = r#"
[[row]]
id = "FS-X"
title = "bad poses"
expected = "Refused."
pose = [
  { id = "dup", tag = "A", radius_m = 4.0, bearing_deg = 10.0, dy_m = 0.1 },
  { id = "dup", tag = "B", radius_m = 4.0, bearing_deg = 10.0, dy_m = 0.1 },
  { id = "near", tag = "C", radius_m = 0.5, bearing_deg = 10.0, dy_m = 0.1 },
  { id = "wide", tag = "D", radius_m = 4.0, bearing_deg = 360, dy_m = 0.1 },
  { id = "pt", tag = "E", radius_m = 4.0, bearing_deg = 10.0, dy_m = 0.1, aim = "point" },
]
step = [
  { tool = "@camera", args = { pose = "nope" }, label = "aim" },
  { tool = "@world_click", args = { tag = "X", name = "Y" }, label = "click" },
  { tool = "client_world_click", args = { face_tag = "Z" }, label = "face" },
]
[[row.expect]]
id = "c"
text = "t"
source = "chat"
contains = "x"
"#;
    let err = parse(&format!("{MINI}{bad}")).unwrap_err();
    for want in [
        "FS-X: duplicate pose id \"dup\"",
        "radius_m 0.5 is outside 1.0..=8.0",
        "bearing_deg 360 is outside 0.0..360.0",
        "aim = \"point\" needs aim_dy_m",
        "pose \"nope\" is not a pose of this row",
        "tag with name: give one",
        "face_tag applies to client_camera only",
    ] {
        assert!(err.contains(want), "missing {want:?} in {err}");
    }
}

/// A one-row section whose step list is the TOML array `steps`.
fn with_steps(steps: &str) -> String {
    format!(
        "{MINI}\n[[row]]\nid = \"W1\"\ntitle = \"wait\"\nexpected = \"Waits.\"\nstep = {steps}\n[[row.expect]]\nid = \"c\"\ntext = \"t\"\nsource = \"chat\"\ncontains = \"x\"\n"
    )
}

#[test]
fn wait_server_is_a_fifth_action_kind() {
    let s = parse(&with_steps(r#"[{ wait_server = { log = "x" } }]"#)).unwrap();
    let step = &s.rows[1].steps[0];
    assert_eq!(step.kind().unwrap(), ActionKind::ServerWait);
    assert_eq!(step.wait_server.as_ref().unwrap().timeout_ms, 30000);

    let bad = with_steps(r#"[{ wait_server = { log = "x" }, wait_ms = 5 }]"#);
    assert!(parse(&bad).unwrap_err().contains("more than one"));
    let bad = with_steps(r#"[{ wait_server = { log = "x", timeout_ms = 0 } }]"#);
    assert!(parse(&bad).unwrap_err().contains("outside 1..=120000"));
}

/// A `server` clause with the given `timeout_ms`.
fn server_clause(timeout_ms: u64) -> String {
    format!(
        "{MINI}\n[[row]]\nid = \"S1\"\ntitle = \"server\"\nexpected = \"Reads the server.\"\nstep = [{{ chat = \".help\" }}]\n[[row.expect]]\nid = \"db\"\ntext = \"a row reaches step 80623\"\nsource = \"server\"\ntool = \"server_db_query\"\nargs = {{ sql = \"SELECT 1\" }}\npointer = \"/rows/0/current_step_id\"\nop = \"eq\"\nvalue = 80623\ntimeout_ms = {timeout_ms}\n"
    )
}

#[test]
fn a_server_clause_timeout_is_capped() {
    assert!(parse(&server_clause(60001))
        .unwrap_err()
        .contains("at most 60000"));
    assert!(parse(&server_clause(60000)).is_ok());
}
