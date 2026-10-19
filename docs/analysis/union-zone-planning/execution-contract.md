# Union campaign execution contract

> **Type:** How-to
> **Audience:** A future main session taking one Union zone campaign
> **Last updated:** 2026-10-14

This contract imports the [first four-zone execution contract](../four-zone-planning/execution-contract.md) for worker size, source classification, test policy, telemetry, wave integration, fresh adversarial review, worktree retirement, CI and the merge queue. It adds the Union-specific start gate and handoff ownership below. If the inherited contract changes before implementation, the future coordinator reconciles the current version rather than copying stale commands. The repo's `CLAUDE.md`, `TESTING.md`, `seed-change`, `lane-build`, `lab-uat` and `ship-pr` instructions still apply.

## Start a separate main session

1. Verify the Cellblock, Castle, Harset and Tollana continuation ledgers and FZ-99 are complete and merged. Check `git worktree list`, recent branch commits and the board before creating a campaign worktree. A planning document alone is not a dispatch signal.
2. Read this [index](README.md), [evidence](evidence.md), the chosen [SGC](sgc.md), [Dakara](dakara.md), [Omega](omega.md) or [Beta](beta.md) zone file and its [SGC](sgc-work-packets.md), [Dakara](dakara-work-packets.md), [Omega](omega-work-packets.md) or [Beta](beta-work-packets.md) packet contract, the applicable [Free Jaffa](free-jaffa-route.md) and [Asgard](asgard-route.md) route briefs, the current source workbook, and the inbound campaign ledger. Create `docs/analysis/union-<zone>-continuation/` with a Decisions table, packet status table, `worknotes/` and `RESUME.md`. Reconcile every claimed baseline against the current seed and client build. Dakara first inherits the existing `dakara-e1-rebuild/` ledger and claims only unfinished work.
3. Record a source manifest per mission: source file/build, exact mission/step/objective/task/dialog IDs, world, faction, grant and reward confidence, actor placement confidence, provenance per beat, and an `activate / reconstruct / hold` disposition. A title or moniker alone never supplies a grant, dialog binding or position. Seal source and placement packets with domain advisors before any implementation coder receives them.
4. Claim chain, mission, dialog, spawn, template and item IDs against current `main`, all active branches and the first four-zone [ID allocation](../four-zone-planning/id-allocation.md). Record claims in the new ledger. A new mission ID also must be absent from the stock client catalog. The [authored mission catalog path](../four-zone-planning/cb-n05a-seal.md) is in `main` as PR #1734 as a mechanism only: `MISSION_ADDITIONS` in `crates/resources/src/base/mission_additions.rs` is empty apart from a test fixture (id 61000), and the Cellblock dialog-override precedent is not on `main`. Every authored mission in a Union plan is therefore HELD until a bounded live-client proof shows a real authored mission in the journal; no packet may assume it works. `id-allocation.md` is coordinator-owned: a packet files an amendment request in its own ledger and never edits that file (see Shared definitions).
5. Run one small evidence packet and one small implementation packet as the measured pilot. Seal every implementation packet to about 40k working tokens under the 90k Haiku context cap. A packet that needs design, a new primitive, a map rebuild or an uncertain client presentation goes to the matching specialist or splits; a small-model coder receives only frozen IDs and file ownership.

The campaign branch names are `content/union-sgc-campaign`, `content/union-dakara-e1-campaign`, `content/union-omega-campaign`, and `content/union-beta-e1-campaign`. They are proposed names for later sessions, not branches created by this PR. One coordinator owns each feature branch and one worktree/test database per implementing packet. A Free Jaffa or Asgard route integration session records cross-zone evidence and sends defects to the owning zone coordinator; it does not edit competing zone branches. The zone files list packet **families**; the first evidence packet converts each into dispatch briefs with exact functions, IDs, allowlists, test names, checks and commit subjects. It may split a family by mission or race without changing the first-Beta convergence boundary.

## Common acceptance for runtime packets

| Concern | Required guard |
|---|---|
| Mission behavior | A chain-replay regression reproduces the grant, step/objective order, reward once, abandon/retry and relog boundary, and fails if the new chain is removed. |
| Seed relationships | A live-DB test validates changed mission/dialog/actor/template joins and persistence using the worktree database. A seed file alone is not a test. |
| Travel and position | A movement/transport test validates world and coordinates; client UAT checks arrival orientation and reachable geometry after owner approval. |
| Faction and phasing | Negative tests use at least one wrong-faction character and two players in different mission phases; no client-supplied mission ID or choice may authorize state change by itself. |
| Presentation | The first press gives visible feedback; the client shows the intended dialog, journal and objective. A newly authored quest goes through the existing ResourceCache mission catalog path. |
| Observability | Record chain fired, step/objective credit and refusal reason in the existing content telemetry. The worknote names events and IDs so a SigNoz trace can explain a failed playthrough. |
| Provenance and documentation | The ledger separates `RESTORED_SOURCE`, `CIMMERIA_RECONSTRUCTION` and `CIMMERIA_AUTHORING` per beat. Runtime packets update `docs/content/mission-chains.md` and the zone UAT checklist; the final zone PR updates `docs/guides/unified-uat.md`. |

Pick each test's exact type with `TESTING.md`; chain replay, live-DB, wire-format, two-player fan-out or smoke may all be needed for one feature. Check the guard's revert proof. Every compiling `cargo` invocation uses `pwsh tools/build-lane/lane.ps1`; every seed reload and live-DB tier uses the worktree's database. The feature branch passes local CI-equivalent checks after each wave and again before its one campaign PR. Fresh read-only review covers each packet and each integrated wave; accepted fixes go to a separate fixer and a new reviewer. Never mark a source-only or SQL-only chain `Done` before the appropriate client evidence exists.

