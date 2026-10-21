# Cellblock boot lock (campaign ledger)

Status: designed 2026-10-20; owner decisions applied 2026-10-20 (OQ-7, OQ-8); nothing built. Packets: [work-packets.md](work-packets.md). Resume card: [RESUME.md](RESUME.md).

## Purpose

The owner's request: when a fresh Praxis character first starts in Castle_CellBlock, put a boot prop on the floor at the spawn point. The player cannot move until they right-click that prop and win the minigame. A win releases movement.

Today mission 689 ("Prison Boot Lock", internal and hidden) exists, but it locks nothing. Its minigame trigger is an `item_use` on the worn boot that the client never sends. This campaign makes the lock real on the server alone, with no client patch:

- a clickable prop;
- a new content verb, `movement_lock`;
- a movement gate that snaps a held player back to the anchor and tells them why;
- seed edits to chains 1023-1025, plus two new chains.

## Findings (verified 2026-10-20)

The read-only audit was treated as a claim. Each row below was checked against the repo; the packet column names the packet that acts on it.

| # | Finding | Evidence | Packet |
|---|---|---|---|
| F1 | Chains 1022-1025 exist as the audit said. 1022 accepts 689 on `player_loaded`. 1023 launches ability 1597 while 689 is not completed. 1024 starts Livewire on `item_use 3438`. 1025 (no trigger, Livewire victory) launches 1598 and completes 689. | `db/resources/Content/Seed/castle_cellblock_chains.sql:149-210` | BL-06 |
| F2 | **The player is not frozen, confirmed.** The tester saw "boot is on but no minigame, can just walk". No `item_use 3438` ever reached the server. The client does not offer "use" on an equipped boot, so chain 1024 can never fire. | `docs/analysis/playtests/2026-09-18-colo-castle/README.md:64`, `screenshot-evidence.md:78,83` | BL-06 |
| F3 | Abilities 1597/1598 do nothing. Effects 1939, 1942 and 3081 have `script_name = NULL` and are single-shot, so `register_active_effect` returns early. No flag is set, no movement is rejected and nothing goes on the wire. The seed's own comment says the same thing. | `castle_cellblock_chains.sql:101-131` | BL-06 (kept, see OQ-6) |
| F4 | There is no boot entity or interactable in world 12 for this mechanic, and no boot `StaticMesh` in the client's cooked packages. `Ga-Props.GA-PuzzleStation00` (template 245) is proven to render. Template 303 (`DebugHub_LivewireTerminal`, spawn 403) proves that an `INT_MinigameLivewire` (256) `being` prop takes a right-click in this same room. | `entity_templates.sql` rows 245 and 303; `spawnlist.sql:1086`; mesh survey (architect research agent) | BL-06 |
| F5 | No movement gate exists for content today. `apply_client_position_update_at` runs these steps in order: the non-finite reject, the GM bypass (`movement_unrestricted`), then layers 1-4. There is no hook for a content hold. | `crates/cell-world/src/cell/space_manager/client_move.rs:153-262` | BL-02 |
| F6 | `BSF_MOVEMENT_LOCK` (`1<<6`) exists and is ref-counted through `update_state_flag`. Ring transport, death and Stun already write it. Whether the client honours it, and whether right-click still works while it is set, is known only from static RE; neither has ever been lab-verified. | `crates/cell-world/src/cell/ring_transport/wire_helpers.rs:123-180` | BL-05, BL-08 |
| F7 | The server snap-back exists. The validator already corrects rejected clients with `CellToBaseMsg::TeleportPlayer { position, prev_pos: position }`, which the base turns into `BASEMSG_FORCED_POSITION`. Each snap is a base DB update, so it must be throttled. | `crates/cell/src/cell/service/base_messages/movement.rs:249-281` | BL-03 |
| F8 | Only a Livewire **Victory** runs `on_victory_chains`. Loss, cancel and timeout run nothing, so the player can retry at once. A stale pending session refuses a new start for up to 180 s and says so. Interact range is 5.0 u. | minigame advisor research; chain 1016/1041 precedent | BL-06 |
| F9 | Goa'uld (archetype 6, char defs 10 and 19, the `NON_CANONICAL_BLOCKED_LEGACY` holding state) start at the same stasis-chamber point, but without boots: item 3438 is forced only for the 10 non-Goa'uld character definitions. The hold does not depend on the worn item. It keys on mission 689 and the prop, and the anchor guard passes at that start point, so a Goa'uld is held like every other character (D-BL8, owner). | `docs/analysis/castle-cellblock-rebuild/audit.md:23,55`; `db/resources/Archetypes/Seed/char_creation.sql:35,37` | BL-06 |
| F10 | `interaction_type` bits are cell-local and lost on a fresh instance. Chain 1045 is the established `player_loaded` restore pattern. | `castle_cellblock_chains.sql:1148-1162` | BL-06 |
| F11 | `client_move.rs` is 716 lines, over the 700 hard cap, so it must be split before the gate goes in. | `wc -l` | BL-01 |
| F12 | Id collisions on open branches: <ul><li>`content/cellblock-campaign` takes chains 1026-1028 and 1036-1037;</li><li>the four-praxis castle branches take templates 412-425 (sparse);</li><li>templates 440-479 belong to Dakara.</li></ul>Free on `main` and on every branch updated in the last 30 days (1,117 refs scanned): chains 1047 and 1048, template 1250, spawn 284, tag `Cellblock_PrisonBootControl`. | `git grep` over recent refs; `docs/analysis/four-zone-planning/id-allocation.md:17,47` | BL-06 |
| F13 | Every spawn in the stasis room must be `is_stationary`. A live-DB guard enforces it. | `crates/cell-catalog/src/cell/spawner/tests/live_db_debug_hub_stationary.rs` | BL-06 |
| F14 | The `interact_tag` linter needs a single-line `set_interaction_type` row naming the tag somewhere in the same seed file. | `crates/content-engine/tests/it/interact_tag_linter/mod.rs:46-80` | BL-06 |
| F15 | `name_id` must be a shipped client moniker. The `display_name` column sends a literal nameplate instead. 7550 is the client's "Terminal". | `docs/analysis/dakara-e1-rebuild/worknotes/DK-03.md:140`; `entity_templates_beta_e1.sql:27` | BL-06 |
| F16 | The player spawns on respawner 8 "Stasis Chamber", about 1.4 u from the room centre (-333.03, 73.472, -227.32). Debug-hub NPCs stand 5.5 u or more away, so the centre is clear. | `spawnlist.sql:1063-1071` | BL-06 |

