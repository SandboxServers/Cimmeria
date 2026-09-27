# CR-E2 Worknotes

> Type: reference. Audience: crafting-campaign coordinator.
> Companions: `docs/analysis/crafting/README.md`, `work-packets.md` (not present in this
> worktree's base — see "Contract" below), `docs/reverse-engineering/findings/crafting-items.md`.

## Contract

- **Packet:** CR-E2 — Blueprint items, Racial Paradigm Guide items, Field Crafting Tools in
  the client's cooked data. Documentation + a proposed mapping file; no Rust.
- **Decisions in force:** D-CR03 (racial paradigm progression via Guide items), D-CR04
  (blueprint acquisition via Blueprint items + research), D-CR05 (stations + tools), D-CR21
  (Field Crafting Tool science/tech_comp rule — proposed, this packet's Q3 answer feeds it),
  D-CR22 (Blueprint/Guide item-use mapping — proposed, this packet's Q1/Q2/Q4 answers feed it).
- **Depends on:** none (Wave 0, dispatched in parallel with CR-01/CR-E1/CR-02).
- **Base:** `95366c59c71fc46126b2bbe43eee2ca7d79d8c40` (origin/main) on branch
  `craft/cre2-cooked-items`, worktree `.claude/worktrees/cre2`. **The crafting campaign
  ledger (`docs/analysis/crafting/{README,work-packets,audit}.md`) does not exist in this
  base** — plan PR #851 had not merged to `main` as of this base commit. The packet, decision
  text (D-CR03/04/05/21/22), and audit rows (C-20 to C-28) were read from the coordinator
  worktree at `C:/Users/Steve/source/projects/Cimmeria/.claude/worktrees/craft-plan/docs/analysis/crafting/`
  per the standing worker-rules instruction for this case. This worktree's
  `docs/analysis/crafting/{source,worknotes}/` directories did not exist either and were
  created fresh by this packet.
- **Owned paths (this packet):**
  - `docs/reverse-engineering/findings/crafting-items.md` (new)
  - `docs/reverse-engineering/findings/README.md` (added one row, bumped the doc count 73→79
    to match the actual row count after the rebase onto CR-E1 (#858), which had already drifted before this packet touched it)
  - `docs/reverse-engineering/README.md` (bumped "72 docs" → "79 docs", date 2026-09-25 →
    2026-09-26)
  - `docs/analysis/crafting/source/blueprint-items.csv` (new)
  - `docs/analysis/crafting/worknotes/cr-e2.md` (this file)
- **Read set:** `db/resources/Items/Seed/items.sql`, `db/resources/Entities/Seed/blueprints.sql`,
  `db/resources/Texts/Seed/texts.sql`, `db/resources/Archetypes/Seed/disciplines.sql`,
  `crates/wire/src/containers.rs`, `crates/resources/src/base/resources/mod.rs` (category→PAK
  table, confirms `data/cache/*.pak` as the loader's data dir),
  `crates/base-methods/src/base/world_entry/methods/inventory/core/use_instance.rs`,
  `docs/gameplay/inventory-system.md`, `docs/engine/cooked-data-pak-format.md`,
  `docs/engine/cooked-data-pipeline.md`, `docs/reverse-engineering/findings/crafting-restoration.md`.
  Ghidra MCP was **not used** this session — the cooked-data PAKs plus the existing
  `crafting-restoration.md`/`cooked-data-pak-format.md` findings gave enough evidence for Q1–Q3
  and the server half of Q4; the client-side "does the Use menu option appear" half of Q4 is
  left as an open question needing a Ghidra/live-client session (see findings doc §Q4/OQ1).

## Data source

`data/cache/CookedDataItems.pak` (category 4 — `CookedDataItems.pak`, confirmed by
`crates/resources/src/base/resources/mod.rs`'s `CATEGORY_PAKS` table) and
`data/cache/CookedBlueprints.pak` (category 15). Both are ZIP archives with one entry per
element, named `_<id>`, holding raw `COOKED_ITEM`/`COOKED_BLUPRINT` XML (QA-build format —
SOAP-namespaced, per `docs/engine/cooked-data-pipeline.md`'s format-generations table). Parsed
with a throwaway Python script (not committed — one-shot analysis, not a reproducible
generator the coordinator asked for; the CSV it produced is the deliverable). Counts: 6,059
items (matches audit C-24's "6,059 items"), 498 blueprints (matches audit C-22's "498
blueprints").

## Findings summary (full detail + evidence in the findings doc)

1. **No cooked-data field links a Blueprint item to a blueprint.** Every one of the 289
   `"Blueprint: …"` items is `IsElementaryComponent="true"`, `AppliedScienceID="0"`, and
   `ItemEventSet AbilityID="0" EventID="5"` — a generic, ability-less "use" item. Zero of the
   289 are referenced as a component inside any of the 498 blueprints. The only usable signal
   is the item's own display name, matched against blueprint product names.
2. **Mapping resolved 192/289 rows with HIGH/MEDIUM-HIGH confidence** (36 exact name match +
   154 TC-suffix family match + 1 disambiguated + 1 resolved via a documented client naming
   bug), **1 row is a genuine unresolved 2-way tie**, **45 rows** narrow only to a group of
   5–15 same-suffix candidate products with no way to pick one, **48 rows** name a product
   that was never shipped (the "Slappack Consumable" reward family — a real 2009 content gap,
   not a matching failure), and **3 rows** have no candidate product under any method tried.
   Full row-by-row detail: `docs/analysis/crafting/source/blueprint-items.csv`.
3. **Racial Paradigm Guide items already exist and are fully cooked** — 5 items (`7805`–
   `7809`), one per paradigm, matching D-CR03's cited text verbatim, already present in the
   seed. **D-CR22's "what does a seed-only item need" fallback question is moot** — nothing
   further is needed for the client to render them.
4. **Field Crafting Tools carry no applied-science field** — `AppliedScienceID="0"` on every
   sampled tool; the science is name-prefix/description-text only, confirming D-CR21's premise
   that the science has to come from the item's name. The `{17,15}` container pair the packet
   asked me to confirm **is confirmed**, but it's not tool-specific (every item family sampled
   shares it); container 17 isn't a real player bag (`bag_max_slots` has no entry for it), so
   bag 15 remains the only bag that actually matters.
5. **`useItem` is already fully generic server-side.** `handle_use_inventory_item` fires
   `ItemUsed` for any owned item regardless of `AbilityID`/`EventID`, with no "is this item
   usable" gate at all. CR-15 needs only a `type_id`-keyed handler on the `ItemUsed` event —
   no change to the dispatch path itself.

## What this means for D-CR21 / D-CR22

- **D-CR21** (Field Crafting Tool rule) is unaffected by any new cooked-data evidence — the
  proposed rule (science from name prefix, since no cooked field exists) is the *only* option;
  this packet confirms there is no better field to use instead. The `{17,15}` container
  observation is corroborating evidence for D-CR21's bag-15-only framing, not a contradiction.
- **D-CR22** (item-use mapping) can now be written concretely for CR-15:
  - Seed a mapping table from **192 rows with HIGH confidence** directly off the CSV's
    `name-exact`, `tc-suffix-match`, `name-exact-disambiguated`, and
    `structural-tier-progression` rows.
  - The **5 Racial Paradigm Guide items** map to `racial_paradigm.sql` by name-suffix
    (`Human`, `Common`, `Asgard`, `Goa'uld`, `Ancient`) — trivial, no CSV row needed since
    these aren't in the Blueprint-item set; CR-15 should build this small table directly.
  - The **1 ambiguous row** (`8882`, two candidate blueprints `367`/`369`) needs an explicit
    owner or CR-15-author call — the CSV documents both candidates rather than guessing.
  - The **45 group-unresolved rows** and **48 no-product rows** should almost certainly be
    **excluded** from CR-15's seed table rather than guessed at — for the no-product rows, the
    reward literally does not exist in cooked data; for the group-unresolved rows, wiring any
    single guess in would silently teach the wrong recipe. The findings doc's open questions
    list what evidence (a Ghidra client-UI trace, or an owner decision) would unblock them.
  - The **3 fully unmatched rows** (`5395`, `5439`, `8885`) should be excluded — no cooked
    product exists under any name variant.

## Design decisions

- **CSV column set** followed the team lead's explicit spec (`item_id, item_name, blueprint_id,
  product_id, method`) with two extra columns (`confidence`, `note`) appended rather than
  substituted, so a positional reader of the first 5 columns still works, and the evidence
  trail for every row (including the unresolved ones) travels with the data instead of living
  only in prose.
- **No fabricated ordinal picks.** For the 45 Subcombine-group rows, an earlier draft of this
  analysis picked "the Nth candidate by item id" to fill in a specific blueprint id per the
  `Aa`/`Ab`/`Ac` variant letter. That pick had zero supporting evidence (no field orders the
  candidates, and the variant letter's meaning is unconfirmed) — dropped in favor of reporting
  the full candidate group and marking the row unresolved, per this agent's "resist the
  temptation to invent" operating principle. The CSV's `note` column carries the full
  candidate list so a future session doesn't have to re-derive it.
- **Cooked pak as ground truth over the seed for item names.** Item `6483`'s seed `name`
  column holds its *description* text ("Crafting Blueprint: Right Click to Use...") instead of
  its real name; the cooked pak's `Name` attribute ("Blueprint: Steel Plating (Materials
  Subcombine A)") is correct and resolves cleanly via plain name-exact match. Rebuilt the
  blueprint-item list from the cooked pak's own `Name="Blueprint: …"` entries (289, exactly
  matching the seed's id set) rather than from the seed's `name` column, so this one bad row
  didn't fall out of the analysis as a false unmatched.

## Known gaps / not done

- No Ghidra session — the client-side "does a Field Crafting Tool even show a Use option"
  question (Q4) is flagged MEDIUM/inferred, not decompiled. See findings doc OQ1.
- Did not attempt to resolve whether the 45 Subcombine-group candidates really are
  quality-tier duplicates (strong circumstantial evidence: block-spaced id ranges per science)
  — flagged as OQ2 for whoever picks this back up.
- Did not touch `db/resources/` — this packet is documentation + a proposed mapping only, per
  scope. CR-15 owns turning the CSV into a seed table.
- The doc-count fix in both README files corrects pre-existing drift (73 claimed vs 78 actual
  rows before this packet's addition) as a byproduct of adding one row — flagged here in case
  the coordinator wants to attribute that fix to a different packet's history.

## Log

- **2026-09-26** — Read CRAFT-WORKER-RULES.md, the CR-E2/CR-15 packet text, D-CR03/04/05/21/22,
  and audit C-20/21/22/24/25/26 from the coordinator worktree (this worktree's base predates the
  plan PR). Located `data/cache/CookedDataItems.pak`/`CookedBlueprints.pak`, parsed both with a
  throwaway Python script, cross-checked against `items.sql`/`blueprints.sql`/`disciplines.sql`.
  Built and iterated the item→blueprint mapping through several passes (plain name match → TC-
  suffix family match → Subcombine structural analysis → the Mark VII/X naming-bug discovery),
  discovering along the way that item `6483`'s seed name is corrupted and that the true
  blueprint-item source of truth is the cooked pak's own `Name` attribute. Confirmed Racial
  Paradigm Guide items already ship complete. Confirmed Field Crafting Tools carry no
  applied-science field. Read `use_instance.rs` to confirm the server's `useItem` path is
  already generic. Wrote the findings doc, the CSV, this worknote, and updated both RE README
  indexes (converted to CRLF after each edit — `docs/**/*.md` convention). No Rust changes; no
  build/test commands run (packet scope is docs + CSV only). Pushed the branch.
