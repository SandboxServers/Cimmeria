//! `lab_ensure_in_world`: from wherever the client is (not running, at the
//! movies, login, server select, character select, or in the world as
//! someone else) to in the world as the requested character, with virtual
//! focus on. Idempotent: already there, it returns after one observation.
//!
//! The loop observes, asks the pure [`plan`] what to do next, does it with
//! the existing flows (`lab_client_start`, `lab_login`, `lab_logout`,
//! `lab_play_character`, `lab_finish_dialog`, `lab_create_character`), and
//! observes again. The one new step is [`Next::WaitReady`]: a fresh client
//! has no window for about 30 s, and `lab_login` used to fail at its first
//! step (virtual focus needs the window) when called too soon.

use std::time::{Duration, Instant};

use serde_json::{json, Map, Value};

use crate::supervisor::flows::characters::CreateRequest;
use crate::supervisor::flows::login::{
    classify_start, start_screen_chunk, LoginRequest, StartScreen,
};
use crate::supervisor::flows::widgets::{self, DIALOG_WIN};
use crate::supervisor::flows::world::{DEFAULT_MAX_PAGES, DEFAULT_PLAY_TIMEOUT};
use crate::supervisor::{process, Supervisor};

/// Window and bridge must answer within this after a launch.
pub const READY_TIMEOUT: Duration = Duration::from_secs(120);
/// Observe-act rounds before giving up (each round is one flow).
pub const MAX_ROUNDS: usize = 10;
/// One-second waits for the player's name to read after zone-in, before
/// the run gives up (they do not count as rounds).
pub const MAX_SETTLES: usize = 15;

/// What the client shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// Movies, loading, or nothing recognisable.
    Other,
    /// EULA, login or server select: `lab_login` takes it from here.
    Startup,
    CharSelect,
    World,
}

/// One look at the client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    /// The supervised process is alive.
    pub running: bool,
    /// Its main window exists.
    pub window: bool,
    /// The bridge answers (heartbeat).
    pub bridge: bool,
    pub screen: Screen,
    /// The player's name, in the world.
    pub player: Option<String>,
    pub dialog: bool,
}

/// What a request asks for, as [`plan`] needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Goal {
    pub character: String,
    /// Create the character when the list does not have it.
    pub can_create: bool,
    /// Where to stop.
    pub stop: StopAt,
}

/// How far `lab_ensure_in_world` goes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum StopAt {
    /// The client is running with its window and bridge up (any screen).
    Running,
    /// Logged in, at character select (logs out of the world if needed).
    CharacterSelect,
    /// In the world as the character.
    #[default]
    World,
}

impl StopAt {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "running" => Ok(Self::Running),
            "character_select" | "char_select" => Ok(Self::CharacterSelect),
            "world" => Ok(Self::World),
            other => Err(format!(
                "stop_at must be running, character_select or world, not {other:?}"
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::CharacterSelect => "character_select",
            Self::World => "world",
        }
    }
}

/// The next action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Next {
    Start,
    WaitReady,
    Login,
    Logout,
    Play,
    FinishDialog,
    /// In the world but the player's name does not read yet (right after
    /// zone-in): wait a second and look again rather than log out.
    Settle,
    Done,
}

/// Decide the next action. `dialog_tried` stops a dialog that will not
/// finish from looping (it is reported instead).
pub fn plan(o: &Observation, goal: &Goal, dialog_tried: bool) -> Next {
    if !o.running {
        return Next::Start;
    }
    if !o.window || !o.bridge {
        return Next::WaitReady;
    }
    match goal.stop {
        StopAt::Running => return Next::Done,
        StopAt::CharacterSelect => {
            return match o.screen {
                Screen::CharSelect => Next::Done,
                Screen::World => Next::Logout,
                Screen::Startup | Screen::Other => Next::Login,
            }
        }
        StopAt::World => {}
    }
    match o.screen {
        Screen::World if o.player.is_none() => Next::Settle,
        Screen::World => {
            let same = o
                .player
                .as_deref()
                .is_some_and(|p| p.eq_ignore_ascii_case(&goal.character));
            if !same {
                Next::Logout
            } else if o.dialog && !dialog_tried {
                Next::FinishDialog
            } else {
                Next::Done
            }
        }
        Screen::CharSelect => Next::Play,
        Screen::Startup | Screen::Other => Next::Login,
    }
}

