# Memory Index

## Exploit references (concrete shapes found in code)

- [Bandolier ammo TOCTOU](exploit_bandolier_ammo_toctou.md) — base UPDATE keys on `type_id` instead of `item_id`; same-type swap dupes ammo across instances.
- [AVATAR_UPDATE_EXPLICIT trust](exploit_avatar_update_explicit.md) — base wire 0x03 accepts client position unconditionally; no anti-teleport, no speed check.
- [Admin API unauthenticated](exploit_admin_api_unauth.md) — `/api/config/stop` and `/api/editor/content` bind on 0.0.0.0 with no auth middleware; trivially DoS-able.
- [useAbility no faction check](exploit_use_ability_no_faction.md) — single-target ability resolves damage on any non-dead target in range; party/vendor friendly fire possible.
- [Lootable no ownership](exploit_loot_no_ownership.md) — lootItem trusts player's own `looting_entity` state; no kill-credit check, no post-interact range re-check.
- [Reference: authoritative state locations](reference_authority_sources.md) — where the server-of-truth state lives for inventory, position, currency, GM flag.

## 2026-05-31 server-authority audit — per-system findings

- [project_mail_handlers_unimplemented.md](project_mail_handlers_unimplemented.md) — Mail send/take/COD/return paths are stubs; future implementers inherit unvalidated wire surface
- [project_trade_handlers_unimplemented.md](project_trade_handlers_unimplemented.md) — All four player-trade RPCs are stubs in social.rs; ordered invariant checklist for the next implementer
- [project_black_market_unimplemented.md](project_black_market_unimplemented.md) — BM/Auction surface stubbed; entire economy vertical lands as exploit chain on implementation
- [project_mission_dialog_audit_2026-05-31.md](project_mission_dialog_audit_2026-05-31.md) — CAT-J audit findings; mission/dialog/interaction trust posture as of 2026-05-31
- [project_gm_commands_audit_2026-05-31.md](project_gm_commands_audit_2026-05-31.md) — CAT-N audit findings; GM/debug/cheat commands trust posture as of 2026-05-31
- [project_org_squad_duel_unimplemented.md](project_org_squad_duel_unimplemented.md) — CAT-M audit findings; all 18 org/squad/duel wire surfaces are stubbed, full checklist for implementer
- [project_crafting_unimplemented.md](project_crafting_unimplemented.md) — CAT-F audit findings; TrainAbility wired (with trainer-NPC gap), all other craft RPCs stubbed; future-implementer must-validate checklist
- [project_chat_contact_audit_2026-05-31.md](project_chat_contact_audit_2026-05-31.md) — CAT-L audit findings; chat/contact/communication trust posture as of 2026-05-31
- [project_world_space_gate_audit_2026-05-31.md](project_world_space_gate_audit_2026-05-31.md) — CAT-O audit findings; gate/ring/region/world-instance trust posture as of 2026-05-31

## Wire-spec references (Ghidra + .def anchors per system)

- [reference_mail_wire_spec.md](reference_mail_wire_spec.md) — SGWMailManager.def authoritative wire shapes for mail messages
- [reference_mail_validation_locations.md](reference_mail_validation_locations.md) — Where mail validation lives (or needs to live) — file/line anchors
- [reference_trade_wire_spec.md](reference_trade_wire_spec.md) — SGWPlayer.def + alias.xml authoritative wire shapes for trade RPCs + Ghidra anchors
- [reference_black_market_wire_spec.md](reference_black_market_wire_spec.md) — Ghidra-extracted wire shapes for BMCreateAuction/BMPlaceBid/BMCancelAuction/BMSearch + inbound reply structs
- [reference_org_squad_duel_wire_spec.md](reference_org_squad_duel_wire_spec.md) — OrganizationMember.def + SGWPlayer.def + SGWDuelMarker.def wire shapes + Ghidra RTTI anchors + permission/rank enums
- [reference_chat_wire_spec.md](reference_chat_wire_spec.md) — Chat/contact/comm wire shapes; SGWPlayer base 0xC0..0xC4 + cell methods 10/55-60/73 + Ghidra anchors for unhandled NetOut events
- [reference_gm_command_wire_spec.md](reference_gm_command_wire_spec.md) — Authoritative wire shapes for ~125 SGW GM commands; SGWGmPlayer.def + Ghidra Event_NetOut_* anchors
- [reference_world_space_gate_wire_spec.md](reference_world_space_gate_wire_spec.md) — CAT-O wire shapes + Ghidra anchors + server-side authority sources for gate/ring/region/movie/system-options
- [reference_cell_method_entity_id_authority.md](reference_cell_method_entity_id_authority.md) — Cell-method entity_id is overwritten with session player_eid in cell_arms.rs

## Recurring exploit classes (rule-of-thumb anchors)

- [reference_auth_handshake_layers.md](reference_auth_handshake_layers.md) — Where Cimmeria's auth/session/character-lifecycle handlers live; which layer is authoritative for what
- [reference_auth_exploit_classes.md](reference_auth_exploit_classes.md) — Recurring SGW auth/handshake exploit classes — TLS, IV reuse, replay, race windows, dev-mode bypass
- [reference_combat_exploit_classes.md](reference_combat_exploit_classes.md) — Recurring combat/abilities exploit classes — caller-state gating, target id existence/AoI, faction/LOS, stub-implementation debt
- [reference_dialog_choice_exploit_shape.md](reference_dialog_choice_exploit_shape.md) — DIALOG_BUTTON_CHOICE has no open-dialog tracking; OnDialogChoice chains are replay-forgeable
- [reference_gm_auth_plumbing_gap.md](reference_gm_auth_plumbing_gap.md) — Systemic gap: cell-method dispatch has no access_level; every future gm* handler is unauthenticated by default

## Per-PR review findings

- [trade-container-whitelist.md](trade-container-whitelist.md) — Trade swap must whitelist source containers (INV_MAIN only) — blacklist-only is a dupe-strip exploit
- [advisory-lock-namespaces.md](advisory-lock-namespaces.md) — `pg_advisory_xact_lock(player_id, ns)` namespace assignments across vendor/trade — divergence is deadlock surface, not correctness
- [pattern-checked-alloc-size.md](pattern-checked-alloc-size.md) — Canonical helper for count*stride bounds + overflow checks on attacker-influenced binary input
- [pr-426-navmesh-extractor.md](pr-426-navmesh-extractor.md) — Build-time navmesh parser hardened with checked_alloc_size; pattern worth reusing for header-driven Vec allocation
- [pr-427-crafting-phase1.md](pr-427-crafting-phase1.md) — Phase 1 dispatch+persist; no mutation surface yet; Phase 2 is where the real adversarial review lands
- [open-followup-runtime-navmesh-load.md](open-followup-runtime-navmesh-load.md) — NavMesh::load in cimmeria-entity has the same unguarded count*stride pattern; worth a follow-up issue
