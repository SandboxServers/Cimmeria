---
name: reference-ability-grant-provenance
description: CS-01a (PR #1264) grant provenance authority map; cleared shape, open gaps (trained-then-granted loss, no archetype self-gate, unguarded init-row gm filter)
metadata:
  type: reference
---

# Ability grant provenance (CS-01a, reviewed 2026-10-05, PR #1264)

Authority: `sgw_player_ability_grants` (PK player_id+ability_id, kind CHECK, cascade FK).
Writers all lock `sgw_player` first: `progression/grant_provenance.rs` (content,
`FOR UPDATE`), `gm_ability_bulk.rs` (FOR UPDATE), `grant_ability.rs` (UPDATE then
gm row). Trainer `persist_purchase` uses `NOT (abilities @> ...)`, so it serializes
with grants via the row-lock re-check.

Credit = `tree_points_spent + grant_credit()` (`cell-catalog/.../gates/spend.rs`),
cell-side only. Hydrated from two separate SQL filters `source_kind <> 'gm'`:
`player_init_row.rs` (world entry) and `grant_provenance::credited_grants` (reset).
Only the reset one has a live-DB guard; the spend_gates unit "gm gives no credit"
test feeds the slice directly (theatre).

Cleared: gm->content promotion (only fires when the chain grants that exact id;
final state equals an un-GM'd character). Replay is a no-op.

Open at review time:
- A trained id that is later content-granted gets no row, so a respec deletes the free signature.
- `grant_ability` has no archetype/race self-gate. Dialog/region/stargate dispatch
  never sets `archetype`, so `Condition::Archetype neq` fails open
  ([[reference-dialog-choice-exploit-shape]]).
- A content grant of a starter writes a row, which makes it credit-bearing.

Re-check these when CS seeds signature/racial_core rows.
