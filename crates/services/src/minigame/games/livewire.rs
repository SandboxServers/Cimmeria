//! Livewire minigame — wire cutting puzzle.
//!
//! Port of `python/base/minigame/Livewire.py` (594 lines).
//! Players must cut all goal wires before the timer runs out.
//! Cutting obstacle wires accelerates the countdown; moving wires
//! provide score/time bonuses.

use std::collections::HashMap;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::minigame::game::{GameOutput, MinigameInstance};
use crate::minigame::protocol::SfsValue;
use crate::minigame::session::MinigameSession;
use crate::sfs_vars;

// ── Constants ────────────────────────────────────────────────────────────────

const TICK_RATE: f64 = 0.25;

struct DifficultyParams {
    countdown_rate: f64,
    timer_base: f64,
    timer_multiplier: f64,
    playfield_base: f64,
    playfield_tech_comp: f64,
    goals: u32,
    moving_base: f64,
    moving_tech_comp: f64,
    move_timer_divider: f64,
    obstacles_base: f64,
    obstacles_tech_comp: f64,
    obstacle_timer_base: f64,
    obstacle_timer_level: f64,
    read_out: &'static str,
}

const DIFFICULTY_LEVELS: [DifficultyParams; 4] = [
    DifficultyParams {
        countdown_rate: 20.0,
        timer_base: 30.0,
        timer_multiplier: 0.588235,
        playfield_base: 6.0,
        playfield_tech_comp: 0.075472,
        goals: 2,
        moving_base: 2.0,
        moving_tech_comp: 0.188679,
        move_timer_divider: 2.5,
        obstacles_base: 2.0,
        obstacles_tech_comp: 0.188679,
        obstacle_timer_base: 5.0,
        obstacle_timer_level: 5.0,
        read_out: "I-",
    },
    DifficultyParams {
        countdown_rate: 17.0,
        timer_base: 20.0,
        timer_multiplier: 0.588235,
        playfield_base: 6.0,
        playfield_tech_comp: 0.094340,
        goals: 4,
        moving_base: 2.0,
        moving_tech_comp: 0.188679,
        move_timer_divider: 2.0,
        obstacles_base: 2.0,
        obstacles_tech_comp: 0.188679,
        obstacle_timer_base: 6.0,
        obstacle_timer_level: 5.0,
        read_out: "S-",
    },
    DifficultyParams {
        countdown_rate: 15.0,
        timer_base: 20.0,
        timer_multiplier: 0.392157,
        playfield_base: 6.0,
        playfield_tech_comp: 0.094340,
        goals: 4,
        moving_base: 3.0,
        moving_tech_comp: 0.245283,
        move_timer_divider: 1.5,
        obstacles_base: 3.0,
        obstacles_tech_comp: 0.245283,
        obstacle_timer_base: 7.0,
        obstacle_timer_level: 5.0,
        read_out: "C-",
    },
    DifficultyParams {
        countdown_rate: 12.0,
        timer_base: 12.0,
        timer_multiplier: 0.372549,
        playfield_base: 6.0,
        playfield_tech_comp: 0.150943,
        goals: 6,
        moving_base: 5.0,
        moving_tech_comp: 0.320755,
        move_timer_divider: 1.0,
        obstacles_base: 3.0,
        obstacles_tech_comp: 0.245283,
        obstacle_timer_base: 9.0,
        obstacle_timer_level: 5.0,
        read_out: "E-",
    },
];

const PLAYFIELD_SOUNDS: &[&str] = &["hackMG/play/low", "hackMG/play/mid", "hackMG/play/high"];
const MOVING_SOUNDS: &[&str] = &["hackMG/move/low", "hackMG/move/mid", "hackMG/move/high"];
const GOAL_SOUNDS: &[&str] = &["hackMG/goal/low", "hackMG/goal/mid", "hackMG/goal/high"];
const OBSTACLE_SOUNDS: &[&str] = &["hackMG/obsta/low", "hackMG/obsta/mid", "hackMG/obsta/high"];

