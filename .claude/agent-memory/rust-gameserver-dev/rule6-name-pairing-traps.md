---
name: rule6-name-pairing-traps
description: Pairing log IDs with names (Rule 6, NT-22): where base-side names come from, the NameBook borrow trap, and rustfmt bailing on long macro lines
metadata:
  type: project
---

Facts learned sweeping base-methods inventory for Rule 6 (NT-22a, 2026-10-04):

- **Base helpers with only an ID + pool** name players, accounts and orgs through
  `cimmeria_entity::known_names::{player_name, account_name, org_name}` (filled at login,
  `playCharacter`, and every org row read in base-session). They take `i32`, `u32`, `i64` or an
  `Option` of those. A player's own `entity_id` pairs with `entity_name = known_names::player_name(player_id)`.
- **Never take an extra base lock to name a line** (`connected` / `entity_to_addr`, e.g. via
  `identity_for_entity`): OTEL exports every cimmeria crate at DEBUG and file layers write TRACE, so
  a debug-level field is not free. Use `known_names`, `space_registry::world_of_space`, or a lock you
  already hold; otherwise mark the field id-only.
- **`cimmeria_names::book().item(id)` borrows the guard**: inline in a tracing field it works, but
  `opt.and_then(|t| book().item(t))` does not compile. Use `cimmeria_names::owned::{item, container}`
  (copies; the field expression only runs when the event is enabled).
- **Container IDs have names**: `book().container(id)` = `resources.containers.name` (`MAIN`, `BANK`).
  Slot indexes, instance IDs whose row isn't read yet, banker/corpse NPC IDs on the base: `// nt:id-only`.
- **rustfmt silently gives up on a macro call that has any line over 100 chars**, so a long
  `// nt:id-only` reason leaves the whole call unformatted (and `fmt --check` still passes). Keep
  reasons short (`slot index, unnamed`).
- Renaming a log key (`type_id` → `item_type_id`) breaks LogCapture tests that assert the old
  key: grep `"type_id"` in the `*_tests.rs` files, including other modules' tests that capture your
  event (vendor purchase asserts `grant_container_chosen`).
- `grant_container_chosen`'s `item_name` comes from the DB seed (`Health Slappack TC1` for 2893),
  not the NameBook, so a test book must use the seed's spelling.

Related: [[observability-test-and-throttle-traps]], [[testing-patterns-index]].
