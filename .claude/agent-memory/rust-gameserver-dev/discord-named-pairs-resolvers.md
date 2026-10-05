---
name: discord-named-pairs-resolvers
description: Since NT-10 every Discord Event object is a Named {id, name} pair; where each seam resolves names, and which objects the server cannot name (dialog button text, minigame id)
metadata:
  type: project
---

NT-10 (2026-10-04) turned every object field on `cimmeria_discord::Event` into `Named { id: Option<i64>, name: Option<String> }`, rendered by `embed::format::named` (`Name (#id)` / `#id` / name / `?`).

Resolvers to reach for at an emit site:
- Base: `ConnectedClientState::discord_account()` / `discord_character()` (player_id = `active_player_id`), `cimmeria_base_session::base::discord_world(name)` (NameBook reverse `world_id`).
- Cell: `SpaceManager::discord_character / discord_entity / discord_world / discord_world_of` (`space_manager/discord_labels.rs`; meant to fold into NT-02's `entity_label`).
- Content: `cimmeria_names::book().mission/item/dialog/template(id)`, `archetype_name`.

Cannot be named server-side: dialog button text (only in the client's `CookedDataDialogs.pak`; `-1` = closed is the one known choice), minigame numeric id (catalogue is keyed by name), content chains (no name column; `scope_type='mission'` + `scope_id` would give the mission if a chain index ever lands).

**Why:** Rule 6 "Discord"; the table test `every_typed_event_renders_each_object_as_name_and_id` fails a variant that renders a bare name.
**How to apply:** a new emit site passes `Named`s, never a formatted `entity:<id>` string; use `Named::or_entity(eid)` for the uncached-character fallback. See [[gm-feedback-cell-base]].