/// Lua returning the startup screens (4), the world HUD, the dialog, and
/// the player's name.
pub fn observe_chunk() -> String {
    let p = |e: String| format!("tostring(select(2, pcall(function() return {e} end)) == true)");
    format!(
        "{}, {}, {}, (function() local ok, n = pcall(unitName, Unit.Player) \
         if ok and n ~= nil then return tostring(n) end return '' end)()",
        start_screen_chunk(),
        p(widgets::world_up()),
        p(widgets::visible(DIALOG_WIN)),
    )
}

/// Parse [`observe_chunk`]'s results into `(screen, dialog, player)`.
pub fn parse_observe(r: &[String]) -> (Screen, bool, Option<String>) {
    let v = |i: usize| r.get(i).map(String::as_str) == Some("true");
    let screen = if v(4) {
        Screen::World
    } else {
        match classify_start(r) {
            StartScreen::CharSelect => Screen::CharSelect,
            StartScreen::Other => Screen::Other,
            _ => Screen::Startup,
        }
    };
    let player = r
        .get(6)
        .filter(|s| !s.is_empty() && s.as_str() != "nil")
        .cloned();
    (screen, v(5), player.filter(|_| screen == Screen::World))
}

/// `lab_ensure_in_world` arguments.
#[derive(Debug, Clone, Default)]
pub struct EnsureRequest {
    /// Start override (`lab_client_start`'s `server`).
    pub server: Option<String>,
    /// Server row at login (default: `server`, then lab-account.json).
    pub shard: Option<String>,
    /// Character (default: lab-account.json's `character`).
    pub character: Option<String>,
    pub focus: bool,
    /// Create the character with these when it is missing.
    pub create: Option<CreateRequest>,
    /// Where to stop (default: in the world).
    pub stop: StopAt,
}

impl Supervisor {
    /// One observation. Never fails: what cannot be read reads as "not yet".
    pub async fn observe(&self) -> Observation {
        let pid = self.state.lock().await.pid;
        let running = pid.is_some_and(process::is_alive);
        let window = match pid.filter(|_| running) {
            Some(pid) => tokio::task::spawn_blocking(move || process::find_main_window(pid))
                .await
                .ok()
                .flatten()
                .is_some(),
            None => false,
        };
        let bridge = running && self.bridge.heartbeat().await.is_ok();
        let (screen, dialog, player) = if bridge {
            match self.lua_results(&observe_chunk()).await {
                Ok(r) => parse_observe(&r),
                Err(_) => (Screen::Other, false, None),
            }
        } else {
            (Screen::Other, false, None)
        };
        Observation {
            running,
            window,
            bridge,
            screen,
            player,
            dialog,
        }
    }

    /// Wait for the window and a live bridge after a launch.
    async fn wait_ready(&self) -> Result<(), String> {
        let t0 = Instant::now();
        self.wait_main_window(READY_TIMEOUT).await?;
        loop {
            if self.bridge.heartbeat().await.is_ok() {
                return Ok(());
            }
            if t0.elapsed() > READY_TIMEOUT {
                return Err(format!(
                    "the bridge did not answer within {} s of the launch",
                    READY_TIMEOUT.as_secs()
                ));
            }
            self.idle(Duration::from_millis(500)).await?;
        }
    }