## Decisions

| Id | Decision | Status | Reason |
|---|---|---|---|
| D-BL1 | **The prop:**<ul><li>template **1250**, `being`, mesh `Ga-Props.GA-PuzzleStation00`, body `GLB_Components.WorldObject_Small`, `interaction_type 0`, `name_id 7550`, `display_name 'Prison Boot Control'`;</li><li>spawn **284** in world 12, tag `Cellblock_PrisonBootControl`, at the room centre (-333.03, 73.472, -227.32), `is_stationary true`.</li></ul> | Decided; mesh and name: owner may override (OQ-1) | F4, F13, F15, F16. The client has no boot mesh; a proven console mesh at the player's feet is the closest match that renders. |
| D-BL2 | **The interaction:**<ul><li>Chain 1024's trigger becomes `interact_tag 'Cellblock_PrisonBootControl'`.</li><li>New chain **1047** (`player_loaded`, 689 not completed) sets `|256` (Livewire) and `|1073741824` (quest glow) on the prop.</li><li>Chain 1025 clears both bits.</li></ul> | Decided | F2, F10. This is the fallback the C00 seed comment already named. A loss or cancel keeps the bits, so the player can retry. |
| D-BL3 | **A new content verb, `movement_lock`,** with ops `hold` and `release`. `hold` pins the player to a **fixed seeded anchor** (`at`, the stasis-chamber respawner point). It does so only when the prop tag is live in the player's space **and** the player is within `max_distance` (3.0 u, horizontal) of `at`; otherwise it is skipped at info level. A `hold` while already held does nothing. `release` removes the hold and always says its message. | Decided | The server-authoritative fix needs no client patch. A fixed anchor cannot drift: re-anchoring at the player's position would let each relog walk the anchor up to 1 u further. The two guards fail open: the hold is skipped when the prop is missing, or when a legacy character stands elsewhere in the Cellblock. |
| D-BL4 | **The gate:** `SpaceManager::check_movement_hold` runs right after the GM bypass. A move within 1.0 u (horizontal) of the anchor falls through to the normal layers. Anything else returns `ClientMoveOutcome::Locked`, which:<ul><li>writes no position and does not touch the snap-back budget;</li><li>snaps the client to the anchor at most every 500 ms;</li><li>repeats the hold message at most every 5 s;</li><li>counts in `movement_hold_snaps_total`.</li></ul>A hold from another space is discarded. | Decided | Reject-and-snap is the proven server primitive (F7). Throttling bounds the base DB writes, and the 1 u slack lets the player turn and shuffle without being snapped. |
| D-BL5 | Layer A (the client-side flag) is staged. Every seed starts with `client_flag: false`, so the hold never sets `BSF_MOVEMENT_LOCK`. BL-08 flips it to `true` only after BL-U2 proves both that the client stops input and that right-click still works while the flag is set. | Decided; the flip is BlockedDecision (OQ-3) | F6: static RE only. If the flag blocked the right-click, players would be soft-locked. |
| D-BL6 | **Persistence and release:**<ul><li>Chain 1023 keeps `launch_ability 1597` (sort 0) and adds the `hold` (sort 1), so every relog re-applies the hold until 689 completes.</li><li>Chain 1025, in order: launch 1598, `release` with its message, clear `~256`, clear `~1073741824`, `complete_mission 689`.</li><li>New chain **1048** (`mission_completed '689'`) runs a silent `release`, which covers a GM completing the mission.</li></ul> | Decided | Mission 689 is already persisted per character (C00). The hold itself is cell-local and needs no new table. |
| D-BL7 | **Storage and files:**<ul><li>`MovementHold` lives in `CellEntity.extensions` (no `entity_struct.rs` change).</li><li>BL-01 splits `client_move.rs` first.</li><li>The verb type, its loader and its executor each get their own new file.</li></ul> | Decided | F11; the 500/700 file caps. |
| D-BL8 | **No archetype exclusion.** Goa'uld are held in the boot lock like every other character. Chains 1022-1025, 1047 and 1048 carry no `archetype` condition. ~~Goa'uld are excluded: chains 1023, 1024 and 1047 gain `archetype neq '6'`.~~ | Decided by owner 2026-10-20 (OQ-7); supersedes the original exclusion | F9: the hold is a server hold on mission 689 and the prop, not on the worn boot, and Goa'uld spawn on the same anchor. Caveat: a Goa'uld has no boot item, but the hold and release lines still read right, because the prop is the prison boot control. |
| D-BL9 | **Ids:** chains 1047 and 1048, template 1250, spawn 284. BL-06 re-runs the collision scan before it commits, and records chains 1047-1048 in `id-allocation.md`'s Cellblock row. | Decided | F12 |
| D-BL10 | **Ship gate:** wave 1 ships as one PR from the integration branch, and it merges **only after BL-U1 passes in the lab**. | Decided | If the prop fails to render or to take a click on colo, every new Cellblock character is stuck, with only GM escapes. The lab must prove the click before the lock goes live. |
| D-BL11 | **Player-facing text** (lives in the seed, so it is editable). Hold: "Your prison boot is locked to the floor. Right-click the boot control beside you and win the Livewire game to work it loose." Release: "The prison boot unlocks. You can move again." | Decided; owner may override (OQ-2) | Movement refusals need visible feedback. The text names the next action. |

