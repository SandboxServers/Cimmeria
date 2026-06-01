---
name: pr-427-crafting-phase1
description: Adversarial review verdict on PR #427 — crafting Phase 1 lands persistence + dispatch only; no exploitable surface yet, but Phase 2 will need a deeper pass
metadata:
  type: project
---

PR #427 (crafting Phase 1) verdict: SHIP (clean adversarial review).

**Why the gate is clean:** Phase 1 is intentionally scoped to (a) DB schema + persistence helpers, (b) wire-format serializer for `onUpdateDiscipline` (method 136), and (c) the dispatch range widening that adds `SPEND_APPLIED_SCIENCE_POINTS` (95). The cell handlers for indices 95/96/97/98/99/100 are all UNIMPLEMENTED stubs that `tracing::info!` and return true. No state mutation, no item/currency mutation, no GM-bit dispatch, no item_id-vs-type_id surface. A malicious client gains nothing by calling these handlers except a log line.

**How to apply:** When Phase 2 lands (the ASP-spend validation, paradigm gate, prerequisite expertise floor, DB UPDATE), re-run the adversarial review with the full handler ↔ DB mutation path in scope. Specific things to check at Phase 2:

- `applied_science_points` decrement must be server-authoritative (`UPDATE sgw_player SET applied_science_points = applied_science_points - 1 WHERE player_id = $1 AND applied_science_points >= 1`) with `rows_affected() == 1` check. Naive `state.applied_science_points -= 1; save_crafting_state(...)` is racy with two concurrent ASP-spend packets.
- `discipline_id` validation against the static discipline table — reject indices outside the canonical set, not just "negative".
- Prerequisite checks (`expertise[prereq_id] >= required_expertise`) must be evaluated server-side using the freshly-loaded state, not the cached client-derived snapshot.
- `racial_paradigm_levels[discipline.racial_paradigm_id] >= discipline.racial_paradigm_level` gate must also be evaluated against the just-loaded state.

**Existing defensive primitives ready to use at Phase 2:**

- `save_crafting_state` already returns `sqlx::Error::RowNotFound` for `rows_affected() == 0` on the player UPDATE — covers the missing-player case but NOT the "ASP balance was 0 so the conditional WHERE didn't match" case. Phase 2 needs a distinct `InsufficientFunds`-shaped error.
- `set_expertise` clamps to `[0, 100]` per the Python `gainExpertise` hard cap.
- `load_crafting_state` defensively clamps `i32 → i8` for paradigm levels via `i8::try_from(...).unwrap_or_else(... clamp ...)` — useful pattern.

**No double-consume risk in Phase 1** — no `remove_item` or `UseInventoryItem` adjacency. Phase 4/5 (craft/research/alloy actually consuming items) is where the double-consume trap can re-emerge.
