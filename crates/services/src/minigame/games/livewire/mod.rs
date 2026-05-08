//! Livewire minigame — wire cutting puzzle.
//!
//! Port of `python/base/minigame/Livewire.py` (594 lines).
//! Players must cut all goal wires before the timer runs out.
//! Cutting obstacle wires accelerates the countdown; moving wires
//! provide score/time bonuses.
//!
//! The game-state construction (difficulty parameter table, wire
//! generation, full-state serialization) lives in [`setup`] so this
//! file is just the runtime: the LivewireGame struct, the constructor,
//! and the [`MinigameInstance`] trait impl that drives the game loop.

use std::collections::HashMap;

use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::minigame::game::{GameOutput, MinigameInstance};
use crate::minigame::protocol::SfsValue;
use crate::minigame::session::MinigameSession;
use crate::sfs_vars;

mod setup;

#[cfg(test)]
mod tests;

const TICK_RATE: f64 = 0.25;

// ── Wire ─────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub(super) struct Wire {
    pub(super) name: String,
    pub(super) lib: String,
    pub(super) depth: u32,
    pub(super) sound: String,
    pub(super) x: f64,
    pub(super) y: f64,
    pub(super) cut: bool,
}

// ── Livewire game ────────────────────────────────────────────────────────────

pub struct LivewireGame {
    // Session params
    pub(super) difficulty: u32,
    pub(super) tech_competency: u32,
    pub(super) abilities_mask: u32,
    pub(super) player_level: u32,
    pub(super) instcc: i32,
    pub(super) ca: [i32; 5],

    // State
    pub(super) initialized: bool,
    pub(super) game_started: bool,
    pub(super) playfield_active: bool,
    pub(super) timer_state: u32, // 0=stopped, 1=countdown, 2=time-stop cheat, 3=time-reverse cheat
    pub(super) time_remaining: f64,
    pub(super) countdown_rate: f64,
    pub(super) time_units: f64,
    pub(super) read_out: String,

    // Wire counts (computed from difficulty)
    pub(super) goal_total: u32,
    pub(super) goal_cut: u32,
    pub(super) move_timer: f64,
    pub(super) obstacle_timer: f64,

    // Wires keyed by depth
    pub(super) wires: HashMap<u32, Wire>,
    pub(super) glow_points: Vec<String>,

    // Cheat timers
    pub(super) stop_time_cheat_time: f64,
    pub(super) up_time_cheat_time: f64,

    // RNG (seeded, Send-safe)
    pub(super) rng: StdRng,
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
