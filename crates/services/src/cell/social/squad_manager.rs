//! In-memory squad (ephemeral organization) state for a single CellApp space.
//!
//! Squads are `OrgType::Squad` — not persisted.  The whole state lives here and
//! is discarded when the space tears down.  All methods are synchronous; the
//! cell message loop is single-threaded per space so there is no contention.
//!
//! # State model
//!
//! ```text
//! SquadManager
//!   └── squads: HashMap<i32 (org_id), Squad>
//!         ├── leader_entity_id: u32
//!         ├── loot_mode: i32          (EGroupLootType — unconfirmed raw i32)
//!         └── members: Vec<SquadMember>
//!               ├── entity_id: u32
//!               ├── player_id: i32
//!               ├── name: String
//!               └── rank: u8          (OrgRank::Leader=8 or OrgRank::Member=2)
//! ```
//!
//! # Pending invites
//!
//! Before both sides agree, an invite is tracked in `pending_invites`:
//! `request_id → PendingInvite`.  Accepts/declines are matched by `request_id`.
//!
//! # squad CELL_PUBLIC property
//!
//! The `squad` property (INT32, CELL_PUBLIC on OrganizationMember) must be set
//! on the CellEntity so AoI observers can see squad membership.  Callers are
//! responsible for writing `entity.properties["squad"] = PropertyValue::Int(org_id)`
//! after `join` and `PropertyValue::Int(0)` after `leave`.
//!
//! # EGroupLootType
//!
//! Values unconfirmed from binary — using raw i32.  Needs x64dbg/playtest.
//! Common pattern: 0=FreeForAll, 1=RoundRobin, 2=MasterLooter, 3=NeedBeforeGreed.

use std::collections::HashMap;

use cimmeria_game::social::guilds::OrgRank;

/// Maximum squad size.  SGW allowed 6 members (TargetSquadMember1..6 in the
/// binary at 0x0184094c–0x018409ec).
pub const MAX_SQUAD_SIZE: usize = 6;

/// EReasons values for `onOrganizationLeft` / `onMemberLeftOrganization`.
///
/// UNCONFIRMED — x64dbg verification needed.  Using Left=1 as working value.
pub mod e_reasons {
    pub const DISBANDED: u8 = 0;
    pub const LEFT: u8 = 1;
    pub const KICKED: u8 = 2;
}

/// A single squad member record, sent in roster packets.
#[derive(Debug, Clone)]
pub struct SquadMember {
    pub entity_id: u32,
    pub player_id: i32,
    pub name: String,
    /// OrgRank::Leader (8) or OrgRank::Member (2).
    pub rank: u8,
}

/// Roster entry for `onOrganizationRosterInfo` [38].
///
/// `level` and `archetype` are populated from `CellEntity` fields at call
/// time.  Notes are always empty for ephemeral squads.
#[derive(Debug, Clone)]
pub struct RosterEntry {
    pub name: String,
    pub level: u8,
    pub archetype: u8,
    pub rank: u8,
    pub note: String,
    pub officer_note: String,
}

impl RosterEntry {
    pub fn from_member(m: &SquadMember, level: u8, archetype: u8) -> Self {
        Self {
            name: m.name.clone(),
            level,
            archetype,
            rank: m.rank,
            note: String::new(),
            officer_note: String::new(),
        }
    }
}

/// A pending squad invite that has been sent but not yet answered.
#[derive(Debug)]
pub struct PendingInvite {
    pub request_id: i32,
    /// Entity that sent the invite.
    pub inviter_entity_id: u32,
    pub inviter_name: String,
    /// Entity the invite was sent to.
    pub target_entity_id: u32,
    /// Squad this would join (or 0 if a new squad will be created on accept).
    pub org_id: i32,
    pub org_name: String,
}

/// Active in-memory squad.
#[derive(Debug)]
pub struct Squad {
    pub org_id: i32,
    pub name: String,
    /// Leader is always the first member; rank = OrgRank::Leader (8).
    pub leader_entity_id: u32,
    pub loot_mode: i32,
    members: Vec<SquadMember>,
}

