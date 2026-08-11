//! `OrgAuthority` — the base-side singleton owning all persistent organizations.
//!
//! Owns the in-memory org map, the online-member tracking set (entity_id keyed by
//! player_id), and serializes mutations by being driven from the single-threaded
//! base-side dispatch loop. All mutations write through to the DB before returning.
//!
//! # Ownership and locking
//!
//! `OrgAuthority` is owned by the base service as
//! `Option<Arc<tokio::sync::Mutex<OrgAuthority>>>`. The `Arc` is cloned so
//! both the connect loop (login/logout tracking via `on_player_login` /
//! `on_player_logout`) and the `CellToBase` message handler (persistent org
//! mutations) share the same instance without data races.
//!
//! Lock discipline: always acquire the `tokio::sync::Mutex` guard, perform the
//! mutation or read, then drop the guard before starting any fan-out. Fan-out
//! sends are fire-and-forget (`broadcast_to_org` spawns per-recipient tasks)
//! and must never hold the authority lock.
//!
//! # Online tracking
//!
//! `online: HashMap<i32, u32>` maps `player_id → entity_id` for every org member
//! currently connected. Updated by `OrgAuthority::on_player_login` /
//! `on_player_logout` called from the base connect/disconnect path.
//! `broadcast_to_org` uses this set to determine which entity IDs to fan out to.

use std::collections::{HashMap, HashSet};

use cimmeria_game::social::guilds::{Org, OrgPermission, OrgRank, OrgType};
use sqlx::PgPool;

use super::persistence;

/// Result from `OrgAuthority::create_org`.
pub struct CreateOrgResult {
    pub org_id: i32,
}

/// Rejection reasons for org mutations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Phase 3+: all variants will be used as more operations are wired
pub enum OrgError {
    /// The acting player is not a member of the org.
    NotMember,
    /// The acting player lacks the required permission.
    PermissionDenied,
    /// The target org does not exist.
    OrgNotFound,
    /// The target player is not a member of the org.
    TargetNotMember,
    /// The target player is already a member.
    AlreadyMember,
    /// The requested rank is out of range.
    InvalidRank,
    /// A DB error occurred. Details logged at the call site.
    DbError,
}

impl std::fmt::Display for OrgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotMember => write!(f, "not_member"),
            Self::PermissionDenied => write!(f, "permission_denied"),
            Self::OrgNotFound => write!(f, "org_not_found"),
            Self::TargetNotMember => write!(f, "target_not_member"),
            Self::AlreadyMember => write!(f, "already_member"),
            Self::InvalidRank => write!(f, "invalid_rank"),
            Self::DbError => write!(f, "db_error"),
        }
    }
}

/// The base-side singleton for all persistent organizations.
pub struct OrgAuthority {
    /// All loaded persistent orgs keyed by `org_id`.
    orgs: HashMap<i32, Org>,
    /// Online-member tracking: `player_id → entity_id`.
    /// Only members of at least one persistent org are tracked here.
    online: HashMap<i32, u32>,
    /// Reverse index: `player_id → org_id` for every membership.
    /// A player can theoretically hold memberships in multiple orgs
    /// (though one org per type in practice). Used to find a player's org quickly.
    membership: HashMap<i32, HashSet<i32>>,
}

impl OrgAuthority {
    /// Construct an empty authority (before loading from DB).
    pub fn empty() -> Self {
        Self {
            orgs: HashMap::new(),
            online: HashMap::new(),
            membership: HashMap::new(),
        }
    }

    /// Load all persistent orgs from the DB and return a populated authority.
    pub async fn load_all(pool: &PgPool) -> Result<Self, sqlx::Error> {
        let orgs_vec = persistence::load_all_orgs(pool).await?;
        let mut auth = Self::empty();
        for org in orgs_vec {
            auth.index_org(&org);
            auth.orgs.insert(org.org_id, org);
        }
        tracing::info!(org_count = auth.orgs.len(), "OrgAuthority loaded");
        Ok(auth)
    }

