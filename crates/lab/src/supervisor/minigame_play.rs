//! Play a SmartFox minigame through the client's native mouse input.

use std::time::Duration;

use serde_json::{json, Value};
use tokio::time::Instant;

use super::input::{parse_widget_rect, widget_rect_chunk, Rect};
use super::livewire_atlas::LivewireAtlas;
use super::livewire_solver;
use super::screenshot::{self, Region};
use super::Supervisor;
use crate::uat::invoke::ServerTools;

const MOVIE_WINDOW: &str = "Minigame_Movie_Area";
const LIVEWIRE_STAGE: (f64, f64) = (1280.0, 960.0);
const PLACEHOLDER_STAGE: (f64, f64) = (640.0, 480.0);

/// The result and two movie-area captures. Content effects are checked by the UAT row.
pub struct PlayResult {
    pub report: Value,
    pub before: Option<Vec<u8>>,
    pub after: Option<Vec<u8>>,
}

fn stage_to_ui(rect: Rect, stage: (f64, f64), point: (f64, f64)) -> Result<(i32, i32), String> {
    let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
    if ![rect.left, rect.top, width, height, point.0, point.1]
        .into_iter()
        .all(f64::is_finite)
        || width <= 0.0
        || height <= 0.0
        || point.0 < 0.0
        || point.0 > stage.0
        || point.1 < 0.0
        || point.1 > stage.1
    {
        return Err("invalid movie rect or stage point".into());
    }
    Ok((
        (rect.left + point.0 * width / stage.0).round() as i32,
        (rect.top + point.1 * height / stage.1).round() as i32,
    ))
}

fn movie_region(rect: Rect) -> Option<Region> {
    let x = rect.left.floor() as i64;
    let y = rect.top.floor() as i64;
    let w = rect.right.ceil() as i64 - x;
    let h = rect.bottom.ceil() as i64 - y;
    (x >= 0 && y >= 0 && w > 0 && h > 0).then_some(Region {
        x: x as u32,
        y: y as u32,
        w: w as u32,
        h: h as u32,
    })
}

async fn capture(supervisor: &Supervisor, rect: Rect) -> Option<Vec<u8>> {
    let region = movie_region(rect)?;
    supervisor
        .capture()
        .await
        .ok()
        .and_then(|image| screenshot::crop(&image, region).ok())
        .and_then(|image| screenshot::encode_png(&image).ok())
}

async fn server_read(server: &ServerTools, tool: &str, args: Value) -> Result<Value, String> {
    let outcome = server.call(tool, args).await;
    if outcome.ok {
        Ok(outcome.json)
    } else {
        Err(outcome.error.unwrap_or_else(|| format!("{tool} failed")))
    }
}

async fn board_read(server: &ServerTools, entity_id: u32) -> Result<Value, String> {
    let reply = server_read(
        server,
        "server_minigame_state",
        json!({"entity_id": entity_id}),
    )
    .await?;
    reply
        .get("state")
        .filter(|value| !value.is_null())
        .cloned()
        .ok_or_else(|| "server has no active minigame board for this entity".to_string())
}