const PLAYFIELD_WIRES: &[&str] = &["pGray", "pRust", "pBronze", "pGreen"];
const PLAYFIELD_ELECTRODES: &[&str] = &[
    "pProcessorFan1",
    "pPurpleBlack1",
    "pPurpleBlack2",
    "pOrangeSilver1",
    "pOrangeSilver2",
    "pLEDWhite",
    "pLEDRed",
    "pLEDBlue",
    "pLEDGreen",
    "pLEDYellow",
];
const MOVING_WIRES: &[&str] = &[
    "mYellowStripe",
    "mPurpleStripe",
    "mOrangeStripe",
    "mBlackStripe",
];
const MOVING_ELECTRODES: &[&str] = &["mGear", "mMicroChip1", "mMicroChip2"];
const GOAL_WIRES: &[&str] = &["gGreenBlack", "gRedSilver", "gBlackGreen"];
const OBSTACLE_WIRES: &[&str] = &["oRed", "oGreen", "oBlack"];

const WIRE_LOCATION_X: f64 = 275.6;
const WIRE_LOCATION_Y: &[f64] = &[391.0, 497.0, 577.0, 657.0, 783.0];

// ── Wire ─────────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct Wire {
    name: String,
    lib: String,
    depth: u32,
    sound: String,
    x: f64,
    y: f64,
    cut: bool,
}

// ── Livewire game ────────────────────────────────────────────────────────────

pub struct LivewireGame {
    // Session params
    difficulty: u32,
    tech_competency: u32,
    abilities_mask: u32,
    player_level: u32,
    instcc: i32,
    ca: [i32; 5],

    // State
    initialized: bool,
    game_started: bool,
    playfield_active: bool,
    timer_state: u32, // 0=stopped, 1=countdown, 2=time-stop cheat, 3=time-reverse cheat
    time_remaining: f64,
    countdown_rate: f64,
    time_units: f64,
    read_out: String,

    // Wire counts (computed from difficulty)
    goal_total: u32,
    goal_cut: u32,
    move_timer: f64,
    obstacle_timer: f64,

    // Wires keyed by depth
    wires: HashMap<u32, Wire>,
    glow_points: Vec<String>,

    // Cheat timers
    stop_time_cheat_time: f64,
    up_time_cheat_time: f64,

    // RNG (seeded, Send-safe)
    rng: StdRng,
}

impl LivewireGame {
    pub fn new(session: &MinigameSession) -> Self {
        Self {
            difficulty: session.difficulty.clamp(1, 4),
            tech_competency: session.tech_competency,
            abilities_mask: session.abilities_mask,
            player_level: session.player_level,
            instcc: -1,
            ca: [0; 5],
            initialized: false,
            game_started: false,
            playfield_active: false,
            timer_state: 0,
            time_remaining: 0.0,
            countdown_rate: 0.0,
            time_units: 0.0,
            read_out: String::new(),
            goal_total: 0,
            goal_cut: 0,
            move_timer: 0.0,
            obstacle_timer: 0.0,
            wires: HashMap::new(),
            glow_points: Vec::new(),
            stop_time_cheat_time: 0.0,
            up_time_cheat_time: 0.0,
            rng: StdRng::seed_from_u64(session.seed as u64),
        }
    }