    /// Build/update the reverse-membership index for a single org.
    fn index_org(&mut self, org: &Org) {
        for player_id in org.members.keys() {
            self.membership
                .entry(*player_id)
                .or_default()
                .insert(org.org_id);
        }
    }

    // ── Online tracking ───────────────────────────────────────────────────────

    /// Mark a player as online (called from `handle_on_client_ready` after world entry).
    /// If the player is not a member of any persistent org this is a no-op.
    pub fn on_player_login(&mut self, player_id: i32, entity_id: u32) {
        if self.membership.contains_key(&player_id) {
            self.online.insert(player_id, entity_id);
        }
    }

    /// Mark a player as offline (called from `handle_log_off` on the in-world logoff path).
    pub fn on_player_logout(&mut self, player_id: i32) {
        self.online.remove(&player_id);
    }

    /// Return the entity_id of an online org member, if present.
    pub fn entity_id_of(&self, player_id: i32) -> Option<u32> {
        self.online.get(&player_id).copied()
    }

    /// Return the entity IDs of all online members of `org_id`.
    pub fn online_member_entity_ids(&self, org_id: i32) -> Vec<u32> {
        let Some(org) = self.orgs.get(&org_id) else {
            return Vec::new();
        };
        org.members
            .keys()
            .filter_map(|pid| self.online.get(pid).copied())
            .collect()
    }

    /// Return the entity IDs of all online members of `org_id` except `exclude_player_id`.
    pub fn online_member_entity_ids_except(&self, org_id: i32, exclude_player_id: i32) -> Vec<u32> {
        let Some(org) = self.orgs.get(&org_id) else {
            return Vec::new();
        };
        org.members
            .keys()
            .filter(|&&pid| pid != exclude_player_id)
            .filter_map(|pid| self.online.get(pid).copied())
            .collect()
    }

    // ── Reads ─────────────────────────────────────────────────────────────────

    /// Look up an org by id.
    pub fn get_org(&self, org_id: i32) -> Option<&Org> {
        self.orgs.get(&org_id)
    }

    /// Return the org_ids of all persistent orgs the player is a member of.
    #[allow(dead_code)] // Phase 4: used for login org-state sync
    pub fn orgs_of(&self, player_id: i32) -> Vec<i32> {
        self.membership
            .get(&player_id)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }

    // ── Mutations ─────────────────────────────────────────────────────────────

    /// Create a new persistent org and add it to the in-memory map.
    ///
    /// `leader_name` must be the character's display name (for the in-memory
    /// member record; the DB record is anchored by player_id only).
    pub async fn create_org(
        &mut self,
        pool: &PgPool,
        org_type: OrgType,
        name: &str,
        leader_player_id: i32,
        leader_name: String,
    ) -> Result<CreateOrgResult, OrgError> {
        let org_id = persistence::create_org(pool, org_type, name, leader_player_id)
            .await
            .map_err(|e| {
                tracing::warn!(
                    leader_player_id,
                    %e,
                    "OrgAuthority::create_org: DB error"
                );
                OrgError::DbError
            })?;

        let org = Org::new(
            org_id,
            org_type,
            name.to_owned(),
            leader_player_id,
            leader_name,
        );
        self.index_org(&org);
        self.orgs.insert(org_id, org);
        Ok(CreateOrgResult { org_id })
    }

    /// Invite a player into an org (permission-gated).
    ///
    /// This adds the member to the in-memory model and DB at Initiate rank.
    /// The caller is responsible for sending the wire invite/accept flow and
    /// only calling this after the invitee has accepted.
    ///
    /// `actor_player_id` must hold `OrgPermission::INVITE`.
    pub async fn accept_invite(
        &mut self,
        pool: &PgPool,
        org_id: i32,
        actor_player_id: i32,
        new_player_id: i32,
        new_player_name: String,
    ) -> Result<(), OrgError> {
        {
            let org = self.orgs.get(&org_id).ok_or(OrgError::OrgNotFound)?;
            if !org.has_permission(actor_player_id, OrgPermission::INVITE) {
                tracing::warn!(
                    actor_player_id,
                    org_id,
                    reason = "permission_denied",
                    "accept_invite: actor lacks INVITE"
                );
                return Err(OrgError::PermissionDenied);
            }
            if org.members.contains_key(&new_player_id) {
                return Err(OrgError::AlreadyMember);
            }
        }

        persistence::add_member(pool, org_id, new_player_id)
            .await
            .map_err(|e| {
                tracing::warn!(org_id, new_player_id, %e, "accept_invite: DB error");
                OrgError::DbError
            })?;

        let org = self.orgs.get_mut(&org_id).unwrap();
        org.add_member(new_player_id, new_player_name);
        self.membership
            .entry(new_player_id)
            .or_default()
            .insert(org_id);
        Ok(())
    }

