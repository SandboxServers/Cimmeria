//! The `StartMinigame` effect script (MG-24, D-MG20): an ability whose effect
//! names a minigame launches it for the caster.
//!
//! An effect script cannot await the base channel, so it only queues a
//! [`MinigameLaunchRequest`] on `space_mgr.minigame_launch.queue`. Combat
//! drains the queue after the effects resolve (`drain_launch_queue`), through
//! `start_or_offer`, so the pre-game dialog applies to ability launches too.
//! The caster plays. A non-player caster, and a caster already in a minigame,
//! are refused.
//!
//! Effect params: `minigame_type` (a game name the client knows) and
//! `difficulty` (1 to 5, default 1; out-of-range values clamp). Log target
//! `abilities`.

use cimmeria_cell_world::cell::minigame_launch::MinigameLaunchRequest;
use cimmeria_wire::state_field::BSF_PLAYING_MINIGAME;

use super::{EffectContext, EffectScript};

/// The minigame type names the client's type table registers. A copy of
/// `MINIGAME_TYPE_NAMES` in `cimmeria-content-engine` (`loader/action.rs`):
/// this crate depends on neither `cimmeria-minigame` nor the content engine,
/// so keep the two lists in step. The match is case-sensitive.
const MINIGAME_TYPE_NAMES: &[&str] = &[
    "Livewire",
    "Alignment",
    "CrystalGame",
    "GoauldCrystals",
    "ConverseBasicHumanoid",
    "Hack",
    "Activate",
    "Analyze",
    "Bypass",
    "Converse",
];

/// The effect param naming the minigame.
const TYPE_PARAM: &str = "minigame_type";

/// The effect param carrying the difficulty.
const DIFFICULTY_PARAM: &str = "difficulty";

/// The difficulty when the effect row carries none.
const DEFAULT_DIFFICULTY: i32 = 1;

/// The lowest and highest difficulty a launch takes.
const MIN_DIFFICULTY: i32 = 1;
const MAX_DIFFICULTY: i32 = 5;

/// Queue a minigame launch for the caster. Effect 1643 and 2813 name it.
pub struct StartMinigame;

impl EffectScript for StartMinigame {
    fn on_apply(&self, ctx: &mut EffectContext) {
        let effect = ctx.effect;
        let game = effect.params.get(TYPE_PARAM).map(String::as_str);
        let Some(game_name) = game.filter(|g| MINIGAME_TYPE_NAMES.contains(g)) else {
            tracing::warn!(
                target: "abilities",
                event = "minigame_launch_refused",
                reason = "unknown_minigame_type",
                game_name = game.unwrap_or(""),
                entity_id = ctx.source_id,
                entity_name = ctx.space_mgr.caster_label(ctx.source_id),
                effect_id = effect.effect_id,
                effect_name = cimmeria_names::book().effect(effect.effect_id),
                ability_id = effect.ability_id,
                ability_name = cimmeria_names::book().ability(effect.ability_id),
                "StartMinigame: effect names no minigame type the client knows -- nothing queued"
            );
            return;
        };
        let row = ctx.row_ids();
        let Some(player_id) = row.player_id else {
            tracing::warn!(
                target: "abilities",
                event = "minigame_launch_refused",
                reason = "npc_caster",
                game_name,
                entity_id = ctx.source_id,
                entity_name = ctx.space_mgr.caster_label(ctx.source_id),
                effect_id = effect.effect_id,
                effect_name = cimmeria_names::book().effect(effect.effect_id),
                ability_id = effect.ability_id,
                ability_name = cimmeria_names::book().ability(effect.ability_id),
                "StartMinigame: caster is not a player -- nothing queued"
            );
            return;
        };
        let already_playing = ctx
            .space_mgr
            .get_entity(ctx.source_id)
            .is_some_and(|e| e.has_state_flag(BSF_PLAYING_MINIGAME));
        if already_playing {
            tracing::warn!(
                target: "abilities",
                event = "minigame_launch_refused",
                reason = "already_playing",
                game_name,
                entity_id = ctx.source_id,
                entity_name = ctx.space_mgr.caster_label(ctx.source_id),
                player_id = row.player_id,
                player_name = row.player_name,
                effect_id = effect.effect_id,
                effect_name = cimmeria_names::book().effect(effect.effect_id),
                ability_id = effect.ability_id,
                ability_name = cimmeria_names::book().ability(effect.ability_id),
                "StartMinigame: caster is already in a minigame -- nothing queued"
            );
            return;
        }
        let difficulty = if effect.params.contains_key(DIFFICULTY_PARAM) {
            effect.param_i32(DIFFICULTY_PARAM)
        } else {
            DEFAULT_DIFFICULTY
        }
        .clamp(MIN_DIFFICULTY, MAX_DIFFICULTY) as u32;
        ctx.space_mgr
            .minigame_launch
            .queue
            .push(MinigameLaunchRequest {
                entity_id: ctx.source_id,
                player_id,
                game_name: game_name.to_string(),
                difficulty,
                on_victory_chains: vec![],
                on_failure_chains: vec![],
                source: "effect_script",
            });
        tracing::info!(
            target: "abilities",
            event = "minigame_launch_queued",
            game_name,
            difficulty,
            entity_id = ctx.source_id,
            entity_name = ctx.space_mgr.caster_label(ctx.source_id),
            player_id,
            player_name = row.player_name,
            effect_id = effect.effect_id,
            effect_name = cimmeria_names::book().effect(effect.effect_id),
            ability_id = effect.ability_id,
            ability_name = cimmeria_names::book().ability(effect.ability_id),
            "StartMinigame: launch queued for the cell to start after the effects resolve"
        );
    }
}
