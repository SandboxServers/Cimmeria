//! The Debug Area dial hub (DA-07): a GM who opens the DHD at an
//! outbound-only `debug_dial_hub` gate may dial every other gate this server
//! can enter.
//!
//! **This is a grant, not an authorization.** Dial authorization stays the
//! one check in [`super::address_book`]: this module only puts addresses into
//! the GM's *in-memory* book, exactly as `gmDHD` does for its one address
//! (`cimmeria_cell_console::cell::console::gm::travel::handle_dhd`), and the
//! dial is then judged like anybody else's. Giving the dial primitive a
//! "GM at the hub" bypass instead would be a second authorization surface on
//! a check whose whole value is having exactly one.
//!
//! **When: on world entry, and again on DHD open.** The grant runs when a
//! GM's world entry into the hub's world finishes (`InitPlayerState`, from
//! [`top_up_gm_on_hub_world_entry`]) and again, as a no-op unless something
//! was missed, when the DHD opens ([`top_up_gm_dial_hub`]).
//!
//! The DHD-open-only version (DA-07) failed live (DA-06, DA-F3): the
//! eleven granted gates listed as "Unknown" on the first open and could not
//! be dialled, and reopening fixed it. The client's `updateStargateAddress`
//! handler (`GateTravel` member `FUN_00e2eff0`, bound at `FUN_00e2f780`)
//! does not resolve the address on receipt: for a new id it builds an
//! address record, asks the cooked-data cache for the stargate element
//! (`Detail__unknown_00e31650`) and parks the record in a pending vector
//! (`this+0x34`) as well as the known list. The name and glyphs are filled
//! when that element arrives, and an open DHD window is not refreshed when
//! they are. Sent in the same tick as `onDisplayDHD`, the records were still
//! pending when the window drew them. See
//! `docs/reverse-engineering/findings/stargate-dhd-state-machine.md`.
//!
//! The earlier objection to a world-entry grant was that the base hands the
//! client its whole book in `setupStargateInfo` at map load, so a cell push
//! during world entry could land first and be overwritten. `InitPlayerState`
//! does not have that race: the base sends it from `onClientReady`, which
//! the client sends only after it has processed the `mapLoaded` bundle that
//! carries `setupStargateInfo` (and on a cross-world transition the base
//! sends the synthesised `mapLoaded` bundle before it forwards
//! `InitPlayerState`). The cell's own book is stamped from the database in
//! the same message just before the grant, so the grant is never lost to
//! that stamp either. A GM then walks from the arrival to the DHD, which
//! gives the cooked elements seconds to arrive instead of none.
//!
//! The DHD-open pass stays because it re-checks the access level at the
//! moment it matters (a GM demoted mid-session gets nothing more) and covers
//! any path into the world that skipped `InitPlayerState`.
//!
//! **What a GM is offered.** Every gate that is not a hub, is not on the
//! hub's own world, and whose world [`SpaceManager::world_is_enterable`] —
//! the gates on the fourteen 2009 worlds this server has no map for are left
//! out and logged by name, because dialling one tears the traveller out of
//! the Debug Area and then fails to create a space for them. A gate whose
//! arrival is known to be unusable is also left out, with its reason, until
//! it is pinned ([`HUB_EXCLUDED_GATES`]).
//!
//! **Nothing is persisted.** The top-up lives on the cell entity and dies
//! with it on the next transfer. A gate the GM actually travels to is then
//! learned by the arrival unlock in `base::world_entry::gate_travel`, the
//! same as a `gmDHD` dial; the hub itself never enters an address book
//! (that statement filters `debug_dial_hub`).

use tokio::sync::mpsc;

use cimmeria_cell_world::cell::dispatch::is_gm;

use crate::cell::client_methods::gate_travel::UPDATE_STARGATE_ADDRESS;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `updateStargateAddress(INT32 addressId, UINT8 hasAddress, UINT8 hidden)`
/// (`entities/defs/interfaces/GateTravel.def`): the address is held, not
/// hidden. Same bytes as the content verb `grant_stargate_address` sends.
pub(crate) fn update_stargate_address_args(stargate_id: i32) -> Vec<u8> {
    let mut args = Vec::with_capacity(6);
    args.extend_from_slice(&stargate_id.to_le_bytes());
    args.push(1); // hasAddress
    args.push(0); // hidden
    args
}