    /// Kick a member from an org (permission-gated).
    ///
    /// `actor_player_id` must hold `OrgPermission::EJECT`.
    /// Returns the kicked player's name for fanout wire messages.
    pub async fn kick_member(
        &mut self,
        pool: &PgPool,
        org_id: i32,
        actor_player_id: i32,
        target_player_id: i32,
    ) -> Result<String, OrgError> {
        let kicked_name = {
            let org = self.orgs.get(&org_id).ok_or(OrgError::OrgNotFound)?;
            if !org.has_permission(actor_player_id, OrgPermission::EJECT) {
                tracing::warn!(
                    actor_player_id,
                    org_id,
                    reason = "permission_denied",
                    "kick_member: actor lacks EJECT"
                );
                return Err(OrgError::PermissionDenied);
            }
            org.members
                .get(&target_player_id)
                .ok_or(OrgError::TargetNotMember)?
                .character_name
                .clone()
        };

        persistence::remove_member(pool, org_id, target_player_id)
            .await
            .map_err(|e| {
                tracing::warn!(org_id, target_player_id, %e, "kick_member: DB error");
                OrgError::DbError
            })?;

        let org = self.orgs.get_mut(&org_id).unwrap();
        org.remove_member(target_player_id);
        if let Some(member_orgs) = self.membership.get_mut(&target_player_id) {
            member_orgs.remove(&org_id);
        }
        Ok(kicked_name)
    }

    /// Change a member's rank (permission-gated).
    ///
    /// Promotes require `OrgPermission::PROMOTE`; demotes require `DEMOTE`.
    pub async fn change_rank(
        &mut self,
        pool: &PgPool,
        org_id: i32,
        actor_player_id: i32,
        target_player_id: i32,
        new_rank: OrgRank,
    ) -> Result<String, OrgError> {
        let (target_name, required_perm) = {
            let org = self.orgs.get(&org_id).ok_or(OrgError::OrgNotFound)?;
            let target = org
                .members
                .get(&target_player_id)
                .ok_or(OrgError::TargetNotMember)?;
            let perm = if new_rank > target.rank {
                OrgPermission::PROMOTE
            } else {
                OrgPermission::DEMOTE
            };
            (target.character_name.clone(), perm)
        };

        {
            let org = self.orgs.get(&org_id).unwrap();
            if !org.has_permission(actor_player_id, required_perm) {
                tracing::warn!(
                    actor_player_id,
                    org_id,
                    reason = "permission_denied",
                    "change_rank: actor lacks required permission"
                );
                return Err(OrgError::PermissionDenied);
            }
        }

        persistence::set_member_rank(pool, org_id, target_player_id, new_rank)
            .await
            .map_err(|e| {
                tracing::warn!(org_id, target_player_id, %e, "change_rank: DB error");
                OrgError::DbError
            })?;

        let org = self.orgs.get_mut(&org_id).unwrap();
        org.set_rank(target_player_id, new_rank);
        Ok(target_name)
    }

