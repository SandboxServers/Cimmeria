---
name: Slash palette typed forms, entries 28–54
description: Patch-012 forms preserve source types while unsupported handlers and native binding remain disabled
type: reference
---

# Slash metadata 28–54 (2026-10-16)

- DW-25B maps the 27 exact patch entries into typed disabled forms. `/gmsetmobabilityset` is optional in patch TOML with no default, while the candidate server handler requires INT32 bytes; a current lab entity snapshot cannot read back known abilities or set ID. Source: `docs/analysis/dev-workbench/evidence/DW-25B-E.md`, `crates/lab/src/workbench/slash_descriptors_1b.rs`.
- `/gmreloadorganizations` re-pushes caller Team/Command state and can partly fail; it is transient, not read-only or a DB definition reload. `/gmdhd` and `/gmgotolocation` may durably learn gates after arrival; same-space travel can conditionally persist player position. Source: the same evidence and descriptor files.
- 2026-10-16 review: the second descriptor slice now rejects event and parameter drift before generating metadata, uses explicit argument target resolution for stat and same-space travel commands, reads grant provenance from `sgw_player_ability_grants`, and leaves organization client-state verification pending because no `client_org_view` exists. Source: `crates/lab/src/workbench/{commands,slash_descriptors_1b}.rs`, `docs/analysis/dev-workbench/slash-palette-reference.md`.
- 2026-10-16 follow-up: source-backed signed INT32 arguments are range-bound; `/gmsettarget` accepts only nonnegative numeric text through INT32 max and `/gmgotolocation` requires a nonblank world name. DB recipes include player and row keys, including nullable ability `source_id`. Source: `crates/lab/src/workbench/{commands,slash_descriptors_1b}.rs`, `db/sgw/Players/Tables/{sgw_player_discipline_expertise,sgw_player_ability_grants}.sql`.