    fn update_difficulty(&mut self) {
        let idx = (self.difficulty as usize - 1).min(3);
        let level = &DIFFICULTY_LEVELS[idx];
        let tc = self.tech_competency as f64;

        let param_timer = (level.timer_base + tc * level.timer_multiplier).floor();
        self.goal_total = level.goals;
        let _playfield_total =
            (level.playfield_base + tc * level.playfield_tech_comp).floor() as u32;
        let _moving_total = (level.moving_base + tc * level.moving_tech_comp).floor() as u32;
        self.move_timer = tc / level.move_timer_divider;
        let _obstacle_total =
            (level.obstacles_base + tc * level.obstacles_tech_comp).floor() as u32;
        self.obstacle_timer = level.obstacle_timer_base + (tc / 5.0) * level.obstacle_timer_level;
        self.countdown_rate = 1.0 / level.countdown_rate;
        self.time_remaining = level.countdown_rate + 0.25;
        self.read_out = format!("{}{}", level.read_out, self.tech_competency);

        // Random timer type (matching Python)
        let timer_type = self.rng.random_range(0..3u32);
        self.time_units = match timer_type {
            0 => param_timer + tc / 1.7,
            1 => 17.0 + tc * 4.5f64.powf(1.25),
            _ => {
                6.0 * (self.obstacle_timer / 10.0 * self.goal_total as f64)
                    + (21.0 + self.player_level as f64)
            }
        };
    }

    fn find_available_depth(&self, start: u32) -> u32 {
        let mut d = start;
        while self.wires.contains_key(&d) {
            d += 1;
        }
        d
    }

    fn find_random_depth(&mut self, min: u32, max: u32) -> u32 {
        let mut d = self.rng.random_range(min..max);
        let mut attempts = 0;
        while self.wires.contains_key(&d) && attempts < 1000 {
            d = self.rng.random_range(min..max);
            attempts += 1;
        }
        if self.wires.contains_key(&d) {
            self.find_available_depth(min)
        } else {
            d
        }
    }