async fn ready_board(server: &ServerTools, entity_id: u32) -> Result<Value, String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(board) = board_read(server, entity_id).await {
            if matches!(board["state"].as_str(), Some("ready" | "running")) {
                return Ok(board);
            }
        }
        if Instant::now() >= deadline {
            return Err("minigame board did not reach ready/running".into());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[derive(Debug, Default)]
struct TapCursor {
    since_seq: u64,
    rows: Vec<Value>,
}

impl TapCursor {
    async fn read(&mut self, server: &ServerTools, entity_id: u32) -> Result<Vec<Value>, String> {
        let reply = server_read(
            server,
            "server_minigame_tap_read",
            json!({"entity_id": entity_id, "since_seq": self.since_seq}),
        )
        .await?;
        let tap = reply.get("tap").ok_or("minigame tap missing")?;
        let fresh = tap
            .get("events")
            .and_then(Value::as_array)
            .ok_or("minigame tap has no events")?
            .clone();
        for row in &fresh {
            self.since_seq = self.since_seq.max(row["seq"].as_u64().unwrap_or(0));
            self.rows.push(row.clone());
        }
        Ok(fresh)
    }
}

fn row_is(row: &Value, dir: &str, cmd: &str, wire: Option<&str>) -> bool {
    row["dir"] == dir && row["cmd"] == cmd && wire.is_none_or(|name| row["wire_name"] == name)
}

fn terminal_failure(rows: &[Value]) -> bool {
    rows.iter().any(|row| {
        row_is(row, "out", "failure", None)
            || row["outcome"] == "defeat"
            || row["event"] == "result_defeat"
    })
}

fn assessed_outcome(
    action_ok: bool,
    smartfox_victory: bool,
    client_hidden: bool,
    server_outcome: Option<&str>,
    defeated: bool,
) -> &'static str {
    if action_ok && smartfox_victory && client_hidden && server_outcome == Some("victory") {
        "victory"
    } else if defeated {
        "defeat"
    } else {
        "unverified"
    }
}

fn rollover(rows: &[Value], intended: &str) -> bool {
    let last = rows.iter().rev().find(|row| {
        row_is(row, "in", "processover", None) || row_is(row, "in", "processout", None)
    });
    last.is_some_and(|row| row_is(row, "in", "processover", Some(intended)))
}

fn cut_confirmed(rows: &[Value], wire: &str) -> bool {
    let Some(move_at) = rows
        .iter()
        .position(|row| row_is(row, "in", "processmove", Some(wire)))
    else {
        return false;
    };
    rows[move_at + 1..]
        .iter()
        .any(|row| row_is(row, "out", "destroy", Some(wire)))
}