## Soft-lock escapes

| Situation | What happens |
|---|---|
| Minigame lost, cancelled or timed out | Nothing runs (F8). The bits stay, so the player right-clicks again. The prop is about 1.4 u from the anchor (a live-DB test pins it at most 3.0 u), so with the 1 u slack it stays inside the 5 u interact range. |
| Stale pending minigame session | The server answers with the existing "already in progress" line for up to 180 s, then a new start works. |
| Disconnect and relog | The cell entity is rebuilt without the hold. Chain 1023 re-applies it at the same fixed anchor (the player is within 1 u of it), and chain 1047 restores the click bits. |
| Prop missing, or player far from the anchor (legacy character, failed spawn) | `hold` is skipped and logged at info level (`hold_skipped_no_anchor` / `hold_skipped_far_from_anchor`). This fails open: the player can walk. |
| Death or respawn | The respawn lands on the same stasis chamber. The hold is unchanged and its anchor is the same point. |
| GM rescue | `/gmsetfly` (`movement_unrestricted`) bypasses the gate. Completing mission 689 runs chain 1048, which releases the hold. |
| Kill switch | Set chain 1023's `enabled` to `false` in the seed (one-line PR). The colo DB rebuilds from seeds on deploy. |

## Lab needs (ask the user first; never start the lab unasked)

