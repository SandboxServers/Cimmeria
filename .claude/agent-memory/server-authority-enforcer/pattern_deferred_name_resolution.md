---
name: pattern-deferred-name-resolution
description: Named-telemetry review shape — an entity_id captured at queue time and named via entity_label at a later confirm/flush names a freed or recycled slot
metadata:
  type: reference
---

When reviewing Rule 6 name-pairing sweeps (NT campaign), check every row that logs an entity id **captured earlier** (a queued change, a batch, a pending record). `SpaceManager::entity_label` / `entity_names` resolve the live occupant *now*; if the entity was despawned or respawned since capture, the name is `None` or names a different occupant. Found 2026-10-04 in PR #1211: `.seedconfirm` row (`cell-console/.../seed.rs`) named `s.entity_id` at confirm while `.delspawn` had already despawned it at queue time.

**How to apply:** fix is capture the name at record time (static `&'static str` from `entity_names`), or `entity_label_at(space, id, captured_at)`, or `// nt:id-only`. Also check Rule 5 role keys: rows whose pre-existing `player_id`/`entity_id` was the *subject* get the subject's name under actor keys (`.givecash`, crafting `send_grant`, `.missionfail` in #1211). Related: [[exploit-entity-id-recycling]].

**Base-side lock re-entry (nt-26 audit, 2026-10-04).** Base name helpers (`session_identity::identity_for_entity`, `player_name_for_entity`, org `identity_of_player`, mail `online_identity`) take `entity_to_addr` then `connected` (std, non-reentrant). Edition 2021: in `match connected.lock() { Ok(..) => .., Err(_) => { <helper> } }` the scrutinee temporary keeps the *poisoned guard* alive through the Err arm, so a name lookup there self-deadlocks and then hangs every other `connected` locker (found in contact_list `notify_online_contacts`). Grep every added lookup for an enclosing `match/if let ... .lock()`; use `Err(p) => { drop(p); .. }` or log ids only. Async fns are safe from caller-held std guards (a guard across `.await` makes the future !Send), so only sync call chains need tracing. Hot-path rule: reuse the identity already read under the dispatcher's lock (`chat.rs` does) rather than re-lock per message.