impl Squad {
    fn new(org_id: i32, name: String, leader: SquadMember) -> Self {
        let leader_entity_id = leader.entity_id;
        Self {
            org_id,
            name,
            leader_entity_id,
            loot_mode: 0,
            members: vec![leader],
        }
    }

    /// All members (immutable).
    pub fn members(&self) -> &[SquadMember] {
        &self.members
    }

    /// Entity IDs of all members.
    pub fn member_entity_ids(&self) -> Vec<u32> {
        self.members.iter().map(|m| m.entity_id).collect()
    }

    /// Check whether an entity is already in this squad.
    pub fn contains_entity(&self, entity_id: u32) -> bool {
        self.members.iter().any(|m| m.entity_id == entity_id)
    }

    /// Add a new member (non-leader, rank = Member = 2).
    ///
    /// Returns `Err` if the squad is at capacity.
    pub fn add_member(&mut self, member: SquadMember) -> Result<(), &'static str> {
        if self.members.len() >= MAX_SQUAD_SIZE {
            return Err("squad is full");
        }
        self.members.push(member);
        Ok(())
    }

    /// Remove a member by entity_id.  Returns the removed member, or `None` if
    /// not found.  Does NOT handle leader auto-promotion — that is done by
    /// `SquadManager::remove_member`.
    fn remove_member_inner(&mut self, entity_id: u32) -> Option<SquadMember> {
        if let Some(pos) = self.members.iter().position(|m| m.entity_id == entity_id) {
            Some(self.members.remove(pos))
        } else {
            None
        }
    }

    /// Promote the oldest non-leader member to leader.  Called after the
    /// current leader leaves.  Returns `None` if the squad is now empty.
    fn promote_next_leader(&mut self) -> Option<u32> {
        if self.members.is_empty() {
            return None;
        }
        let next = &mut self.members[0];
        next.rank = OrgRank::Leader as u8;
        let new_leader_id = next.entity_id;
        self.leader_entity_id = new_leader_id;
        Some(new_leader_id)
    }
}

/// Per-space squad state.  One instance lives on the `SpaceManager` (or passed
/// in to cell-method handlers).
#[derive(Debug, Default)]
pub struct SquadManager {
    squads: HashMap<i32, Squad>,
    /// Map from entity_id → org_id for O(1) "which squad am I in?" lookups.
    entity_squad: HashMap<u32, i32>,
    /// Pending invites keyed by `request_id`.
    pending_invites: HashMap<i32, PendingInvite>,
    /// Monotonically increasing counter for generating unique request IDs and
    /// org IDs within a space.
    next_id: i32,
}

/// Result of a successful `accept_invite`.
#[derive(Debug)]
pub struct AcceptResult {
    pub org_id: i32,
    pub new_member_entity_id: u32,
    pub new_member_player_id: i32,
    pub new_member_name: String,
    pub new_member_rank: u8,
    pub existing_member_entity_ids: Vec<u32>,
}

/// Result of `remove_member`.
#[derive(Debug)]
pub struct RemoveResult {
    pub org_id: i32,
    pub removed_entity_id: u32,
    pub removed_player_id: i32,
    pub removed_name: String,
    /// Whether the squad still has members after removal.
    pub squad_alive: bool,
    /// Entity IDs of all remaining members (empty = squad disbanded).
    pub remaining_entity_ids: Vec<u32>,
    /// New leader after auto-promotion, if the leader was the one who left.
    pub new_leader_entity_id: Option<u32>,
}

impl SquadManager {
    pub fn new() -> Self {
        Self::default()
    }

    fn alloc_id(&mut self) -> i32 {
        self.next_id += 1;
        self.next_id
    }

    // ── Invite lifecycle ──────────────────────────────────────────────────────

    /// Record a pending invite and return the allocated `request_id`.
    ///
    /// `org_id` should be 0 when the inviter is not yet in a squad (a new
    /// squad will be created on accept).
    pub fn record_invite(
        &mut self,
        inviter_entity_id: u32,
        inviter_name: String,
        target_entity_id: u32,
        org_id: i32,
        org_name: String,
    ) -> i32 {
        let request_id = self.alloc_id();
        self.pending_invites.insert(
            request_id,
            PendingInvite {
                request_id,
                inviter_entity_id,
                inviter_name,
                target_entity_id,
                org_id,
                org_name,
            },
        );
        request_id
    }