    /// Play `character`, creating it first when allowed and missing.
    async fn ensure_play(
        &self,
        character: &str,
        create: Option<&CreateRequest>,
    ) -> Result<Value, String> {
        let played = self.play_flow(character, true, DEFAULT_PLAY_TIMEOUT).await;
        let Err(e) = played else {
            return Ok(Value::Null);
        };
        // `select_character` names this step when the list lacks the name.
        let missing = e.step == "find_character";
        match create {
            Some(req) if missing => {
                // The arguments were checked before the run started, so a
                // character is only deleted for a create that can succeed.
                let freed = self
                    .ensure_slot_flow(1, vec![character.to_string()])
                    .await?;
                self.create_character_flow(req.clone()).await?;
                self.play_flow(character, true, DEFAULT_PLAY_TIMEOUT)
                    .await
                    .map_err(String::from)?;
                Ok(freed["deleted"].clone())
            }
            _ => Err(e.summary()),
        }
    }

    /// `lab_ensure_in_world`.
    pub async fn ensure_in_world(&self, req: EnsureRequest) -> Result<Value, String> {
        let account = self.lab_account();
        let character = req
            .character
            .clone()
            .or_else(|| account.as_ref().map(|a| a.character.clone()))
            .filter(|c| !c.is_empty())
            .ok_or("no character: pass `character` or set it in lab-account.json")?;
        // The character's last name is its list name. Check every create
        // argument before acting: freeing a slot deletes a character, and
        // must not happen for a create the flow would then refuse (review
        // of #1309).
        let create = req.create.clone().map(|mut c| {
            if c.last.is_empty() {
                c.last = character.clone();
            }
            c
        });
        if let Some(c) = &create {
            c.check()
                .map_err(|e| format!("create: {e} (nothing was changed)"))?;
        }
        let goal = Goal {
            character: character.clone(),
            can_create: create.is_some(),
            stop: req.stop,
        };
        let mut steps_ms = Map::new();
        let mut dialog_tried = false;
        let mut already = true;
        let mut deleted = Value::Null;
        let (mut rounds, mut settles) = (0, 0);
        while rounds < MAX_ROUNDS {
            let o = self.observe().await;
            let next = plan(&o, &goal, dialog_tried);
            if next == Next::Settle {
                settles += 1;
                if settles > MAX_SETTLES {
                    return Err(format!(
                        "in the world but the player's name did not read for {MAX_SETTLES} s"
                    ));
                }
                self.idle(Duration::from_secs(1)).await?;
                continue;
            }
            rounds += 1;
            let t0 = Instant::now();
            let step = match next {
                Next::Done if req.stop != StopAt::World => {
                    let mut out = json!({ "at": req.stop.as_str(), "steps_ms": steps_ms });
                    if req.stop == StopAt::CharacterSelect {
                        if let Ok(v) = self.characters_flow().await {
                            let names: Vec<Value> = v["characters"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .map(|c| c["name"].clone())
                                .collect();
                            out["characters"] = json!(names);
                        }
                    }
                    if already {
                        out["already"] = json!(true);
                    }
                    return Ok(out);
                }
                Next::Done => {
                    if req.focus {
                        self.input_focus(true).await?;
                    }
                    let me = self.ui_player_state(false, false).await.unwrap_or_default();
                    let pos = &me["position"];
                    let mut out = json!({
                        "in_world": true,
                        "character": o.player.unwrap_or(character),
                        "world_id": me["world_id"],
                        "pos": [pos["x"], pos["y"], pos["z"]],
                        "steps_ms": steps_ms,
                    });
                    if already {
                        out["already"] = json!(true);
                    }
                    if o.dialog {
                        out["dialog_open"] = json!(true);
                    }
                    if !deleted.is_null() {
                        out["deleted_to_free_a_slot"] = deleted;
                    }
                    return Ok(out);
                }
                Next::Settle => unreachable!("handled before the match"),
                Next::Start => self.start(req.server.clone()).await.map(|_| "start"),
                Next::WaitReady => self.wait_ready().await.map(|_| "ready"),
                Next::Login => {
                    let login = LoginRequest {
                        server: req.shard.clone().or_else(|| req.server.clone()),
                        ..Default::default()
                    };
                    self.login_flow(login)
                        .await
                        .map(|_| "login")
                        .map_err(String::from)
                }
                Next::Logout => self
                    .logout_flow()
                    .await
                    .map(|_| "logout")
                    .map_err(String::from),
                Next::Play => self
                    .ensure_play(&character, create.as_ref())
                    .await
                    .map(|d| {
                        deleted = d;
                        "play"
                    }),
                Next::FinishDialog => {
                    dialog_tried = true;
                    self.finish_dialog_flow(false, DEFAULT_MAX_PAGES)
                        .await
                        .map(|_| "dialog")
                        .map_err(String::from)
                }
            };
            let name = step.map_err(|e| format!("{next:?}: {e}"))?;
            already = false;
            let ms = t0.elapsed().as_millis() as u64;
            let prev = steps_ms.get(name).and_then(Value::as_u64).unwrap_or(0);
            steps_ms.insert(name.into(), json!(prev + ms));
        }
        Err(format!(
            "not in the world as {character} after {MAX_ROUNDS} rounds (steps {})",
            Value::Object(steps_ms)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(screen: Screen, player: Option<&str>, dialog: bool) -> Observation {
        Observation {
            running: true,
            window: true,
            bridge: true,
            screen,
            player: player.map(str::to_string),
            dialog,
        }
    }

    fn goal() -> Goal {
        Goal {
            character: "Labone".into(),
            can_create: false,
            stop: StopAt::World,
        }
    }

    #[test]
    fn a_stopped_client_is_started_then_waited_for() {
        let mut o = obs(Screen::Other, None, false);
        o.running = false;
        assert_eq!(plan(&o, &goal(), false), Next::Start);
        o.running = true;
        o.window = false;
        assert_eq!(plan(&o, &goal(), false), Next::WaitReady);
        o.window = true;
        o.bridge = false;
        assert_eq!(plan(&o, &goal(), false), Next::WaitReady);
    }

    /// Regression guard: login is never attempted before the window and the
    /// bridge are up (lab_login used to fail at `focus` after 0 ms).
    #[test]
    fn login_waits_for_the_window() {
        let mut o = obs(Screen::Startup, None, false);
        o.window = false;
        assert_ne!(plan(&o, &goal(), false), Next::Login);
        o.window = true;
        assert_eq!(plan(&o, &goal(), false), Next::Login);
        assert_eq!(
            plan(&obs(Screen::Other, None, false), &goal(), false),
            Next::Login
        );
    }

    #[test]
    fn character_select_plays_and_the_world_finishes_a_dialog_once() {
        assert_eq!(
            plan(&obs(Screen::CharSelect, None, false), &goal(), false),
            Next::Play
        );
        let with_dialog = obs(Screen::World, Some("labone"), true);
        assert_eq!(plan(&with_dialog, &goal(), false), Next::FinishDialog);
        assert_eq!(plan(&with_dialog, &goal(), true), Next::Done);
    }

    #[test]
    fn already_in_world_as_the_character_is_done_at_once() {
        assert_eq!(
            plan(&obs(Screen::World, Some("Labone"), false), &goal(), false),
            Next::Done
        );
    }

    #[test]
    fn in_world_as_someone_else_logs_out() {
        assert_eq!(
            plan(&obs(Screen::World, Some("Other"), false), &goal(), false),
            Next::Logout
        );
    }

    /// Regression guard (review of #1309): a name that does not read yet
    /// (right after zone-in) is no reason to log out, and a failed lookup
    /// reads as no name, never as the Lua error text.
    #[test]
    fn an_unread_name_waits_instead_of_logging_out() {
        assert_eq!(
            plan(&obs(Screen::World, None, false), &goal(), false),
            Next::Settle
        );
        let c = observe_chunk();
        assert!(
            c.contains("local ok, n = pcall(unitName, Unit.Player)"),
            "{c}"
        );
        assert!(c.contains("return ''"), "{c}");
    }

    /// Regression guard (review of #1309): the create arguments are
    /// checked before anything runs, and the last name defaults to the
    /// character.
    #[test]
    fn create_arguments_are_checked_up_front() {
        let req = |last: &str| CreateRequest {
            first: "Lab".into(),
            last: last.into(),
            alignment: "sgu".into(),
            archetype: "Soldier".into(),
            gender: "male".into(),
        };
        assert!(req("Labone").check().is_ok());
        assert!(req("").check().unwrap_err().contains("last name"));
        assert!(req("Lab-1").check().unwrap_err().contains("letters only"));
        let mut bad = req("Labone");
        bad.archetype = "Wizard".into();
        assert!(bad.check().is_err());
    }

    /// Walk a whole run: each action moves the simulated client on, and
    /// the planner reaches Done in the expected order.
    #[test]
    fn a_cold_start_runs_start_ready_login_play_dialog_done() {
        let mut o = obs(Screen::Other, None, false);
        o.running = false;
        o.window = false;
        o.bridge = false;
        let mut seen = Vec::new();
        let mut tried = false;
        for _ in 0..MAX_ROUNDS {
            let n = plan(&o, &goal(), tried);
            seen.push(n);
            match n {
                Next::Start => o.running = true,
                Next::WaitReady => (o.window, o.bridge) = (true, true),
                Next::Login => o.screen = Screen::CharSelect,
                Next::Play => {
                    (o.screen, o.player, o.dialog) = (Screen::World, Some("Labone".into()), true)
                }
                Next::FinishDialog => (tried, o.dialog) = (true, false),
                Next::Logout => o.screen = Screen::CharSelect,
                Next::Settle => o.player = Some("Labone".into()),
                Next::Done => break,
            }
        }
        assert_eq!(
            seen,
            [
                Next::Start,
                Next::WaitReady,
                Next::Login,
                Next::Play,
                Next::FinishDialog,
                Next::Done
            ]
        );
    }

    #[test]
    fn stop_at_ends_at_running_or_character_select() {
        let running = Goal {
            stop: StopAt::Running,
            ..goal()
        };
        assert_eq!(
            plan(&obs(Screen::Other, None, false), &running, false),
            Next::Done
        );
        let mut cold = obs(Screen::Other, None, false);
        cold.running = false;
        assert_eq!(plan(&cold, &running, false), Next::Start);

        let select = Goal {
            stop: StopAt::CharacterSelect,
            ..goal()
        };
        assert_eq!(
            plan(&obs(Screen::Startup, None, false), &select, false),
            Next::Login
        );
        assert_eq!(
            plan(&obs(Screen::CharSelect, None, false), &select, false),
            Next::Done
        );
        // In the world, even as the right character: back to select.
        assert_eq!(
            plan(&obs(Screen::World, Some("Labone"), false), &select, false),
            Next::Logout
        );

        assert_eq!(
            StopAt::parse("character_select"),
            Ok(StopAt::CharacterSelect)
        );
        assert_eq!(StopAt::default(), StopAt::World);
        assert!(StopAt::parse("server_select").is_err());
    }

    #[test]
    fn observations_parse_screens_dialog_and_player() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        let world = parse_observe(&s(&[
            "false", "false", "false", "false", "true", "true", "Labone",
        ]));
        assert_eq!(world, (Screen::World, true, Some("Labone".into())));
        let select = parse_observe(&s(&[
            "false", "false", "false", "true", "false", "false", "",
        ]));
        assert_eq!(select, (Screen::CharSelect, false, None));
        let login = parse_observe(&s(&[
            "false", "true", "false", "false", "false", "false", "Ghost",
        ]));
        assert_eq!(
            login,
            (Screen::Startup, false, None),
            "a name off-world is ignored"
        );
        assert_eq!(parse_observe(&[]).0, Screen::Other);
        let c = observe_chunk();
        assert!(
            c.starts_with("return ")
                && c.contains("SelfStatusWin")
                && c.contains("pcall(unitName, Unit.Player)")
        );
    }
}
