---
name: plugin-architecture-compat-boundary
description: Client-compat rules for #962 feature plugins (ADR docs/architecture/plugin-architecture.md §2): single FIFO CellToBaseMsg channel, hook points at the inline call's exact line, cell/base index spaces separate
metadata:
  type: reference
---

Confirmed as advisor for #962 (2026-09-28); canonical text is ADR §2 (C1-C8).

- Moving per-entity state into `CellEntity::extensions` cannot change what the
  client sees: SGW's 436 properties use only CELL_PRIVATE / BASE / CELL_PUBLIC,
  so every client-visible value is an explicit send. Only send ORDER can break.
- Therefore: plugins emit on the one FIFO `CellToBaseMsg` channel (no
  per-feature channels, including for the future `PluginMsg` envelope), and a
  hook point fires at exactly the line the inline call occupied
  (`TickStage` / `EntityHookPoint` variants document their position).
- Cell and base method indices are separate namespaces (0x80 / 0xBD+sub vs
  0xC0); plugins register flattened indices from `cimmeria-wire` constants,
  decode stays in core, GM gate runs before plugin lookup.
- Pilot gotcha: tests in crates below a leaf plugin cannot install it; tests
  that drive a hook point move up or install the plugin via dev-dependency.
- Step 2 (duels, `cimmeria-cell-duel`, 2026-09-28): a lower crate CAN fire a
  hook (`SpaceManager::fire_entity_hook` / `fire_death_hook`), which is how the
  13 travel sites and the combat death resolver stopped naming duels. A lower
  crate can dev-depend on a leaf only if the leaf does not depend on it (cell-duel
  depends on cell-world alone, so combat/console tests install it). Seams the
  model still lacks: value-returning queries (harm gate, damage clamp) and
  base-message handlers (`BaseToCellMsg::Duel`) - those parts stay in cell-world.
  Space-wide feature state goes in `SpaceManager::resources` via an extension
  trait called on the field, so borrows stay field-disjoint.
- Step 3 (org, `cimmeria-cell-org`, 2026-09-28, ADR §4.3): two different
  disconnect points exist - the base's `DisconnectEntity` handler
  (`AfterDisconnectTradeCancel`, squad leave) fires before
  `SpaceManager::disconnect_entity` (`BeforeDisconnectTeardown`, duel end);
  pick the one at the inline call's line, not the nearest name. When a hook
  needs an id the message carries but the entity may have lost (InitPlayerState
  player_id), add a hook kind with that argument (`PlayerHook`) rather than
  reading it off the entity. What the base-message handler and the console
  still call goes to the lowest crate both reach (`cell-interactions`), which
  also removed the console -> cell-methods edge. Windows gotcha while editing:
  Python `open()` without `encoding='utf-8'` silently mis-matches or
  mis-encodes non-ASCII (`–`, `§`) in Rust docs; always pass the encoding and
  assert each replacement landed.
- Step 5 part A (BasePlugin core, `cimmeria-base-session` `base::plugin`,
  2026-09-28, ADR §4.5): crafting's verbs 95-100 are SGWPlayer CELL methods
  (`SGWPlayer.def:916-948` sits in `<CellMethods>` 564-1109), not base
  methods, so the brief's "register 95-100 as base methods" was wrong;
  check the def section before trusting a ticket's index space. SGWPlayer
  has 30 exposed base methods (0xC0-0xDD). The base has no hub like
  `SpaceManager`, so each `ConnectedClientState` carries the registry
  (`plugins`, stamped at login) and hook sites read it via
  `session_plugins(connected, addr)`; the cell-message loop takes it as an
  argument (`route_cell_message`; `handle_cell_message` became a
  test-support wrapper with an empty table so ~50 tests kept their calls).
  Step 5 part B (crafting, `cimmeria-base-crafting`): base half only; a
  combined cell+base crate would sit above `cell-world` (every cell edit
  rebuilds 10k lines of crafting). Base-methods called crafting directly
  (useItem, inventory resync, grant_xp ASP push) - those became hook points,
  item use a value-returning one (first non-NotHandled wins). A missed
  inline site surfaced only at compile: `playCharacter` reset
  `crafting_options` - removing the field is the reliable way to find every
  site. Session-carried registry means a hook site with no session runs
  nothing (a documented edge, never a production path).
  Some `.rs` files have mixed CRLF/LF lines (`server/src/logging/filters.rs`
  filter string): a CRLF-normalizing replace misses them; use Edit there.

Related: [[na38-client-orders-reliable-stream]], [[entity-def-and-pak-ground-truth]].
