//! Livewire minigame — wire cutting puzzle.
//!
//! Players must cut all goal wires before the timer runs out. Cutting
//! obstacle wires accelerates the countdown; moving wires provide
//! score/time bonuses.
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
use crate::minigame::session::{BoardSnapshot, MinigameSession, WireSnapshot};
use crate::sfs_vars;

mod setup;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tests_parity;

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

pub(super) struct LivewireGame {
    // Session params
    /// Entity and player identity, carried so every rejection row names
    /// the player (TG-MG-05).
    pub(super) entity_id: u32,
    pub(super) player_id: i32,
    pub(super) player_name: Option<String>,
    pub(super) difficulty: u32,
    pub(super) tech_competency: u32,
    pub(super) abilities_mask: u32,
    pub(super) player_level: u32,
    pub(super) instcc: i32,
    pub(super) ca: [i32; 5],

    // State
    pub(super) initialized: bool,
    pub(super) outcome: Option<&'static str>,
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
    pub(super) fn new(session: &MinigameSession) -> Self {
        // The content layer asserted difficulty 1-5
        // (`deprecated/python/cell/Minigame.py:16`) but every per-game
        // difficulty table only has rows 1-4 (`Livewire.py:52`,
        // `Alignment.py:14`, `GoauldCrystals.py:52`) — difficulty 5 passed
        // the original's assert and then raised `KeyError` inside the game.
        // Clamping is the right call, but it silently downgrades an
        // authored 5, so say so: the seed row is the thing to fix.
        if session.difficulty > 4 {
            tracing::warn!(
                entity_id = session.entity_id,
                entity_name = session.player_name.as_deref(),
                difficulty = session.difficulty,
                "Livewire: difficulty clamped to 4 -- the game has no table \
                 above 4; fix the authored difficulty in the seed row"
            );
        }
        Self {
            entity_id: session.entity_id,
            player_id: session.player_id,
            player_name: session.player_name.clone(),
            difficulty: session.difficulty.clamp(1, 4),
            tech_competency: session.tech_competency,
            abilities_mask: session.abilities_mask,
            player_level: session.player_level,
            instcc: -1,
            ca: [0; 5],
            initialized: false,
            outcome: None,
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
    fn board_snapshot(&self) -> Option<BoardSnapshot> {
        if !self.initialized {
            return None;
        }
        let mut wires: Vec<_> = self
            .wires
            .values()
            .map(|w| WireSnapshot {
                name: w.name.clone(),
                lib: w.lib.clone(),
                kind: match w.name.chars().next() {
                    Some('g') => "goal",
                    Some('p') => "playfield",
                    Some('m') => "moving",
                    Some('o') => "obstacle",
                    _ => "unknown",
                },
                clickable: self.game_started
                    && self.outcome.is_none()
                    && !w.cut
                    && (!w.name.starts_with('p') || self.playfield_active),
                depth: w.depth,
                x: w.x,
                y: w.y,
                cut: w.cut,
            })
            .collect();
        wires.sort_by_key(|w| w.depth);
        Some(BoardSnapshot {
            entity_id: self.entity_id,
            game_name: "Livewire".to_owned(),
            state: if self.outcome.is_some() {
                "finished"
            } else if self.game_started {
                "running"
            } else {
                "ready"
            },
            outcome: self.outcome,
            difficulty: self.difficulty,
            game_started: self.game_started,
            timer_state: self.timer_state,
            time_remaining: self.time_remaining,
            goal_total: self.goal_total,
            goal_cut: self.goal_cut,
            wires,
            events: Vec::new(),
            events_dropped: 0,
        })
    }
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
                    tracing::warn!(
                        entity_id = self.entity_id,
                        entity_name = self.player_name.as_deref(),
                        player_id = self.player_id,
                        player_name = self.player_name.as_deref(),
                        wire = %wirename,
                        reason = "not_started",
                        "Wire cut when game not started"
                    );
                    return vec![];
                }

                // Find the wire
                let wire = self.wires.values_mut().find(|w| w.name == wirename);
                let wire = match wire {
                    Some(w) => w,
                    None => {
                        tracing::warn!(
                            entity_id = self.entity_id,
                            entity_name = self.player_name.as_deref(),
                            player_id = self.player_id,
                            player_name = self.player_name.as_deref(),
                            wire = %wirename,
                            reason = "unknown_wire",
                            "Unknown wire"
                        );
                        return vec![];
                    }
                };

                if wire.cut {
                    tracing::warn!(
                        entity_id = self.entity_id,
                        entity_name = self.player_name.as_deref(),
                        player_id = self.player_id,
                        player_name = self.player_name.as_deref(),
                        wire = %wirename,
                        reason = "already_cut",
                        "Wire already cut"
                    );
                    return vec![];
                }
                let mut out = Vec::new();
                let prefix = wirename.chars().next().unwrap_or(' ');

                // An ignored playfield wire is refused before it is marked cut.
                if prefix == 'p' && !self.playfield_active {
                    tracing::warn!(
                        entity_id = self.entity_id,
                        entity_name = self.player_name.as_deref(),
                        player_id = self.player_id,
                        player_name = self.player_name.as_deref(),
                        wire = %wirename,
                        reason = "playfield_inactive",
                        "Playfield wire cut when inactive"
                    );
                    return vec![];
                }
                wire.cut = true;

                match prefix {
                    'g' => {
                        self.goal_cut += 1;
                    }
                    'p' => {}
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
                        tracing::warn!(
                            entity_id = self.entity_id,
                            entity_name = self.player_name.as_deref(),
                            player_id = self.player_id,
                            player_name = self.player_name.as_deref(),
                            wire = %wirename,
                            reason = "illegal_prefix",
                            "Illegal wire prefix"
                        );
                        return vec![];
                    }
                }