    fn random_item<'a>(&mut self, items: &'a [&str]) -> &'a str {
        let idx = self.rng.random_range(0..items.len().max(1));
        items[idx.min(items.len() - 1)]
    }

    fn random_y(&mut self) -> f64 {
        let idx = self.rng.random_range(0..WIRE_LOCATION_Y.len());
        WIRE_LOCATION_Y[idx]
    }

    fn setup_wires(&mut self) {
        self.wires.clear();
        self.glow_points.clear();
        self.goal_cut = 0;

        let idx = (self.difficulty as usize - 1).min(3);
        let level = &DIFFICULTY_LEVELS[idx];
        let tc = self.tech_competency as f64;

        let playfield_total =
            (level.playfield_base + tc * level.playfield_tech_comp).floor() as u32;
        let moving_total = (level.moving_base + tc * level.moving_tech_comp).floor() as u32;
        let obstacle_total = (level.obstacles_base + tc * level.obstacles_tech_comp).floor() as u32;

        // Playfield electrodes (30% of playfield)
        let electrodes = (playfield_total as f64 * 0.3).floor() as u32;
        for i in 0..electrodes {
            let depth = self.find_available_depth(101);
            let lib = self.random_item(PLAYFIELD_ELECTRODES).to_string();
            let sound = self.random_item(PLAYFIELD_SOUNDS).to_string();
            let x = self.rng.random_range(375.0..900.0);
            let y = self.rng.random_range(250.0..700.0);
            self.wires.insert(
                depth,
                Wire {
                    name: format!("pe{}", i + 1),
                    lib,
                    depth,
                    sound,
                    x,
                    y,
                    cut: false,
                },
            );
        }

        // Playfield wires
        for i in 0..(playfield_total - electrodes) {
            let depth = self.find_random_depth(1, 100);
            let lib = self.random_item(PLAYFIELD_WIRES).to_string();
            let sound = self.random_item(PLAYFIELD_SOUNDS).to_string();
            let y = self.random_y();
            self.wires.insert(
                depth,
                Wire {
                    name: format!("p{}", i + 1),
                    lib,
                    depth,
                    sound,
                    x: WIRE_LOCATION_X,
                    y,
                    cut: false,
                },
            );
        }

        // Goal wires
        for i in 0..self.goal_total {
            let depth = self.find_random_depth(1, 100);
            let base = self.random_item(GOAL_WIRES);
            let variant = self.rng.random_range(1..4u32);
            let lib = format!("{base}{variant}");
            let sound = self.random_item(GOAL_SOUNDS).to_string();
            let y_idx = self.rng.random_range(0..WIRE_LOCATION_Y.len());
            let y = WIRE_LOCATION_Y[y_idx];
            self.wires.insert(
                depth,
                Wire {
                    name: format!("g{}", i + 1),
                    lib,
                    depth,
                    sound,
                    x: WIRE_LOCATION_X,
                    y,
                    cut: false,
                },
            );
            self.glow_points.push(format!("attach{}", y_idx + 1));
        }

        // Moving electrodes (30% of moving)
        let moving_electrodes = (moving_total as f64 * 0.3).floor() as u32;
        for i in 0..moving_electrodes {
            let depth = self.find_available_depth(101);
            let lib = self.random_item(MOVING_ELECTRODES).to_string();
            let sound = self.random_item(MOVING_SOUNDS).to_string();
            let x = self.rng.random_range(375.0..900.0);
            let y = self.rng.random_range(250.0..700.0);
            self.wires.insert(
                depth,
                Wire {
                    name: format!("me{}", i + 1),
                    lib,
                    depth,
                    sound,
                    x,
                    y,
                    cut: false,
                },
            );
        }

        // Moving wires
        for i in 0..(moving_total - moving_electrodes) {
            let depth = self.find_random_depth(1, 100);
            let lib = self.random_item(MOVING_WIRES).to_string();
            let sound = self.random_item(MOVING_SOUNDS).to_string();
            let y = self.random_y();
            self.wires.insert(
                depth,
                Wire {
                    name: format!("m{}", i + 1),
                    lib,
                    depth,
                    sound,
                    x: WIRE_LOCATION_X,
                    y,
                    cut: false,
                },
            );
        }

        // Obstacle wires
        for i in 0..obstacle_total {
            let depth = self.find_random_depth(1, 100);
            let base = self.random_item(OBSTACLE_WIRES);
            let variant = self.rng.random_range(1..4u32);
            let lib = format!("{base}{variant}");
            let sound = self.random_item(OBSTACLE_SOUNDS).to_string();
            let y = self.random_y();
            self.wires.insert(
                depth,
                Wire {
                    name: format!("o{}", i + 1),
                    lib,
                    depth,
                    sound,
                    x: WIRE_LOCATION_X,
                    y,
                    cut: false,
                },
            );
        }
    }

    fn build_full_game_state(&self) -> HashMap<String, SfsValue> {
        let wires: Vec<&Wire> = {
            let mut keys: Vec<u32> = self.wires.keys().copied().collect();
            keys.sort();
            keys.iter().map(|k| &self.wires[k]).collect()
        };

        let wire_names: Vec<SfsValue> = wires
            .iter()
            .map(|w| SfsValue::String(w.name.clone()))
            .collect();
        let wire_libs: Vec<SfsValue> = wires
            .iter()
            .map(|w| SfsValue::String(w.lib.clone()))
            .collect();
        let wire_sounds: Vec<SfsValue> = wires
            .iter()
            .map(|w| SfsValue::String(w.sound.clone()))
            .collect();
        let wire_xs: Vec<SfsValue> = wires.iter().map(|w| SfsValue::Number(w.x)).collect();
        let wire_ys: Vec<SfsValue> = wires.iter().map(|w| SfsValue::Number(w.y)).collect();
        let wire_depths: Vec<SfsValue> = wires
            .iter()
            .map(|w| SfsValue::Number(w.depth as f64))
            .collect();
        let glow_points: Vec<SfsValue> = self
            .glow_points
            .iter()
            .map(|g| SfsValue::String(g.clone()))
            .collect();

        // Pack arrays as objects with numeric keys (SFS convention)
        fn array_to_obj(arr: Vec<SfsValue>) -> SfsValue {
            let mut map = HashMap::new();
            for (i, v) in arr.into_iter().enumerate() {
                map.insert(i.to_string(), v);
            }
            SfsValue::Object(map)
        }

        sfs_vars! {
            "_cmd" => SfsValue::String("fullgamestate".to_string()),
            "abilityBitfield" => SfsValue::String(self.abilities_mask.to_string()),
            "wireSounds" => array_to_obj(wire_sounds),
            "wireNames" => array_to_obj(wire_names),
            "glowPoints" => array_to_obj(glow_points),
            "wireXs" => array_to_obj(wire_xs),
            "wireYs" => array_to_obj(wire_ys),
            "wireDepths" => array_to_obj(wire_depths),
            "wireLibs" => array_to_obj(wire_libs),
            "gameReadOut" => SfsValue::String(self.read_out.clone()),
            "gameStarted" => SfsValue::String(if self.game_started { "true" } else { "false" }.to_string()),
            "countdownRate" => SfsValue::String(self.countdown_rate.to_string()),
            "playfieldActive" => SfsValue::String(if self.playfield_active { "true" } else { "false" }.to_string()),
            "instcc" => SfsValue::String(self.instcc.to_string()),
            "CA0" => SfsValue::String(self.ca[0].to_string()),
            "CA1" => SfsValue::String(self.ca[1].to_string()),
            "CA2" => SfsValue::String(self.ca[2].to_string()),
            "CA3" => SfsValue::String(self.ca[3].to_string()),
            "CA4" => SfsValue::String(self.ca[4].to_string()),
            "timeUnits" => SfsValue::String(self.time_units.to_string()),
            "timeRemaining" => SfsValue::Number(self.time_remaining),
            "timerState" => SfsValue::String("0.0".to_string()),
        }
    }

    fn init_game(&mut self) {
        self.initialized = true;
        self.game_started = false;
        self.playfield_active = false;
        self.timer_state = 0;
        self.update_difficulty();
        self.setup_wires();
    }
}

