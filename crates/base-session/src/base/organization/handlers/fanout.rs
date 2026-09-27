//! Who of an organization is online, and sending client methods to them.
//!
//! "Online" is a view over the connected-client map, never a second index
//! (the `player_index` rule): a session whose character is in the world
//! (`listed_online`, set at `onClientReady`, cleared by `logOff`) with an
//! active `player_id` and a player entity. A session mid world entry is not
//! online yet; it gets the whole state from its own login push instead.

use cimmeria_mercury::channel_bundle::{ChannelBundle, IDBASE_SGW_PLAYER};

use super::OrgCtx;
use crate::base::helpers::send_bundle_to_witness_reliable;

/// One online member's session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OnlineMember {
    pub player_id: i32,
    /// The live player entity: the id the client roster stores (ORG-E1 Q1).
    pub entity_id: u32,
    pub account_id: u32,
}

/// The online sessions among `player_ids`, by `player_id`.
pub fn online_members(ctx: &OrgCtx<'_>, player_ids: &[i32]) -> Vec<OnlineMember> {
    let Ok(clients) = ctx.connected.lock() else {
        return Vec::new();
    };
    let mut out: Vec<OnlineMember> = clients
        .values()
        .filter(|c| c.listed_online)
        .filter_map(|c| {
            let player_id = c.active_player_id.filter(|p| player_ids.contains(p))?;
            Some(OnlineMember {
                player_id,
                entity_id: c.player_entity_id?,
                account_id: c.account_id,
            })
        })
        .collect();
    out.sort_unstable_by_key(|m| m.player_id);
    out
}

/// Send `messages` to `entity_id`'s own client as one reliable bundle.
/// Every message targets the player's own, long-created entity, so they may
/// share a bundle (`channel_bundle` "safe to combine"), and the bundle
/// fragments a roster too big for one packet.
///
/// `Err` carries the `reason` the bundle did not go out; the caller logs it.
pub async fn send_to_player(
    ctx: &OrgCtx<'_>,
    entity_id: u32,
    messages: &[(u16, Vec<u8>)],
) -> Result<(), &'static str> {
    let mut bundle = ChannelBundle::new(true);
    for (method_index, args) in messages {
        bundle.append_entity_method(*method_index, IDBASE_SGW_PLAYER, entity_id, args);
    }
    let outcome = send_bundle_to_witness_reliable(
        ctx.transport,
        ctx.connected,
        ctx.entity_to_addr,
        entity_id,
        bundle,
    )
    .await;
    match outcome.failure_reason() {
        None => Ok(()),
        Some(reason) => Err(reason),
    }
}

/// [`send_to_player`] for each recipient, logging each failed send as WARN
/// `org.send_failed` with `reason`. Returns how many bundles went out.
pub async fn send_to_members(
    ctx: &OrgCtx<'_>,
    org_id: i32,
    recipients: &[OnlineMember],
    messages: &[(u16, Vec<u8>)],
    what: &'static str,
) -> usize {
    let mut sent = 0;
    for r in recipients {
        match send_to_player(ctx, r.entity_id, messages).await {
            Ok(()) => sent += 1,
            Err(reason) => tracing::warn!(
                target: "org",
                event = "org.send_failed",
                what,
                org_id,
                target_account_id = r.account_id,
                target_player_id = r.player_id,
                entity_id = r.entity_id,
                reason,
                "organization message could not be sent to a member"
            ),
        }
    }
    sent
}

/// `text` on the player's feedback channel (the line every refusal owes,
/// work-packets § "Common acceptance").
pub async fn feedback(ctx: &OrgCtx<'_>, entity_id: u32, text: &str) {
    crate::base::gm_feedback::send_gm_feedback_to_client(
        entity_id,
        text,
        ctx.transport,
        ctx.connected,
        ctx.entity_to_addr,
    )
    .await;
}