                out.push(GameOutput::Send(sfs_vars! {
                    "_cmd" => SfsValue::String("destroy".to_string()),
                    "wirename" => SfsValue::String(wirename.clone()),
                }));

                if self.goal_cut == self.goal_total {
                    self.outcome = Some("victory");
                    out.push(GameOutput::Send(sfs_vars! {
                        "_cmd" => SfsValue::String("victory".to_string()),
                    }));
                    out.push(GameOutput::Victory);
                }

                out
            }

            // No cheats in a server-validated game (D-MG17): instcc and CA stay hardcoded.
            "activateitemcheat" => {
                tracing::warn!(
                    entity_id = self.entity_id,
                    entity_name = self.player_name.as_deref(),
                    player_id = self.player_id,
                    player_name = self.player_name.as_deref(),
                    cmd,
                    reason = "item_cheat_unavailable",
                    "Livewire: item cheats are not available"
                );
                vec![]
            }
            "activateabilitycheat" => {
                tracing::warn!(
                    entity_id = self.entity_id,
                    entity_name = self.player_name.as_deref(),
                    player_id = self.player_id,
                    player_name = self.player_name.as_deref(),
                    cmd,
                    reason = "ability_cheat_unavailable",
                    "Livewire: ability cheats are not available"
                );
                vec![]
            }

            _ => {
                tracing::warn!(
                    entity_id = self.entity_id,
                    entity_name = self.player_name.as_deref(),
                    player_id = self.player_id,
                    player_name = self.player_name.as_deref(),
                    cmd,
                    reason = "unknown_command",
                    "Livewire: unknown command"
                );
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
            self.outcome = Some("defeat");
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

    /// The client has closed the window: it is gone, so nothing is sent (no `failure`).
    fn aborted(&mut self) -> Vec<GameOutput> {
        self.outcome = Some("canceled");
        self.game_started = false;
        vec![]
    }

    fn needs_tick(&self) -> bool {
        true
    }
}