## Shared definitions

Every zone's packets cite these. Do not restate them differently.

- **Human gate.** A chain or dialog only a Human may use carries two `content_conditions` rows: `archetype gte '1'` and `archetype lte '4'` (Soldier 1, Commando 2, Scientist 3, Archeologist 4). Never `lt '5'`: a player whose archetype was never stamped (a GM-created character) reads as -1 (`crates/content-engine/src/conditions/mod.rs:333-338`) and passes `lt`. `sort_order` is not unique in `content_conditions` (the key is the serial `condition_id`), so two archetype rows are fine. Every packet that adds a Human-only chain tests a wrong-archetype player **and** an unstamped one (`Rig::new(pool, None)`); US-10 supplies the `UNSTAMPED` constant.
  - Dakara UD-04a's grant is intentionally wider (`gte '1'` and `lte '6'`, admitting Asgard 5 and Goa'uld 6) because the exit valve is a rescue, not a Human line. Its packet says so and tests archetypes 5 and 6 as admitted.
  - A Free Jaffa gate is `archetype eq '7'`; never widen an existing `gte 5` chain.
- **Line endings.** `docs/**/*.md` is CRLF. A seed or chain SQL file matches its neighbouring seed **in the working tree** (CRLF under autocrlf, LF in the index); do not convert a file, and never write a mixed file.
- **`CHAIN_FILES` merge order.** Appending to `CHAIN_FILES` (the seed-model chain list) from several packets conflicts on the same lines. Each zone's scaffold packet (SGC US-10, the Dakara, Omega and Beta equivalents) registers all of that zone's files at once, in the order of its file table. Later packets edit only their own file. A rebase conflict on `CHAIN_FILES` is resolved by keeping both lists in file-table order, never by dropping a line.
- **`id-allocation.md` policy.** [`id-allocation.md`](../four-zone-planning/id-allocation.md) belongs to the coordinator. A zone's ID packet (SGC US-00, Dakara UD-00, Omega UO-00, Beta UB-00) writes the amendment rows it needs into its own ledger as a request, labelled "request; the coordinator merges into id-allocation.md", and never edits that file. A block is valid for dispatch once the coordinator has merged the request.
- **Seed sequence footers.** A seed that inserts explicit ids into `entity_templates`, `spawnlist`, `item_lists` or `item_list_items` ends with a `setval` footer in the existing `GREATEST((SELECT MAX(id) ...), (SELECT last_value ...), <top of the block>)` form, so `seeded_sequences_allocate_past_every_seeded_row` (`live_db_seed_sequences.rs`) keeps passing. The packet that adds the rows adds the footer and names that test in its lane list.

## Cross-zone handoff contracts

| Boundary | Producing owner | Receiving proof |
|---|---|---|
| SGC mission 1570 gate to Dakara | Union SGC owns the Human SGC offer, departure and once-only handoff. | Dakara_E1 owns arrival and local SG-18/Dakara story; both sides test the same persisted character state. |
| Free Jaffa level-1 Dakara bridge | Union Dakara owns the newly authored start and truthful route into recovered level-3–5 missions. | SGC receives only later support/aftermath states; the Jaffa does not inherit the Human SGC tutorial. |
| Dakara return via 1651 and 1654 | Dakara_E1 owns departure after local beats. | Union SGC owns Hammond/Carter support and SGC aftermath interactions; tests distinguish Human from Free Jaffa dialogue. |
| SGC 1655 to Omega | Union SGC owns shutdown offer, race-specific letter and gate departure. | Union Omega owns arrival, beam platform, Daniel Jackson or Bra'tac letter delivery and onward 1423 or 1500 offer. |
| Asgard post-Pertho to public Omega | Union Omega owns the new private prologue and remote Council ruling in existing geometry. | Public Omega receives an Asgard with no falsely completed Pertho or physical-Council objectives and grants a truthful Beta-bound induction. |
| Omega to Beta E1 | Union Omega owns distinct Human 1423, Free Jaffa 1500 and Asgard/481 outbound offers and travel. | Union Beta owns the three local receiving steps, faction checks, no double grant and safe return. |
| Asgard 421 infirmary lead | Beta UB-00 first proves whether “SGC Administrative Complex Infirmary” is a Beta-local named space or a separate world; 421 remains held meanwhile. | If offworld, the appropriate SGC/other zone coordinator must accept a signed receiver and return fixture before Beta dispatches 421. No Beta-only credit substitutes for the trip. |
| Later Beta visits | Beta Visit 2 and world-60 E2 are researched separately after first-visit acceptance. | They are not required for the four Union campaigns' first-convergence completion. |

An upstream zone can merge its PR before a receiving zone is complete if the handoff contract and receiving fixture are already sealed. The receiving zone must then test the real inbound state before calling its own PR complete. Any temporary fixture or GM shortcut is identified as such in the worknote; it is not end-to-end UAT.

## Deferred execution and close-out

The four Union plans may be researched further while the first four Praxis-side zone continuations finish, but no new Union implementation packet starts until their completion gate is satisfied. The previously active Dakara_E1 rebuild keeps its own historical ledger; its work is reconciled, not restarted by this plan. Implement the Human route first, then Free Jaffa and Asgard extensions. Each zone's close-out updates its ledger, mission documentation and tester checklist; shared status docs change once in a Union close-out packet after the four zone PRs, or in an explicitly owner-approved partial close-out. Record source gaps and `UATPending` honestly. Retire every packet and campaign worktree after its reviewed commit or merged PR as the inherited contract directs.
