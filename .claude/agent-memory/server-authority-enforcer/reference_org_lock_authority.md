---
name: reference-org-lock-authority
description: Where Team/Command authority lives (ORG-02, PR #881) and the gaps found in its first review — authz is by convention, audit export is at-most-once
metadata:
  type: reference
---

ORG-02 (PR #881, reviewed 2026-09-27) put Team/Command state in `sgw_organizations` / `_ranks` / `_members`, API in `crates/base-session/src/base/organization/`.

- Authority read: `api::member_access_locked(tx, org_id, player_id)` after `lock_org` (ORG-LOCK, D-ORG04; order org row -> sgw_player -> items).
- Persistence mutations (`add_member`, `remove_member`, `set_rank`, `set_text`, `set_rank_permissions`, `disband`) take NO actor/witness: "authorize inside the lock" is convention only. `loads::load_memberships` returns rank+permissions from a pool read, the natural stale-authz trap for handlers (ORG-05..08) and the Bank campaign. Recommended fix: mutations take an `&OrgAccess` witness only `member_access_locked` can build.
- Leader invariant: AFTER DELETE trigger `org_member_after_delete` promotes/disbands; there is no UPDATE guard, so raw SQL can demote the leader or move a member's org_id and leave an org leaderless.
- Audit (`sgw_organization_events` + `organization::audit`): stamps `exported_at` before logging, so it is at-most-once, not exactly-once; the in-transaction path logs at DEBUG and stamps, so kick/leave promotions never reach INFO.

Re-review incrementally from these points when ORG-05..08 or the Bank vault land. Related: [[project-org-squad-duel-unimplemented]], [[advisory-lock-namespaces]].
