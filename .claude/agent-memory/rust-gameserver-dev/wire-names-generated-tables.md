---
name: wire-names-generated-tables
description: cimmeria_wire::names (NT-30, 2026-10-04) is the one id->name source for message ids and method indices; tables are generated, regen command, naming rules for unknown entity types, and the traps hit building it
metadata:
  type: project
---

`cimmeria_wire::names` names Mercury message ids and per-entity-type method
indices for logs. Every lookup returns `Option<&'static str>`; resolve it
inside the macro, never `"unknown"`. Rule 6 keys: `msg_id`/`opcode` ->
`msg_name` (`client_msg_name`/`server_msg_name`: interface entry, or the range
name `cellMethod`/`baseMethod`/`entityMethod`); `method_index`/`method_id` ->
`method_name`. A `msg_id` in an entity range logs both (review of #1191).

- `names/defs/*.rs` are **generated** by the test
  `mercury::def_conformance::names_codegen` from `entities/entities.xml` +
  `entities/defs/`. After a `.def` change:
  `CIMMERIA_REGEN_NAMES=1 bash tools/build-lane/lane.sh cargo test -p cimmeria-wire names_codegen`
  (the env var passes through the lane). The files carry `#[rustfmt::skip]` on
  their `mod` lines: rustfmt re-aligns `"name", // 12` trailing comments and
  would break the byte-for-byte check.
- `names::doc_conformance` parses all four `docs/protocol/*-dispatch-table.md`
  and fails with doc path:line on a disagreeing row. All rows agreed at
  creation; the losing side then was code: wire-log's old `inbound_msg_name`
  had 0x06 `loggedOff`, 0x0A `createCellPlayer`, a fake 0x0D `channelSetup`.
- Old `&str` APIs (`cell_method_name`, `base_method_name`,
  `outbound_method_name`, `OrgBaseCall/OrgCellCall/CraftVerb::method_name`,
  wire_ledger `method_name`) now delegate; keep their `"unknown"`/`"other"`
  fallbacks, they feed registry checks and closed label sets.
- Naming rules: a player's own or any player entity's client method ->
  `player_client_method` (GM table, superset: the cell keeps GMs at class
  0x02). Unknown entity: `entity_client_method(is_player, idx)`; for a
  non-player, `any_entity_client_method` (None only for 27-31: player
  Communicator vs mob/pet own methods). Keep `entity_is_player` where the
  caller has it (witness methods carry it). Cell: `SpaceManager::client_method_name`.
  Inbound at the base: `Account` (0x07) at char select, player class in world
  (0xC0 is `versionInfoRequest` vs `chatJoin`); `inbound_method` reads the GM
  superset for SGWPlayer so a non-GM probing a GM index is named.
- `mercury.tx_hole` cannot call wire (dep cycle) and holds no session key.
  The send path records nothing except a later bundle fragment's head, taken
  from `plan_fragments`' existing framing walk (`FragmentPlan::heads`; blobs
  hold several messages). On a warning tick base-session's namer
  (`check_tx_hole_named`) decrypts the retained `raw_bytes` and reads the head
  at body offset 0 (`first_message_head`), skipping non-first fragments.
- A new top-level `cimmeria_wire` module needs its own `cimmeria_wire::<mod>=debug`
  row in `crates/server/src/logging/filters.rs` `OTEL_FILTER`, or
  `parity_tests::crate_rows::every_in_process_crate_has_its_own_otel_row` fails.

**Why:** named-telemetry campaign (docs/analysis/named-telemetry/); NT-40 reuses
the API for client replay rows. **How to apply:** never hand-type a method-name
table again; add lookups to `names` and regenerate. Related:
[[method-idx-duplicate-table-drift]], [[gm-tail-dispatch-doc-filename-trap]].