/// Gates the hub does not offer although their world can be entered, each
/// with the reason the grant log carries. An entry leaves when its arrival is
/// fixed (an `arrival_*` pin or a respawner for the world); the live-DB
/// survey in `gate_travel/tests/debug_area_live_db.rs` re-measures it.
///
/// - 22 `Men'fa (SGU)`: the gate row (and the client's cooked entry) is at
///   y -191.9, but `menfa_light.nav` has no polygon within 3 m of it and its
///   playable surface at that XZ is near y 0, about 192 m above. Men'fa
///   (Praxis), gate 7, has the same row and stands on `menfa_dark.nav`, so
///   the two maps differ and only an in-client look can place the Light
///   gate's pad (DA-06). World 78 has no respawner to fall back on.
pub(crate) const HUB_EXCLUDED_GATES: &[(i32, &str)] = &[(
    22,
    "arrival ~192 m below menfa_light.nav's playable surface; needs an arrival_* pin (DA-06)",
)];

/// What [`top_up_gm_dial_hub`] did, for the caller's tests and the log.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct HubTopUp {
    /// Stargate ids newly added to the book, ascending.
    pub(crate) granted: Vec<i32>,
    /// Offered gates the GM already held.
    pub(crate) already_known: usize,
    /// Gates left out because their world cannot be entered here, ascending.
    pub(crate) unenterable: Vec<i32>,
    /// Gates left out by [`HUB_EXCLUDED_GATES`], ascending.
    pub(crate) excluded: Vec<i32>,
}

/// What ran the top-up, for the log's `trigger` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HubTopUpTrigger {
    /// The GM's world entry into the hub's world finished (`InitPlayerState`).
    WorldEntry,
    /// The GM opened the hub's DHD.
    DhdOpen,
}

impl HubTopUpTrigger {
    fn label(self) -> &'static str {
        match self {
            Self::WorldEntry => "world_entry",
            Self::DhdOpen => "dhd_open",
        }
    }
}