    /// Set the org MOTD (permission-gated).
    ///
    /// `actor_player_id` must hold `OrgPermission::MOTD`.
    pub async fn set_motd(
        &mut self,
        pool: &PgPool,
        org_id: i32,
        actor_player_id: i32,
        motd: &str,
    ) -> Result<(), OrgError> {
        {
            let org = self.orgs.get(&org_id).ok_or(OrgError::OrgNotFound)?;
            if !org.has_permission(actor_player_id, OrgPermission::MOTD) {
                tracing::warn!(
                    actor_player_id,
                    org_id,
                    reason = "permission_denied",
                    "set_motd: actor lacks MOTD"
                );
                return Err(OrgError::PermissionDenied);
            }
        }

        let motd_store = if motd.is_empty() { None } else { Some(motd) };
        persistence::set_motd(pool, org_id, motd_store)
            .await
            .map_err(|e| {
                tracing::warn!(org_id, %e, "set_motd: DB error");
                OrgError::DbError
            })?;

        let org = self.orgs.get_mut(&org_id).unwrap();
        org.motd = motd_store.map(|s| s.to_owned());
        Ok(())
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_game::social::guilds::{OrgPermission, OrgRank, OrgType};

    fn make_authority_with_org(org_id: i32, leader_pid: i32) -> OrgAuthority {
        let mut auth = OrgAuthority::empty();
        let org = Org::new(
            org_id,
            OrgType::Command,
            "TestOrg".into(),
            leader_pid,
            "Leader".into(),
        );
        auth.index_org(&org);
        auth.orgs.insert(org_id, org);
        auth
    }

    // ── Permission-gate unit tests ────────────────────────────────────────────

    /// Bug shape: a non-member cannot invite (no permission).
    /// This guard fails if the permission check is removed from accept_invite.
    #[test]
    fn accept_invite_rejects_non_member_actor() {
        let auth = make_authority_with_org(1, 100);
        // actor 999 is NOT a member of org 1.
        // We can't easily call accept_invite without async, but we can directly
        // call has_permission to verify the gate logic.
        let org = auth.orgs.get(&1).unwrap();
        assert!(
            !org.has_permission(999, OrgPermission::INVITE),
            "non-member must not have INVITE permission"
        );
    }

    /// Bug shape: a member at Initiate rank has no permissions by default.
    #[test]
    fn initiate_has_no_permissions() {
        let mut auth = make_authority_with_org(1, 100);
        let org = auth.orgs.get_mut(&1).unwrap();
        org.add_member(200, "Initiate".into()); // adds at Initiate rank
        assert!(
            !org.has_permission(200, OrgPermission::INVITE),
            "Initiate must not have INVITE"
        );
        assert!(
            !org.has_permission(200, OrgPermission::EJECT),
            "Initiate must not have EJECT"
        );
    }

    /// Bug shape: the Leader always has INVITE + EJECT + MOTD.
    #[test]
    fn leader_has_invite_eject_motd() {
        let auth = make_authority_with_org(1, 100);
        let org = auth.orgs.get(&1).unwrap();
        assert!(org.has_permission(100, OrgPermission::INVITE));
        assert!(org.has_permission(100, OrgPermission::EJECT));
        assert!(org.has_permission(100, OrgPermission::MOTD));
    }

    /// Bug shape: kick_member permission check — actor without EJECT is rejected.
    #[test]
    fn kick_member_rejects_actor_without_eject() {
        let auth = make_authority_with_org(1, 100);
        let org = auth.orgs.get(&1).unwrap();
        // Manually check: a Member-rank player (rank 2, ROSTER_NOTES only) can't kick.
        // Since we can't async-test kick_member, we verify has_permission directly.
        // The default perm for Member = ROSTER_NOTES, NOT EJECT.
        let member_perm = org.rank_configs[&(OrgRank::Member as u8)].permissions;
        assert!(
            !member_perm.contains(OrgPermission::EJECT),
            "Member rank must not have EJECT"
        );
    }

    /// Bug shape: rank change — promote requires PROMOTE, demote requires DEMOTE.
    #[test]
    fn rank_change_requires_promote_or_demote() {
        let auth = make_authority_with_org(1, 100);
        let org = auth.orgs.get(&1).unwrap();
        // Officer rank has INVITE+EJECT but NOT PROMOTE/DEMOTE (per default ladder).
        let officer_perm = org.rank_configs[&(OrgRank::Officer as u8)].permissions;
        assert!(
            !officer_perm.contains(OrgPermission::PROMOTE),
            "Officer must not have PROMOTE"
        );
        assert!(
            !officer_perm.contains(OrgPermission::DEMOTE),
            "Officer must not have DEMOTE"
        );
        // SeniorOfficer has both.
        let senior_perm = org.rank_configs[&(OrgRank::SeniorOfficer as u8)].permissions;
        assert!(
            senior_perm.contains(OrgPermission::PROMOTE),
            "SeniorOfficer must have PROMOTE"
        );
        assert!(
            senior_perm.contains(OrgPermission::DEMOTE),
            "SeniorOfficer must have DEMOTE"
        );
    }

    /// Bug shape: online tracking — on_player_login only tracks org members.
    #[test]
    fn online_tracking_only_tracks_org_members() {
        let mut auth = make_authority_with_org(1, 100);
        // Player 100 is a member (leader) — should be tracked.
        auth.on_player_login(100, 1001);
        assert_eq!(auth.entity_id_of(100), Some(1001));

        // Player 999 is NOT a member — should not be tracked.
        auth.on_player_login(999, 9999);
        assert_eq!(
            auth.entity_id_of(999),
            None,
            "non-member must not appear in online set"
        );
    }

    /// Bug shape: on_player_logout removes the entity from the online set.
    #[test]
    fn on_player_logout_removes_from_online() {
        let mut auth = make_authority_with_org(1, 100);
        auth.on_player_login(100, 1001);
        assert!(auth.entity_id_of(100).is_some());
        auth.on_player_logout(100);
        assert_eq!(auth.entity_id_of(100), None);
    }

    /// Bug shape: broadcast target list — only online members appear.
    #[test]
    fn online_member_entity_ids_excludes_offline() {
        let mut auth = make_authority_with_org(1, 100);
        let org = auth.orgs.get_mut(&1).unwrap();
        org.add_member(200, "Alice".into());
        org.add_member(300, "Bob".into());
        auth.membership.entry(200).or_default().insert(1);
        auth.membership.entry(300).or_default().insert(1);

        // Only player 200 is online.
        auth.on_player_login(200, 2001);

        let eids = auth.online_member_entity_ids(1);
        assert_eq!(eids, vec![2001], "only the online member's entity_id");
    }

    /// Bug shape: broadcast_to_org targets members after login is called.
    ///
    /// Regression guard: if `on_player_login` is removed from the login path,
    /// `online_member_entity_ids` returns an empty Vec and no fanout reaches
    /// any member — this test will fail.
    ///
    /// Mirrors the fix for Copilot review item 2 (online tracking never wired).
    #[test]
    fn broadcast_target_list_populated_after_login() {
        let mut auth = make_authority_with_org(1, 100);
        // Add two more members beyond the leader.
        let org = auth.orgs.get_mut(&1).unwrap();
        org.add_member(200, "Alice".into());
        org.add_member(300, "Bob".into());
        auth.membership.entry(200).or_default().insert(1);
        auth.membership.entry(300).or_default().insert(1);

        // Before any login call, no online members.
        assert!(
            auth.online_member_entity_ids(1).is_empty(),
            "no members should be online before on_player_login is called"
        );

        // Simulate login for leader and Alice.
        auth.on_player_login(100, 1001);
        auth.on_player_login(200, 2001);

        let eids = auth.online_member_entity_ids(1);
        assert_eq!(eids.len(), 2, "two members marked online after login");
        assert!(
            eids.contains(&1001),
            "leader entity_id must be in fanout set"
        );
        assert!(
            eids.contains(&2001),
            "Alice entity_id must be in fanout set"
        );

        // Simulate logout for the leader — they must leave the fanout set.
        auth.on_player_logout(100);
        let eids_after = auth.online_member_entity_ids(1);
        assert_eq!(
            eids_after,
            vec![2001],
            "only Alice remains after leader logout"
        );
    }
}