- **BL-U1** (before the wave-1 PR merges), on a local server built from the integration worktree. The user builds it.
  - The prop renders at the spawn and shows its nameplate and glow.
  - A right-click opens Livewire.
  - Walking away snaps the player back, and the hold line shows.
  - A loss lets the player retry.
  - A relog mid-lock re-locks with the prop still clickable.
  - A win releases the player and the prop stops glowing.
  - A Goa'uld character is held and released the same way.
- **BL-U2** (after BL-08), with `client_flag: true`: does the client stop movement input, and does right-click still open Livewire?

## Open owner questions

| Id | Question | Default (applied unless the owner says otherwise) | Blocks |
|---|---|---|---|
| OQ-1 | Prop mesh and nameplate: `GA-PuzzleStation00` named "Prison Boot Control"? Alternatives, each needing a lab load test: `CA-Props.CA-SecurityLock00`, `Ca-StasisCamber_Wallstation00`. | Use the default | BL-06 (BlockedDecision, but its default is Ready) |
| OQ-2 | The exact hold and release lines (D-BL11). | Use D-BL11 | BL-06 (same) |
| OQ-3 | Turn on the client-side flag (Layer A) once BL-U2 passes? | Leave it off | BL-08 (BlockedDecision) |
| OQ-4 | Auto-release after N lost games? | No | Nothing |
| OQ-5 | After release, leave the prop as inert scenery or despawn it? | Leave it, without click bits | Nothing |
| OQ-6 | Keep firing the no-op abilities 1597 and 1598? | Keep: they are the original ability records and cost nothing | Nothing |
| OQ-7 | Exclude Goa'uld (archetype 6)? | **Decided by owner 2026-10-20: no.** Goa'uld are held too (D-BL8). | - |
| OQ-8 | Reuse Livewire? D-CB14 already said yes. | **Decided by owner 2026-10-20: Livewire.** The owner's note called this OQ-5; OQ-5 (the prop after release) keeps its default. | - |
| OQ-9 | Coordinate with the cellblock continuation (four-zone) campaign: its "689 boot gate relog" probe is superseded by this campaign. | Tell its coordinator when wave 1 merges | Close-out |

## Cross-campaign coordination

- `content/cellblock-campaign` and the four-zone Cellblock packets edit the same seed files (`castle_cellblock_chains.sql`, `spawnlist.sql`). BL-06 inserts its rows inside the mission 689 block, not at the end of the file, to keep conflicts local.
- That branch moves the "next free" note in the header (line 53 on `main`). Whichever PR merges second resolves that line by hand.
- `docs/analysis/four-zone-planning/cellblock-work-packets.md` lists a "689 boot gate relog" probe and `mission_689.rs` guards. This campaign replaces both. Note it in the close-out (OQ-9).

## Packets

| Packet | Wave | Title | Agent | Depends on | Status |
|---|---|---|---|---|---|
| BL-01 | 1 | Split `client_move.rs` (no behaviour change) | packet-coder | - | Ready |
| BL-02 | 1 | `MovementHold` and the `Locked` gate | packet-coder | BL-01 | BlockedDependency |
| BL-03 | 1 | `EntityMove` handler: snap-back and feedback for `Locked` | packet-coder | BL-02 | BlockedDependency |
| BL-04 | 1 | Content-engine `movement_lock` type and loader | packet-coder | - | Ready |
| BL-05 | 1 | Executor for `movement_lock` | packet-coder | BL-02, BL-04 | BlockedDependency |
| BL-06 | 1 | Seeds: prop, chains 1023-1025, 1047, 1048, plus live-DB tests | packet-coder | BL-04; OQ-1, OQ-2 defaults | BlockedDependency |
| BL-07 | 1 | Docs | documentation-writer | BL-01..BL-06 | BlockedDependency |
| BL-U1 | 1 gate | Lab UAT on the integration build (ask the user) | orchestrator + lab-driver | BL-07, the wave review | BlockedDependency |
| BL-08 | 2 | Seed flip: `client_flag: true` | packet-coder | wave 1 merged, OQ-3 | BlockedDecision |
| BL-U2 | 2 | Lab UAT of Layer A (ask the user) | orchestrator + lab-driver | BL-08 | BlockedDependency |
| BL-09 | 2 | Close-out: status docs and unified UAT | documentation-writer | wave 1 merged (and BL-08 merged or dropped) | BlockedDependency |
