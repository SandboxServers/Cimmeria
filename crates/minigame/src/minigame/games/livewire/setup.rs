//! Per-game setup — difficulty parameter table, wire generation, and
//! full-game-state serialization for the Livewire minigame.

use std::collections::HashMap;

use rand::RngExt;

use crate::minigame::protocol::SfsValue;
use crate::sfs_vars;

use super::{LivewireGame, Wire};

pub(super) struct DifficultyParams {
    pub countdown_rate: f64,
    pub timer_base: f64,
    pub timer_multiplier: f64,
    pub playfield_base: f64,
    pub playfield_tech_comp: f64,
    pub goals: u32,
    pub moving_base: f64,
    pub moving_tech_comp: f64,
    pub move_timer_divider: f64,
    pub obstacles_base: f64,
    pub obstacles_tech_comp: f64,
    pub obstacle_timer_base: f64,
    pub obstacle_timer_level: f64,
    pub read_out: &'static str,
}

pub(super) const DIFFICULTY_LEVELS: [DifficultyParams; 4] = [
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
        // 0 means moving wires do not change the rate on D4.
        move_timer_divider: 0.0,
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

impl LivewireGame {
    pub(super) fn init_game(&mut self) {
        self.initialized = true;
        self.outcome = None;
        self.game_started = false;
        self.playfield_active = false;
        self.timer_state = 0;
        self.update_difficulty();
        self.setup_wires();
    }

    pub(super) fn update_difficulty(&mut self) {
        // Content difficulty 5 uses difficulty 4's table (the original has no D5 branch).
        let idx = (self.difficulty as usize - 1).min(3);
        let level = &DIFFICULTY_LEVELS[idx];
        let tc = self.tech_competency as f64;

        let param_timer = (level.timer_base + tc * level.timer_multiplier).floor();
        self.goal_total = level.goals;
        let _playfield_total =
            (level.playfield_base + tc * level.playfield_tech_comp).floor() as u32;
        let _moving_total = (level.moving_base + tc * level.moving_tech_comp).floor() as u32;
        // A divider of 0 is the original's unset value: moving wires leave the rate alone.
        self.move_timer = if level.move_timer_divider > 0.0 {
            tc / level.move_timer_divider
        } else {
            0.0
        };
        let _obstacle_total =
            2 * (level.obstacles_base + tc * level.obstacles_tech_comp).floor() as u32;
        // Integer steps: base + 5 * floor((tc - 1) / 5).
        let obstacle_steps = self.tech_competency.saturating_sub(1) / 5;
        self.obstacle_timer =
            level.obstacle_timer_base + obstacle_steps as f64 * level.obstacle_timer_level;
        self.countdown_rate = 1.0 / level.countdown_rate;
        self.time_remaining = level.countdown_rate + 0.25;
        self.read_out = format!("{}{}", level.read_out, self.tech_competency);

        // Random timer type (matching Python)
        let timer_type = self.rng.random_range(0..3u32);
        self.time_units = match timer_type {
            0 => (param_timer * tc) / 1.7,
            // The original computes (17 + tc*4.5) ^ 1.25 as a bitwise XOR on truncated
            // integers. The port keeps a real power on purpose.
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

    /// Goal depth, the port of `twoRandRange(1,25,60,85)`: draw `a` in 1..=25, then `b`
    /// in 60..=85, then pick one with `random_range(0..2)`. Retries on a taken depth.
    fn find_goal_depth(&mut self) -> u32 {
        let mut attempts = 0;
        loop {
            let a = self.rng.random_range(1..=25u32);
            let b = self.rng.random_range(60..=85u32);
            let d = if self.rng.random_range(0..2u32) == 0 {
                a
            } else {
                b
            };
            if !self.wires.contains_key(&d) {
                return d;
            }
            attempts += 1;
            if attempts >= 1000 {
                return self.find_available_depth(1);
            }
        }
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

    pub(super) fn setup_wires(&mut self) {
        self.wires.clear();
        self.glow_points.clear();
        self.goal_cut = 0;

        let idx = (self.difficulty as usize - 1).min(3);
        let level = &DIFFICULTY_LEVELS[idx];
        let tc = self.tech_competency as f64;

        let playfield_total =
            (level.playfield_base + tc * level.playfield_tech_comp).floor() as u32;
        let moving_total = (level.moving_base + tc * level.moving_tech_comp).floor() as u32;
        let obstacle_total =
            2 * (level.obstacles_base + tc * level.obstacles_tech_comp).floor() as u32;

        // Playfield electrodes (30% of playfield)
        let electrodes = (playfield_total as f64 * 0.3).floor() as u32;
        for i in 0..electrodes {
            let depth = self.find_available_depth(101);
            let lib = self.random_item(PLAYFIELD_ELECTRODES).to_string();
            let sound = self.random_item(PLAYFIELD_SOUNDS).to_string();
            let x = self.rng.random_range(375.0f64..900.0).floor();
            let y = self.rng.random_range(250.0f64..700.0).floor();
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
            let base = self.random_item(PLAYFIELD_WIRES);
            let variant = self.rng.random_range(1..=4u32);
            let lib = format!("{base}{variant}");
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
            let depth = self.find_goal_depth();
            let base = self.random_item(GOAL_WIRES);
            let variant = self.rng.random_range(1..=4u32);
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
            let x = self.rng.random_range(375.0f64..900.0).floor();
            let y = self.rng.random_range(250.0f64..700.0).floor();
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
            let base = self.random_item(MOVING_WIRES);
            let variant = self.rng.random_range(1..=4u32);
            let lib = format!("{base}{variant}");
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
            let variant = self.rng.random_range(1..=4u32);
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

    pub(super) fn build_full_game_state(&self) -> HashMap<String, SfsValue> {
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

        sfs_vars! {
            "_cmd" => SfsValue::String("fullgamestate".to_string()),
            "abilityBitfield" => SfsValue::String(self.abilities_mask.to_string()),
            "wireSounds" => SfsValue::list(wire_sounds),
            "wireNames" => SfsValue::list(wire_names),
            "glowPoints" => SfsValue::list(glow_points),
            "wireXs" => SfsValue::list(wire_xs),
            "wireYs" => SfsValue::list(wire_ys),
            "wireDepths" => SfsValue::list(wire_depths),
            "wireLibs" => SfsValue::list(wire_libs),
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
            "timerState" => SfsValue::Number(self.timer_state as f64),
        }
    }
}