/// If `hub_stargate_id` is a `debug_dial_hub` gate and the caller is a GM,
/// add every dialable gate to the caller's in-memory address book and tell
/// the client. Returns `None` when nothing was attempted (not a hub, not a
/// GM, no entity). The DHD-open pass; see the module doc for why the
/// world-entry pass ([`top_up_gm_on_hub_world_entry`]) runs first.
pub(crate) async fn top_up_gm_dial_hub(
    entity_id: u32,
    hub_stargate_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> Option<HubTopUp> {
    top_up(
        entity_id,
        hub_stargate_id,
        HubTopUpTrigger::DhdOpen,
        tx,
        space_mgr,
    )
    .await
}

/// The world-entry pass: if the GM's world has a `debug_dial_hub` gate,
/// grant as [`top_up_gm_dial_hub`] does, so the client has resolved every
/// address before the GM reaches the DHD (DA-F3). Called from the cell's
/// `InitPlayerState` handler after the address book is stamped from the
/// database. A world with no hub, and a non-GM, are no-ops.
pub async fn top_up_gm_on_hub_world_entry(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let _ = top_up_on_world_entry(entity_id, tx, space_mgr).await;
}

/// [`top_up_gm_on_hub_world_entry`] with the result, for tests.
pub(crate) async fn top_up_on_world_entry(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> Option<HubTopUp> {
    let world = space_mgr.get_entity_world_name(entity_id)?;
    // Lowest id, so the pick is stable if a world ever carries two hubs
    // (`stargates` is a HashMap); the grant does not depend on which.
    let hub_stargate_id = space_mgr
        .stargates
        .iter()
        .filter(|(_, g)| g.debug_dial_hub && g.world_name == world)
        .map(|(id, _)| *id)
        .min()?;
    top_up(
        entity_id,
        hub_stargate_id,
        HubTopUpTrigger::WorldEntry,
        tx,
        space_mgr,
    )
    .await
}

async fn top_up(
    entity_id: u32,
    hub_stargate_id: i32,
    trigger: HubTopUpTrigger,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> Option<HubTopUp> {
    let hub = space_mgr.stargates.get(&hub_stargate_id)?;
    if !hub.debug_dial_hub {
        return None;
    }
    let hub_world = hub.world_name.clone();

    let entity = space_mgr.get_entity(entity_id)?;
    let (player_id, access_level) = (entity.player_id, entity.access_level);
    // Owned: the entity is borrowed mutably below.
    let player_name = entity.identity().player_name;
    let entity_name = space_mgr.entity_label(entity_id).map(str::to_owned);
    let entity_name = entity_name.as_deref();
    let names = cimmeria_names::book();
    if !is_gm(access_level) {
        // Visible on purpose: "the Debug Area DHD only lists my own
        // addresses" from a tester whose account lost its GM flag should be
        // one log row, not a mystery.
        tracing::info!(
            entity_id,
            entity_name,
            player_id,
            player_name,
            access_level,
            hub_stargate_id,
            hub_stargate_name = names.stargate(hub_stargate_id),
            world = %hub_world,
            trigger = trigger.label(),
            reason = "dial_hub_not_gm",
            "debug dial hub: non-GM caller — no address top-up; the DHD offers only the caller's own address book"
        );
        return None;
    }

    let mut offered: Vec<(i32, String)> = Vec::new();
    let mut unenterable: Vec<(i32, String)> = Vec::new();
    let mut excluded: Vec<(i32, String)> = Vec::new();
    for (&id, gate) in &space_mgr.stargates {
        if gate.debug_dial_hub || gate.world_name == hub_world {
            continue;
        }
        let row = (id, gate.world_name.clone());
        if HUB_EXCLUDED_GATES.iter().any(|(x, _)| *x == id) {
            excluded.push(row);
        } else if space_mgr.world_is_enterable(&gate.world_name) {
            offered.push(row);
        } else {
            unenterable.push(row);
        }
    }
    offered.sort_unstable_by_key(|(id, _)| *id);
    unenterable.sort_unstable_by_key(|(id, _)| *id);
    excluded.sort_unstable_by_key(|(id, _)| *id);

    // `29:Debug Area@DebugArea`; the name is left out when the book has none.
    let label = |id: i32, world: &str| match names.stargate(id) {
        Some(name) => format!("{id}:{name}@{world}"),
        None => format!("{id}@{world}"),
    };

    let entity = space_mgr.get_entity_mut(entity_id)?;
    let mut result = HubTopUp {
        unenterable: unenterable.iter().map(|(id, _)| *id).collect(),
        excluded: excluded.iter().map(|(id, _)| *id).collect(),
        ..HubTopUp::default()
    };
    let mut granted_labels: Vec<String> = Vec::new();
    for (id, world) in &offered {
        if entity.known_stargates.contains(id) {
            result.already_known += 1;
            continue;
        }
        entity.known_stargates.push(*id);
        result.granted.push(*id);
        granted_labels.push(label(*id, world));
    }

    // Audit line, shaped like gmDHD's `gm_address_grant`: a GM was handed
    // addresses they did not earn. One row per pass that runs (world entry,
    // DHD open) with the whole list, so a SigNoz query on `reason` answers
    // "who could dial what, when".
    tracing::warn!(
        entity_id,
        entity_name,
        player_id,
        player_name,
        access_level,
        hub_stargate_id,
        hub_stargate_name = names.stargate(hub_stargate_id),
        world = %hub_world,
        granted_count = result.granted.len(),
        already_known = result.already_known,
        granted = %granted_labels.join(", "),
        unenterable_count = unenterable.len(),
        unenterable = %unenterable
            .iter()
            .map(|(id, world)| label(*id, world))
            .collect::<Vec<_>>()
            .join(", "),
        excluded = %excluded
            .iter()
            .map(|(id, world)| {
                let why = HUB_EXCLUDED_GATES
                    .iter()
                    .find(|(x, _)| x == id)
                    .map_or("", |(_, why)| why);
                format!("{} ({why})", label(*id, world))
            })
            .collect::<Vec<_>>()
            .join(", "),
        trigger = trigger.label(),
        reason = "gm_dial_hub_grant",
        "debug dial hub: granting a GM every enterable gate for this session (in memory only); gates on worlds this server cannot load are left out"
    );

    // Legs 1 (the cell's book, above) and 2 (the client's). On the DHD-open
    // pass these go out before `onDisplayDHD` on the same channel; the
    // world-entry pass is what gives the client time to resolve them (see
    // the module doc).
    //
    // The world-entry pass runs inside `InitPlayerState`'s burst, so its
    // ~11 updates ride one `EntityMethodCallBatch` (one packet), the way the
    // region hints do since PR #410: separate reliable packets during world
    // entry fed the #408 freeze.
    if trigger == HubTopUpTrigger::WorldEntry {
        if !result.granted.is_empty() {
            let calls = result
                .granted
                .iter()
                .map(|&id| (UPDATE_STARGATE_ADDRESS, update_stargate_address_args(id)))
                .collect();
            if let Err(e) = tx
                .send(CellToBaseMsg::EntityMethodCallBatch { entity_id, calls })
                .await
            {
                tracing::warn!(
                    entity_id,
                    entity_name,
                    player_id,
                    player_name,
                    granted_count = result.granted.len(),
                    reason = "dial_hub_notify_send_failed",
                    "debug dial hub: the updateStargateAddress batch could not be enqueued ({e}) — the server will accept dials to these gates but the DHD will not list them"
                );
            }
        }
        return Some(result);
    }
    for &id in &result.granted {
        if let Err(e) = tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: UPDATE_STARGATE_ADDRESS,
                args: update_stargate_address_args(id),
            })
            .await
        {
            tracing::warn!(
                entity_id,
                entity_name,
                player_id,
                player_name,
                stargate_id = id,
                stargate_name = names.stargate(id),
                reason = "dial_hub_notify_send_failed",
                "debug dial hub: updateStargateAddress could not be enqueued ({e}) — the server will accept a dial to this gate but the DHD will not list it"
            );
            break;
        }
    }

    Some(result)
}