    /// Look up a pending invite by request_id without consuming it.
    pub fn get_invite(&self, request_id: i32) -> Option<&PendingInvite> {
        self.pending_invites.get(&request_id)
    }

    /// Cancel / expire a pending invite.
    pub fn cancel_invite(&mut self, request_id: i32) {
        self.pending_invites.remove(&request_id);
    }

    // ── Squad membership ──────────────────────────────────────────────────────

    /// Accept a pending invite.  Creates the squad if needed.
    ///
    /// The `target_player_id` and `target_name` come from the cell's entity
    /// state at accept time (not stored in the invite to avoid stale copies).
    ///
    /// Returns `Err` if the invite does not exist, the target is already in a
    /// squad, or the squad is full.
    pub fn accept_invite(
        &mut self,
        request_id: i32,
        target_entity_id: u32,
        target_player_id: i32,
        target_name: String,
        // Injected so callers (tests) can supply player_id/name for the inviter.
        inviter_player_id: i32,
    ) -> Result<AcceptResult, &'static str> {
        let invite = self
            .pending_invites
            .remove(&request_id)
            .ok_or("invite not found")?;

        if self.entity_squad.contains_key(&target_entity_id) {
            return Err("target already in a squad");
        }

        let new_member = SquadMember {
            entity_id: target_entity_id,
            player_id: target_player_id,
            name: target_name.clone(),
            rank: OrgRank::Member as u8,
        };