impl MinigameInstance for LivewireGame {
    fn started(&mut self) -> Vec<GameOutput> {
        self.init_game();
        vec![GameOutput::Send(self.build_full_game_state())]
    }

    fn message(&mut self, cmd: &str, params: &HashMap<String, SfsValue>) -> Vec<GameOutput> {
        match cmd {
            "opendoor" => {
                self.game_started = true;
                if self.timer_state == 0 {
                    self.timer_state = 1;
                }
                let mut out = vec![GameOutput::Send(sfs_vars! {
                    "_cmd" => SfsValue::String("opendoor".to_string()),
                })];
                // Send initial timer update
                out.push(GameOutput::Send(sfs_vars! {
                    "_cmd" => SfsValue::String("timerupdate".to_string()),
                    "timeRemaining" => SfsValue::Number(self.time_remaining - TICK_RATE),
                    "timerState" => SfsValue::Number(self.timer_state as f64),
                }));
                out
            }

            "closedoor" => {
                vec![GameOutput::Send(sfs_vars! {
                    "_cmd" => SfsValue::String("closedoor".to_string()),
                    "wirename" => SfsValue::String(String::new()),
                })]
            }

            "processreset" => {
                self.game_started = false;
                self.init_game();
                let mut out = vec![GameOutput::Send(sfs_vars! {
                    "_cmd" => SfsValue::String("resetgame".to_string()),
                })];
                out.push(GameOutput::Send(self.build_full_game_state()));
                out
            }

            "processover" => {
                let wirename = params
                    .get("wirename")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                vec![GameOutput::Send(sfs_vars! {
                    "_cmd" => SfsValue::String("over".to_string()),
                    "wirename" => SfsValue::String(wirename.to_string()),
                })]
            }

            "processout" => {
                let wirename = params
                    .get("wirename")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                vec![GameOutput::Send(sfs_vars! {
                    "_cmd" => SfsValue::String("out".to_string()),
                    "wirename" => SfsValue::String(wirename.to_string()),
                })]
            }

            "processmove" => {
                let wirename = params
                    .get("wirename")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                if !self.game_started {
                    tracing::warn!(wire = %wirename, "Wire cut when game not started");
                    return vec![];
                }

                // Find the wire
                let wire = self.wires.values_mut().find(|w| w.name == wirename);
                let wire = match wire {
                    Some(w) => w,
                    None => {
                        tracing::warn!(wire = %wirename, "Unknown wire");
                        return vec![];
                    }
                };

                if wire.cut {
                    tracing::warn!(wire = %wirename, "Wire already cut");
                    return vec![];
                }
                wire.cut = true;

                let mut out = Vec::new();
                let prefix = wirename.chars().next().unwrap_or(' ');

                match prefix {
                    'g' => {
                        self.goal_cut += 1;
                    }
                    'p' => {
                        if !self.playfield_active {
                            tracing::warn!(wire = %wirename, "Playfield wire cut when inactive");
                            return vec![];
                        }
                    }
                    'o' => {
                        self.countdown_rate += self.countdown_rate * self.obstacle_timer / 100.0;
                        out.push(GameOutput::Send(sfs_vars! {
                            "_cmd" => SfsValue::String("countdownupdate".to_string()),
                            "countdownRate" => SfsValue::Number(self.countdown_rate),
                        }));
                    }
                    'm' => {
                        self.countdown_rate += self.countdown_rate * self.move_timer / 100.0;
                        out.push(GameOutput::Send(sfs_vars! {
                            "_cmd" => SfsValue::String("countdownupdate".to_string()),
                            "countdownRate" => SfsValue::Number(self.countdown_rate),
                        }));
                    }
                    _ => {
                        tracing::warn!(wire = %wirename, "Illegal wire prefix");
                        return vec![];
                    }
                }

                out.push(GameOutput::Send(sfs_vars! {
                    "_cmd" => SfsValue::String("destroy".to_string()),
                    "wirename" => SfsValue::String(wirename.clone()),
                }));

                if self.goal_cut == self.goal_total {
                    out.push(GameOutput::Send(sfs_vars! {
                        "_cmd" => SfsValue::String("victory".to_string()),
                    }));
                    out.push(GameOutput::Victory);
                }

                out
            }

            "activateitemcheat" | "activateabilitycheat" => {
                // Stub — cheats acknowledged but no effect yet
                vec![]
            }

            _ => {
                tracing::warn!(cmd, "Livewire: unknown command");
                vec![]
            }
        }
    }