async fn await_rows(
    cursor: &mut TapCursor,
    server: &ServerTools,
    entity_id: u32,
    timeout: Duration,
    mut found: impl FnMut(&[Value]) -> bool,
) -> Result<bool, String> {
    let deadline = Instant::now() + timeout;
    let mut fresh = Vec::new();
    loop {
        fresh.extend(cursor.read(server, entity_id).await?);
        if terminal_failure(&fresh) {
            return Err("minigame reported defeat".into());
        }
        if found(&fresh) {
            return Ok(true);
        }
        if Instant::now() >= deadline {
            return Ok(false);
        }
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
}

async fn native_move(supervisor: &Supervisor, at: (i32, i32)) -> Result<(), String> {
    supervisor.place_cursor_native(at.0, at.1).await?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Opening {
    OpenDoor,
    Resume,
}

fn opening(board: &Value) -> Result<Opening, String> {
    match board["state"].as_str() {
        Some("ready") if board["game_started"] == false => Ok(Opening::OpenDoor),
        Some("running")
            if board["game_started"] == true
                && board["time_remaining"]
                    .as_f64()
                    .is_some_and(|time| time > 0.0) =>
        {
            Ok(Opening::Resume)
        }
        _ => Err("Livewire board is neither unopened nor running with time remaining".into()),
    }
}

fn calibration_point(plan: &livewire_solver::ClickPlan) -> Result<(&str, (f64, f64)), String> {
    let click = plan
        .clicks
        .first()
        .ok_or("Livewire plan has no calibration wire")?;
    let point = click
        .candidates
        .first()
        .ok_or("Livewire plan has no exposed calibration point")?;
    if !(445..=1270).contains(&point.x) || !(110..=890).contains(&point.y) {
        return Err("Livewire calibration point is covered by SWF chrome".into());
    }
    Ok((&click.name, (f64::from(point.x), f64::from(point.y))))
}

async fn native_click(supervisor: &Supervisor, at: (i32, i32)) -> Result<(), String> {
    native_move(supervisor, at).await?;
    tokio::time::sleep(Duration::from_millis(50)).await;
    supervisor.post_button(0, "click", at, None).await
}

async fn run_livewire(
    supervisor: &Supervisor,
    server: &ServerTools,
    atlas: &LivewireAtlas,
    entity_id: u32,
    rect: Rect,
    board: &Value,
    tap: &mut TapCursor,
    probes: &mut Vec<Value>,
) -> Result<(), String> {
    let plan = livewire_solver::plan(board, atlas)?;
    let mode = opening(board)?;
    if plan.clicks.is_empty() && mode == Opening::OpenDoor {
        return Err("Livewire plan contains no remaining goals".into());
    }
    if mode == Opening::OpenDoor {
        let start = stage_to_ui(rect, LIVEWIRE_STAGE, (363.4, 665.25))?;
        native_click(supervisor, start).await?;
        if !await_rows(tap, server, entity_id, Duration::from_secs(2), |rows| {
            rows.iter().any(|row| row_is(row, "in", "opendoor", None))
        })
        .await?
        {
            return Err("Open click produced no SmartFox opendoor".into());
        }
        let opened = board_read(server, entity_id).await?;
        if opening(&opened) != Ok(Opening::Resume) {
            return Err("opendoor did not start the Livewire timer".into());
        }
    }
    probes.push(json!({"kind":"opening","mode":format!("{mode:?}")}));
    if plan.clicks.is_empty() {
        return Ok(());
    }
    let (calibration_wire, calibration_stage) = calibration_point(&plan)?;
    let centre = stage_to_ui(rect, LIVEWIRE_STAGE, calibration_stage)?;
    let mut calibrated = false;
    for (dx, dy) in [(0, 0), (2, 0), (-2, 0), (0, 2), (0, -2)] {
        let at = (centre.0 + dx, centre.1 + dy);
        native_move(supervisor, at).await?;
        let hit = await_rows(tap, server, entity_id, Duration::from_millis(300), |rows| {
            rollover(rows, calibration_wire)
        })
        .await?;
        probes.push(json!({"kind":"calibration","wire":calibration_wire,"at":at,"rollover":hit}));
        if hit {
            calibrated = true;
            break;
        }
    }
    if !calibrated {
        return Err(format!(
            "calibration: no processover for {calibration_wire}"
        ));
    }
    let reset = stage_to_ui(rect, LIVEWIRE_STAGE, (363.4, 665.25))?;
    native_move(supervisor, reset).await?;
    for click in &plan.clicks {
        let mut matched = false;
        for point in &click.candidates {
            let at = stage_to_ui(rect, LIVEWIRE_STAGE, (point.x as f64, point.y as f64))?;
            native_move(supervisor, at).await?;
            let hit = await_rows(tap, server, entity_id, Duration::from_millis(300), |rows| {
                rollover(rows, &click.name)
            })
            .await?;
            probes.push(json!({"kind":"rollover","wire":click.name,"at":at,"rollover":hit}));
            if !hit {
                continue;
            }
            supervisor.post_button(0, "click", at, None).await?;
            let cut = await_rows(tap, server, entity_id, Duration::from_secs(2), |rows| {
                cut_confirmed(rows, &click.name)
            })
            .await?;
            probes.push(json!({"kind":"cut","wire":click.name,"confirmed":cut}));
            if !cut {
                return Err(format!(
                    "click on {} lacked processmove/destroy",
                    click.name
                ));
            }
            matched = true;
            break;
        }
        if !matched {
            return Err(format!("no matching rollover for {}", click.name));
        }
    }
    Ok(())
}

async fn run_placeholder(
    supervisor: &Supervisor,
    rect: Rect,
    probes: &mut Vec<Value>,
) -> Result<(), String> {
    let at = stage_to_ui(rect, PLACEHOLDER_STAGE, (470.45, 388.95))?;
    native_click(supervisor, at).await?;
    probes.push(json!({"kind":"placeholder_win_btn","at":at}));
    Ok(())
}

/// Drive the visible minigame and report SmartFox evidence plus native input tier.
pub async fn play(
    supervisor: &Supervisor,
    atlas: Option<&LivewireAtlas>,
    entity_id: u32,
    requested_game: Option<&str>,
) -> Result<PlayResult, String> {
    let server = ServerTools::from_env().ok_or("lab server MCP endpoint is not configured")?;
    let visible = supervisor
        .wait_for(
            "MinigameWin ~= nil and MinigameWin:isVisible()",
            Some(10_000),
            Some(100),
        )
        .await?;
    if visible["met"] != true {
        return Err("MinigameWin did not become visible".into());
    }
    let placeholder = requested_game.is_some_and(|game| {
        matches!(
            game,
            "Hack" | "Activate" | "Analyze" | "Bypass" | "Converse"
        )
    });
    let board = if placeholder {
        board_read(&server, entity_id).await.ok()
    } else {
        Some(ready_board(&server, entity_id).await?)
    };
    let game = board
        .as_ref()
        .and_then(|state| state["game_name"].as_str())
        .or(requested_game)
        .ok_or("minigame has no game_name; pass game for a placeholder")?;
    if requested_game.is_some_and(|requested| requested != game) {
        return Err("requested game differs from active server board".into());
    }
    let rect = parse_widget_rect(
        MOVIE_WINDOW,
        &supervisor
            .lua_results(&widget_rect_chunk(MOVIE_WINDOW))
            .await?,
    )?;
    let before = capture(supervisor, rect).await;
    let mut tap = TapCursor::default();
    if board.is_some() {
        tap.read(&server, entity_id).await?;
    }
    let mut probes = Vec::new();
    let action = if game == "Livewire" {
        match atlas {
            Some(atlas) => {
                run_livewire(
                    supervisor,
                    &server,
                    atlas,
                    entity_id,
                    rect,
                    board.as_ref().ok_or("Livewire board missing")?,
                    &mut tap,
                    &mut probes,
                )
                .await
            }
            None => Err("installed Livewire hit atlas unavailable".into()),
        }
    } else if matches!(
        game,
        "Hack" | "Activate" | "Analyze" | "Bypass" | "Converse"
    ) {
        run_placeholder(supervisor, rect, &mut probes).await
    } else {
        Err(format!("minigame {game} has no verified native play flow"))
    };
    let victory = if action.is_ok() && board.is_some() {
        await_rows(
            &mut tap,
            &server,
            entity_id,
            Duration::from_secs(3),
            |rows| rows.iter().any(|row| row_is(row, "out", "victory", None)),
        )
        .await
        .unwrap_or(false)
            || tap
                .rows
                .iter()
                .any(|row| row_is(row, "out", "victory", None))
    } else {
        false
    };
    let hidden = supervisor
        .wait_for(
            "MinigameWin == nil or not MinigameWin:isVisible()",
            Some(3_000),
            Some(100),
        )
        .await
        .ok()
        .is_some_and(|value| value["met"] == true);
    let final_state = board_read(&server, entity_id).await.ok();
    let after = capture(supervisor, rect).await;
    let server_outcome = final_state
        .as_ref()
        .and_then(|state| state["outcome"].as_str());
    let outcome = assessed_outcome(
        action.is_ok(),
        victory,
        hidden,
        server_outcome,
        terminal_failure(&tap.rows),
    );
    let effect_ok = outcome == "victory";
    Ok(PlayResult {
        report: json!({
            "entity_id": entity_id,
            "game": game,
            "outcome": outcome,
            "effect_ok": effect_ok,
            "reason": action.err(),
            "smartfox_victory": victory,
            "server_outcome": server_outcome,
            "client_window_hidden": hidden,
            "movie_rect": [rect.left, rect.top, rect.right, rect.bottom],
            "probes": probes,
            "tap_rows": tap.rows,
            "native_level": {"tier":"N1"},
        }),
        before,
        after,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_mapping_uses_movie_area_scale() {
        let rect = Rect {
            left: 320.0,
            top: 140.0,
            right: 960.0,
            bottom: 620.0,
        };
        assert_eq!(
            stage_to_ui(rect, LIVEWIRE_STAGE, (363.4, 665.25)).unwrap(),
            (502, 473)
        );
        assert_eq!(
            stage_to_ui(rect, LIVEWIRE_STAGE, (1280.0, 960.0)).unwrap(),
            (960, 620)
        );
        assert_eq!(
            stage_to_ui(rect, PLACEHOLDER_STAGE, (470.45, 388.95)).unwrap(),
            (790, 529)
        );
        assert!(stage_to_ui(rect, LIVEWIRE_STAGE, (-1.0, 30.0)).is_err());
    }

    #[test]
    fn running_board_resumes_without_another_open_click() {
        let ready = json!({"state":"ready","game_started":false,"time_remaining":20.0});
        let running = json!({"state":"running","game_started":true,"time_remaining":12.5});
        assert_eq!(opening(&ready), Ok(Opening::OpenDoor));
        assert_eq!(opening(&running), Ok(Opening::Resume));
        assert!(
            opening(&json!({"state":"running","game_started":true,"time_remaining":0.0})).is_err()
        );
        assert!(
            opening(&json!({"state":"ready","game_started":true,"time_remaining":20.0})).is_err()
        );
    }

    #[test]
    fn calibration_uses_an_exposed_plan_point_outside_the_cover() {
        let plan = livewire_solver::ClickPlan {
            status: "planned",
            goal_total: 1,
            clicks: vec![livewire_solver::PlannedClick {
                name: "g1".into(),
                kind: "goal".into(),
                lib: "gGreenBlack1".into(),
                depth: 100,
                candidates: vec![livewire_solver::Candidate {
                    x: 760,
                    y: 410,
                    clearance: 10,
                }],
            }],
            unattached: vec![],
            atlas_sha256: "a".repeat(64),
        };
        assert_eq!(calibration_point(&plan).unwrap(), ("g1", (760.0, 410.0)));
        let mut covered = plan;
        covered.clicks[0].candidates[0].x = 400;
        assert!(calibration_point(&covered).is_err());
        covered.clicks.clear();
        assert!(calibration_point(&covered).is_err());
    }

    #[test]
    fn failed_native_action_cannot_report_a_uat_pass() {
        assert_eq!(
            assessed_outcome(false, true, true, Some("victory"), false),
            "unverified"
        );
        assert_eq!(
            assessed_outcome(true, true, true, Some("victory"), false),
            "victory"
        );
        assert_eq!(
            assessed_outcome(true, true, false, Some("victory"), false),
            "unverified"
        );
    }

    #[test]
    fn rollover_requires_latest_matching_processover() {
        let target = json!({"dir":"in","cmd":"processover","wire_name":"g1"});
        let wrong = json!({"dir":"in","cmd":"processover","wire_name":"g2"});
        let out = json!({"dir":"in","cmd":"processout","wire_name":"g1"});
        assert!(rollover(&[wrong.clone(), target.clone()], "g1"));
        assert!(!rollover(&[target.clone(), wrong], "g1"));
        assert!(!rollover(&[target, out], "g1"));
    }

    #[test]
    fn tap_cut_needs_matching_in_and_out_rows() {
        let input = json!({"dir":"in","cmd":"processmove","wire_name":"g1"});
        let output = json!({"dir":"out","cmd":"destroy","wire_name":"g1"});
        assert!(cut_confirmed(&[input.clone(), output.clone()], "g1"));
        assert!(!cut_confirmed(&[output.clone(), input.clone()], "g1"));
        assert!(!cut_confirmed(&[input, output], "g2"));
    }
}