        if invite.org_id == 0 {
            // Inviter is not yet in a squad — create one now.
            if self.entity_squad.contains_key(&invite.inviter_entity_id) {
                return Err("inviter joined a different squad while invite was pending");
            }
            let org_id = self.alloc_id();
            let leader = SquadMember {
                entity_id: invite.inviter_entity_id,
                player_id: inviter_player_id,
                name: invite.inviter_name.clone(),
                rank: OrgRank::Leader as u8,
            };
            let existing_ids = vec![invite.inviter_entity_id];
            let mut squad = Squad::new(org_id, invite.org_name.clone(), leader);
            squad.add_member(new_member)?;
            self.entity_squad.insert(invite.inviter_entity_id, org_id);
            self.entity_squad.insert(target_entity_id, org_id);
            self.squads.insert(org_id, squad);

            Ok(AcceptResult {
                org_id,
                new_member_entity_id: target_entity_id,
                new_member_player_id: target_player_id,
                new_member_name: target_name,
                new_member_rank: OrgRank::Member as u8,
                existing_member_entity_ids: existing_ids,
            })
        } else {
            // Join an existing squad.
            let squad = self
                .squads
                .get_mut(&invite.org_id)
                .ok_or("squad no longer exists")?;
            let existing_ids: Vec<u32> = squad.member_entity_ids();
            squad.add_member(new_member)?;
            self.entity_squad.insert(target_entity_id, invite.org_id);

            Ok(AcceptResult {
                org_id: invite.org_id,
                new_member_entity_id: target_entity_id,
                new_member_player_id: target_player_id,
                new_member_name: target_name,
                new_member_rank: OrgRank::Member as u8,
                existing_member_entity_ids: existing_ids,
            })
        }
    }

    /// Remove a member from their squad (leave or kick).
    ///
    /// Handles leader auto-promotion and disbands the squad when it becomes
    /// empty.
    ///
    /// Returns `Err` if the entity is not in any squad.
    pub fn remove_member(
        &mut self,
        entity_id: u32,
        player_id: i32,
        name: String,
    ) -> Result<RemoveResult, &'static str> {
        let org_id = *self
            .entity_squad
            .get(&entity_id)
            .ok_or("entity not in a squad")?;

        let squad = self.squads.get_mut(&org_id).ok_or("squad not found")?;

        let was_leader = squad.leader_entity_id == entity_id;
        squad
            .remove_member_inner(entity_id)
            .ok_or("member not in squad members list")?;
        self.entity_squad.remove(&entity_id);

        if squad.members.is_empty() {
            // Squad dissolved.
            self.squads.remove(&org_id);
            return Ok(RemoveResult {
                org_id,
                removed_entity_id: entity_id,
                removed_player_id: player_id,
                removed_name: name,
                squad_alive: false,
                remaining_entity_ids: Vec::new(),
                new_leader_entity_id: None,
            });
        }

        let new_leader = if was_leader {
            // Promote oldest remaining member.
            self.squads
                .get_mut(&org_id)
                .and_then(|s| s.promote_next_leader())
        } else {
            None
        };

        let remaining = self
            .squads
            .get(&org_id)
            .map(|s| s.member_entity_ids())
            .unwrap_or_default();

        Ok(RemoveResult {
            org_id,
            removed_entity_id: entity_id,
            removed_player_id: player_id,
            removed_name: name,
            squad_alive: true,
            remaining_entity_ids: remaining,
            new_leader_entity_id: new_leader,
        })
    }

    /// Return the org_id for an entity, if any.
    pub fn squad_of(&self, entity_id: u32) -> Option<i32> {
        self.entity_squad.get(&entity_id).copied()
    }

    /// Return an immutable reference to a squad.
    pub fn get_squad(&self, org_id: i32) -> Option<&Squad> {
        self.squads.get(&org_id)
    }

    /// Set the loot mode for a squad.  Returns the member entity IDs for
    /// fanout, or `None` if the squad does not exist.
    pub fn set_loot_mode(&mut self, entity_id: u32, loot_mode: i32) -> Option<(i32, Vec<u32>)> {
        let org_id = *self.entity_squad.get(&entity_id)?;
        let squad = self.squads.get_mut(&org_id)?;
        squad.loot_mode = loot_mode;
        Some((org_id, squad.member_entity_ids()))
    }

    /// Build a full roster snapshot for `onOrganizationRosterInfo`.
    ///
    /// `level_fn` and `archetype_fn` are callbacks to pull live data from the
    /// space's entity store without coupling this module to `SpaceManager`.
    pub fn build_roster<F>(&self, org_id: i32, mut entity_info: F) -> Vec<RosterEntry>
    where
        F: FnMut(u32) -> (u8, u8), // (level, archetype_id)
    {
        let Some(squad) = self.squads.get(&org_id) else {
            return Vec::new();
        };
        squad
            .members()
            .iter()
            .map(|m| {
                let (level, archetype) = entity_info(m.entity_id);
                RosterEntry::from_member(m, level, archetype)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_mgr() -> SquadManager {
        SquadManager::new()
    }

    // ── record_invite / get_invite / cancel_invite ────────────────────────────

    #[test]
    fn record_and_retrieve_invite() {
        let mut mgr = make_mgr();
        let rid = mgr.record_invite(10, "Alice".into(), 20, 0, "Alpha Squad".into());
        let inv = mgr.get_invite(rid).expect("invite must exist");
        assert_eq!(inv.inviter_entity_id, 10);
        assert_eq!(inv.target_entity_id, 20);
        assert_eq!(inv.org_id, 0);
    }

    #[test]
    fn cancel_invite_removes_it() {
        let mut mgr = make_mgr();
        let rid = mgr.record_invite(10, "Alice".into(), 20, 0, "Alpha".into());
        mgr.cancel_invite(rid);
        assert!(mgr.get_invite(rid).is_none());
    }

    // ── accept_invite creates squad ───────────────────────────────────────────

    #[test]
    fn accept_creates_squad_with_two_members() {
        let mut mgr = make_mgr();
        let rid = mgr.record_invite(10, "Alice".into(), 20, 0, "Alpha".into());
        let res = mgr
            .accept_invite(rid, 20, 200, "Bob".into(), 100)
            .expect("accept must succeed");

        assert!(res.org_id > 0);
        assert_eq!(res.new_member_entity_id, 20);
        assert_eq!(res.existing_member_entity_ids, vec![10]);

        let squad = mgr.get_squad(res.org_id).expect("squad must exist");
        assert_eq!(squad.members().len(), 2);
        assert_eq!(squad.leader_entity_id, 10);
        assert_eq!(mgr.squad_of(10), Some(res.org_id));
        assert_eq!(mgr.squad_of(20), Some(res.org_id));
    }

    #[test]
    fn accept_nonexistent_invite_is_err() {
        let mut mgr = make_mgr();
        assert!(mgr.accept_invite(999, 20, 200, "Bob".into(), 100).is_err());
    }

    #[test]
    fn accept_duplicate_target_is_err() {
        let mut mgr = make_mgr();
        let rid1 = mgr.record_invite(10, "Alice".into(), 20, 0, "Alpha".into());
        mgr.accept_invite(rid1, 20, 200, "Bob".into(), 100).unwrap();
        // Bob is already in a squad; second accept must fail.
        let rid2 = mgr.record_invite(30, "Carol".into(), 20, 0, "Beta".into());
        assert!(mgr.accept_invite(rid2, 20, 200, "Bob".into(), 300).is_err());
    }

    // ── remove_member / auto-promotion / disband ──────────────────────────────

    #[test]
    fn non_leader_leave_does_not_promote() {
        let mut mgr = make_mgr();
        let rid = mgr.record_invite(10, "Alice".into(), 20, 0, "Alpha".into());
        let accept = mgr.accept_invite(rid, 20, 200, "Bob".into(), 100).unwrap();

        let res = mgr
            .remove_member(20, 200, "Bob".into())
            .expect("remove must succeed");
        assert_eq!(res.org_id, accept.org_id);
        assert!(res.squad_alive);
        assert!(res.new_leader_entity_id.is_none());
        // Bob is gone; Alice is sole remaining member.
        assert_eq!(res.remaining_entity_ids, vec![10]);
        assert!(mgr.squad_of(20).is_none());
    }

    #[test]
    fn leader_leave_auto_promotes_next_member() {
        let mut mgr = make_mgr();
        let rid = mgr.record_invite(10, "Alice".into(), 20, 0, "Alpha".into());
        let accept = mgr.accept_invite(rid, 20, 200, "Bob".into(), 100).unwrap();

        // Alice (entity 10, leader) leaves.
        let res = mgr
            .remove_member(10, 100, "Alice".into())
            .expect("remove must succeed");
        assert!(res.squad_alive);
        assert_eq!(res.new_leader_entity_id, Some(20));

        let squad = mgr.get_squad(accept.org_id).unwrap();
        assert_eq!(squad.leader_entity_id, 20);
        assert_eq!(squad.members()[0].rank, OrgRank::Leader as u8);
    }

    #[test]
    fn last_member_leave_disbands_squad() {
        let mut mgr = make_mgr();
        let rid = mgr.record_invite(10, "Alice".into(), 20, 0, "Alpha".into());
        let accept = mgr.accept_invite(rid, 20, 200, "Bob".into(), 100).unwrap();

        mgr.remove_member(20, 200, "Bob".into()).unwrap();
        let res = mgr
            .remove_member(10, 100, "Alice".into())
            .expect("leader leave must succeed");

        assert!(!res.squad_alive);
        assert!(res.remaining_entity_ids.is_empty());
        assert!(mgr.get_squad(accept.org_id).is_none());
    }

    // ── loot mode ────────────────────────────────────────────────────────────

    #[test]
    fn set_loot_mode_fans_out_member_ids() {
        let mut mgr = make_mgr();
        let rid = mgr.record_invite(10, "Alice".into(), 20, 0, "Alpha".into());
        let accept = mgr.accept_invite(rid, 20, 200, "Bob".into(), 100).unwrap();

        let result = mgr.set_loot_mode(10, 2);
        let (org_id, ids) = result.expect("set_loot_mode must return Some");
        assert_eq!(org_id, accept.org_id);
        assert!(ids.contains(&10));
        assert!(ids.contains(&20));

        // Confirm stored.
        let squad = mgr.get_squad(org_id).unwrap();
        assert_eq!(squad.loot_mode, 2);
    }

    #[test]
    fn set_loot_mode_for_nonmember_returns_none() {
        let mut mgr = make_mgr();
        assert!(mgr.set_loot_mode(99, 1).is_none());
    }

    // ── squad_of ─────────────────────────────────────────────────────────────

    #[test]
    fn squad_of_unknown_entity_is_none() {
        let mgr = make_mgr();
        assert_eq!(mgr.squad_of(42), None);
    }
}