    fn tick(&mut self) -> Vec<GameOutput> {
        if !self.game_started {
            return vec![];
        }

        match self.timer_state {
            1 => {
                // Normal countdown
                self.time_remaining -= TICK_RATE;
            }
            2 => {
                // Stop time cheat
                self.stop_time_cheat_time -= TICK_RATE;
                if self.stop_time_cheat_time <= 0.0 {
                    self.time_remaining += self.stop_time_cheat_time;
                    self.timer_state = 1;
                }
            }
            3 => {
                // Reverse time cheat
                self.up_time_cheat_time -= TICK_RATE;
                self.time_remaining += TICK_RATE;
                if self.up_time_cheat_time <= 0.0 {
                    self.time_remaining += self.up_time_cheat_time * 2.0;
                    self.timer_state = 1;
                }
            }
            _ => {}
        }

        if self.time_remaining <= 0.0 && self.timer_state >= 1 {
            self.timer_state = 0;
            self.game_started = false;
            return vec![
                GameOutput::Send(sfs_vars! {
                    "_cmd" => SfsValue::String("failure".to_string()),
                }),
                GameOutput::Failure,
            ];
        }

        vec![]
    }

    fn aborted(&mut self) -> Vec<GameOutput> {
        self.game_started = false;
        vec![GameOutput::Send(sfs_vars! {
            "_cmd" => SfsValue::String("failure".to_string()),
        })]
    }

    fn needs_tick(&self) -> bool {
        true
    }
}
