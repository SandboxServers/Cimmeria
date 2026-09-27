//! Helpers the ORG-08 handler tests share: rank masks and texts set as the
//! server, and the stored values read back.

use cimmeria_entity::organization::OrgPermission;

use super::*;
use crate::base::organization::persistence::{set_rank_permissions, set_text, OrgTextTarget};

impl Fixture {
    /// Store `mask` as `rank`'s permissions, as the server.
    pub(super) async fn set_perms_of(&self, org_id: i32, rank: OrgRank, mask: OrgPermission) {
        let mut tx = self.pool.begin().await.unwrap();
        let actor = OrgAccess::system(
            &mut tx,
            org_id,
            SystemActor::Server {
                source: "org08_test",
            },
        )
        .await
        .unwrap()
        .unwrap();
        set_rank_permissions(&mut tx, &actor, org_id, rank, mask)
            .await
            .expect("set_rank_permissions");
        tx.commit().await.unwrap();
    }

    /// Store an officer note on character `i`, as the server.
    pub(super) async fn set_officer_note_of(&self, org_id: i32, i: usize, note: &str) {
        let mut tx = self.pool.begin().await.unwrap();
        let actor = OrgAccess::system(
            &mut tx,
            org_id,
            SystemActor::Server {
                source: "org08_test",
            },
        )
        .await
        .unwrap()
        .unwrap();
        let target = OrgTextTarget::OfficerNote {
            player_id: self.player_id(i),
        };
        set_text(&mut tx, &actor, org_id, target, note)
            .await
            .expect("set_text");
        tx.commit().await.unwrap();
    }

    /// `rank`'s stored mask.
    pub(super) async fn perms_of(&self, org_id: i32, rank: OrgRank) -> OrgPermission {
        let mask: i32 = sqlx::query_scalar(
            "SELECT permissions FROM sgw_organization_ranks WHERE org_id = $1 AND rank = $2",
        )
        .bind(org_id)
        .bind(i16::from(rank.as_u8()))
        .fetch_one(&self.pool)
        .await
        .unwrap();
        OrgPermission::from_wire(mask)
    }

    /// `rank`'s stored name.
    pub(super) async fn rank_name_of(&self, org_id: i32, rank: OrgRank) -> Option<String> {
        sqlx::query_scalar(
            "SELECT name FROM sgw_organization_ranks WHERE org_id = $1 AND rank = $2",
        )
        .bind(org_id)
        .bind(i16::from(rank.as_u8()))
        .fetch_one(&self.pool)
        .await
        .unwrap()
    }

    pub(super) async fn motd_of(&self, org_id: i32) -> String {
        sqlx::query_scalar("SELECT motd FROM sgw_organizations WHERE org_id = $1")
            .bind(org_id)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }

    /// Character `i`'s `(note, officer_note)` in `org_id`.
    pub(super) async fn notes_of(&self, org_id: i32, i: usize) -> (String, String) {
        sqlx::query_as(
            "SELECT note, officer_note FROM sgw_organization_members \
             WHERE org_id = $1 AND player_id = $2",
        )
        .bind(org_id)
        .bind(self.player_id(i))
        .fetch_one(&self.pool)
        .await
        .unwrap()
    }

    /// The calls to character `i` with method index `method`.
    pub(super) fn calls_of(&self, i: usize, method: u16) -> Vec<Vec<u8>> {
        self.calls_to(i)
            .into_iter()
            .filter(|c| c.0 == method)
            .map(|c| c.1)
            .collect()
    }
}
