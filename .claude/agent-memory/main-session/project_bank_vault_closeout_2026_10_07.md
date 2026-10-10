---
name: project-bank-vault-closeout-2026-10-07
description: "BV-10 release status and Banker entity-id reuse guard, verified 2026-10-07."
metadata:
  type: project
---

BV-10 PR #968 merged on 2026-09-27 (`9e9e91f0d`), and its `/release` comment dispatched the release workflow. The ledger's Review status was stale. Owner UAT remains pending; use `docs/analysis/bank-vault/handoffs/session-resume.md` steps 1-25.

The vault verdict in `crates/cell-world/src/cell/space_manager/vault_access.rs` previously checked the pinned Banker id only for existence, space and range. Entity ids can be reused; a nearby non-Banker or Banker of another scope could inherit a session. The verdict now checks `NpcInteractionType::Banker` and the session scope after the range check, returning the existing `banker_gone` reason on mismatch. The regression case is `vault_move_allowed_enforces_session_space_and_proximity` in `crates/cell-interactions/src/cell/interactions/bank/tests.rs`. A replacement Banker of the same scope can still inherit the id because `VaultSession` has no spawn identity; the remaining gap is in the session-resume doc.
