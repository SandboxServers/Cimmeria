//! Chat dispatch arm for `CellToBaseMsg::Chat`.
//!
//! Every `ChatCellToBase` variant lands here. Later chat packets add their
//! handlers to this file rather than to `mod.rs`.

use crate::base::feedback::FeedbackCtx;
use crate::base::gm_broadcast::{broadcast_to_online_players, GmBroadcastActor};
use crate::base::mutes::gm::{apply_gm_mute, apply_gm_unmute, GmActor, GmMuteCtx};
use crate::base::mutes::mute_table;
use crate::cell::messages::ChatCellToBase;

use super::DispatchCtx;

/// Route one chat message from the cell.
pub(super) async fn route(msg: ChatCellToBase, ctx: &DispatchCtx<'_>) {
    match msg {
        ChatCellToBase::GmBroadcast {
            entity_id,
            player_id,
            account_id,
            source,
            args,
        } => {
            let feedback = FeedbackCtx {
                transport: ctx.transport,
                connected: ctx.connected,
            };
            let actor = GmBroadcastActor {
                entity_id,
                player_id,
                account_id,
            };
            let report = broadcast_to_online_players(&feedback, actor, &args).await;
            // The cell logged `chat.gm_broadcast` (actor, scope, text) when
            // it accepted the shout; this is the delivery half, joined on
            // the actor's ids.
            tracing::info!(
                target: "chat",
                event = "chat.gm_broadcast_delivered",
                entity_id,
                player_id,
                account_id,
                source,
                scope = "global",
                delivered = report.delivered,
                failed = report.failed,
                not_in_world = report.not_in_world,
                "GM broadcast fanned out to every online player",
            );
        }
        ChatCellToBase::Mute {
            entity_id,
            player_id,
            account_id,
            target_name,
            minutes,
            reason,
        } => {
            let actor = GmActor {
                entity_id,
                player_id,
                account_id,
            };
            apply_gm_mute(
                &mute_ctx(ctx),
                mute_table(),
                actor,
                &target_name,
                minutes,
                &reason,
                std::time::Instant::now(),
            )
            .await;
        }
        ChatCellToBase::Unmute {
            entity_id,
            player_id,
            account_id,
            target_name,
        } => {
            let actor = GmActor {
                entity_id,
                player_id,
                account_id,
            };
            apply_gm_unmute(
                &mute_ctx(ctx),
                mute_table(),
                actor,
                &target_name,
                std::time::Instant::now(),
            )
            .await;
        }
    }
}

fn mute_ctx<'a>(ctx: &DispatchCtx<'a>) -> GmMuteCtx<'a> {
    GmMuteCtx {
        feedback: FeedbackCtx {
            transport: ctx.transport,
            connected: ctx.connected,
        },
        entity_to_addr: ctx.entity_to_addr,
    }
}
