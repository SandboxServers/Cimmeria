---
name: project-character-editor-handoff
description: "External character-create/select handoff (2026-09-20) reviewed, not imported: wants CharDefs 9/10/19 blocked client- and server-side; no server allowlist exists; the live-DB fixture uses CharDef 9; the WQHD layout it assumes is not available"
metadata:
  type: project
---

On 2026-09-20 an external audit of CharacterCreate/CharacterSelect arrived (`SGW_CHARACTER_EDITOR_CLAUDE_HANDOFF_PRE_INGAME_TEST_2026-09-20.zip`, same lineage as the v1.2 handoff pack under `docs/analysis/sgw-handoff-pack-v1.2/`). It was reviewed only. Nothing was imported, and no patch was written.

**What it asks for:** allow Praxis/SGU Human Soldier, Commando, Scientist and Archeologist, plus Praxis Jaffa and SGU Shol'va, male and female. Block CharDefIds 9 (SGU Asgard), 10 (Praxis Goa'uld M) and 19 (Praxis Goa'uld F), both client-side and server-side.

**Verified against main at b93ae222 (2026-09-20):**

- `chardef_lookup` accepts all ids 1-23. Its only caller is `handle_create_character`. No allowlist exists.
- Names are 3-20 ASCII characters. Skin tint is 0..=15. `requestCharacterVisuals` sends primary/secondary tint as `0xFF`/`0xFF`.
- `db/resources/Archetypes/Seed/char_creation_choices.sql` confirms the glasses duplication: 16 rows of Glasses00, 144 of Glasses01, and no other glasses mesh.

**Traps:**

- `character_create_live_db_tests.rs` uses CharDefId 9 (Asgard) as its fixture, because it has the fewest optional visual groups. A server allowlist breaks that test unless the fixture moves to an allowed CharDef, which has more optional groups and a longer payload.
- The pack says "preserve the current WQHD layout", but a stock client still has the untouched 2009 CharacterCreate/CharacterSelect files (hash-identical to the pack's `original_reference`). The WQHD layout and `SGW_CHARACTERCREATE_FILTER_v27.zip` are not in the pack or the repo. Get them from the owner before any client-side work.
- Client UI Lua and layout files have no home in this repo, so client-side changes are client patches (see `docs/agents/rules-and-gotchas.md`).
- The pack leaves these unresolved, so don't invent them: zoom (native camera), Play Demo, Jaffa/Shol'va suffix combo population, primary/secondary tint semantics.
