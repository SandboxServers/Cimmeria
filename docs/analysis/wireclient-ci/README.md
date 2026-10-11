# Wireclient in CI

> Type: ledger. Audience: the coordinator, packet workers and reviewers.
> Opened 2026-10-10 against `main` @ `c05c0638a`. Prefix `WC-`. Tracking issue:
> [#281](https://github.com/SandboxServers/Cimmeria/issues/281). Related:
> [#1341](https://github.com/SandboxServers/Cimmeria/issues/1341) (first-login
> flush tail drop). Packet specs: [work-packets.md](work-packets.md). ADR:
> [wireclient.md](../../architecture/wireclient.md).
>
> **Campaign status (2026-10-10): planned, nothing built.** Five owner
> decisions (D-WC2, D-WC3, D-WC6, D-WC7, D-WC8) gate the packets marked
> BlockedDecision. WC-01 and WC-02 can start now.

## Purpose

Finish the headless wire client far enough that one real gameplay stage, the
Praxis start (mission 622 "Arm Yourself!"), runs end to end on the wire
against a spawned server, and get every wireclient end-to-end test running in
CI reliably enough to gate merges.

That is a slice of the ADR's phases, not all of them:

| ADR phase | This campaign |
|---|---|
| 3: entity mirror and semantic decoder | The mirror, and decoders for the messages the Praxis start needs (WC-05 to WC-09, WC-11). The general behaviour-trace diff is not in scope. |
| 4: Castle Cellblock script | Its first stage, the Praxis start (WC-12, WC-14). The rest of Castle Cellblock is not in scope. |
| 5: combat; 6: minigame force-victory hook | Not in scope. |
| 7: nextest profile and CI | All of it (WC-03, WC-04, WC-13, WC-16). |

Out of scope: a non-GM movement path (D-WC2 records it as a follow-up), the
Castle Cellblock steps past mission 622, a replay engine for
`session_trace::Trace`, and any client patch.

## What was found

Against `main` @ `c05c0638a`. "Fixture #n" is the n-th record (0-based) of
`crates/wireclient/tests/fixtures/praxis_start_tap.json`.

| # | Finding | Packets |
|---|---|---|
| F1 | `decode_bundle` stops at an unknown message id or a truncated header and returns the messages before it, with only a `warn`. That is the same failure shape as #1341 (the client drops the rest of a bundle). A test built on it passes with a silently lost tail. | WC-01 |
| F2 | `decode_bundle` assumes idbase 61 for every `0xBD` message. NPC targets use 62 (`IDBASE_NPC_DEFAULT`), so an NPC method at index 61 or above is mislabelled. Nothing records the raw sub-slot byte. | WC-01, WC-11 |
| F3 | `tests/it/support::wait_for` throws away every message that does not match its predicate, so a test cannot assert the order of several answers to one call, and two waits in a row can miss a message. | WC-12 |
| F4 | `GameSession::enter_world` consumes the world-entry bundles (the `mapLoaded` entity data with `onUpdateItem`, the player's inventory) and returns nothing, so a script cannot learn the starter pistol's instance id from the wire. | WC-10 |
| F5 | `CREATE_ENTITY` carries no template id: only the entity id, an alias byte, the class id and two unknown bytes (`compose_create_entity_base_body`). Identity arrives in the AoI cascade: `onStaticMeshNameUpdate` (method 0), `onBeingNameIDUpdate` (11), `InteractionType` (3). Corporal Frost's corpse is template 14: static mesh `CA-Props.CA-PrisonerCorpse00`, name id 7031, class `being`. | D-WC1, WC-09, WC-11 |
| F6 | Every client cell call in the fixture carries entity id **0** in its 4-byte prefix (fixture #1, #4, #8, #19, #65; the tap records the raw inbound payload). The wireclient builders and tests send the player's own id. The server ignores the prefix either way (`cell_arms.rs` uses the session's `player_eid`). | D-WC10, WC-02 |
| F7 | The ADR's "`moveItem` (msg 166) raw `[0, <pistol item id>, 3, 1, 1]`, field meanings not confirmed": msg 166 is `0xA6`, cell method 38 in direct encoding. The leading `0` is the entity-id prefix of F6. The arguments are `moveItem(itemId 10345, targetBag 3, targetSlot 1, quantity 1)` per `cell-method-dispatch-table.md` row 38 and `handle_move_item` (the slot is 1-based on the wire; the cell subtracts 1). | WC-02 |
| F8 | The ADR's "an undecoded method 27 to a nearby NPC" (fixture #6, one byte `0x03` to entity 100745) is, by the SGWMob table in `client-method-dispatch-table.md`, `onAggressionOverrideUpdate(INT8 3)`. Not verified against the binary. Index 27 is `onSystemCommunication` only on a player. | WC-05 |
| F9 | The tap's inbound records keep the entity-id prefix and the `0xBD` sub-slot byte in `args_hex`; its outbound records hold the arguments only. Decoder tests feed outbound `args_hex` as is; builder tests compare against the whole inbound payload. | WC-02, WC-05 |
| F10 | The fixture's `decoded` field is `cimmeria-wire-log`'s output. Its `onSequence` decoder ignores the last field: the 26-byte body ends with `INT32 InstanceId` (0). | WC-07 |
| F11 | The ADR's step 9 answers omit `onStatUpdate`, a second `onKnownAbilitiesUpdate` (`[579, 597, 1218, 592, 594, 708]`), `onEntityProperty(3 AmmoTypeId, 1)`, and a later `onSequence 1873` about 10 s after the move (fixture #70, #71, #73, #77). Omissions, not contradictions. | WC-14 |
| F12 | Floats in the fixture are not always the decimal shown: the region trigger's z (fixture #8) is `0xC354CCCC` (-212.79999), one ulp from `-212.8f32` (`0xC354CCCD`) that `gmGotoXYZ` carries (fixture #4). Byte pins must take positions from the fixture's bytes. | WC-02 |
| F13 | The first-login AoI hold applies only when `sgw_player.first_login != 0` (default 1). The existing tests insert characters with `first_login = 0`, so none of them exercises the hold or its flush. A wire-created character does. `cancelMovie` (cell 108, `WSTRING movieName`) releases the hold at once (`release_on_cancel`). | D-WC3, WC-10, WC-14 |
| F14 | Every live-DB test in `tests/it/` reads `DATABASE_URL` itself, which bypasses the per-slot clone resolver (`cimmeria_test_support::database_url`). Under the `live-db` nextest group they would all share the template database. The `database_url_is_only_resolved_by_the_gate` guard scans `src/` only, so it does not see them. | WC-04 |
| F15 | No wireclient end-to-end test runs in CI. `tools/test-live-db.sh` runs `--lib` only, so the `tests/it` binary is never built against a database, and `live-db-test.ps1` cannot run it locally either. | WC-03, WC-13 |
| F16 | The ADR's Phase 7 plan, a profile that serialises the suite because "parallel runs would corrupt shared state", is out of date. nextest runs each test in its own process (so the `chdir` and globals are per test), every port is ephemeral with retry, and the `live-db` test group gives each running test its own database clone. | D-WC6, WC-03 |
| F17 | #1341's own evidence: the first-login flush packs one 11-byte `CREATE_ENTITY` plus one 26-byte avatar update per NPC into an 889-byte single-packet bundle. Any wire guard of the form "no quest-critical introduction after an avatar update in the first packet" fails on today's server, so the guard can land only with #1341's mitigation. The wire client itself cannot be fooled by the residue (it parses each message from its own header), which is why it needs a packet-level recorder, not a decoder change, to see the hazard. | D-WC8, WC-15 |
| F18 | The ADR's TL;DR says the wire client drives the protocol "the way the original Flash client would have". The SGW client is Unreal Engine 3 with CEGUI; only the minigames are Flash. Wording only. | WC-17 |

## Decisions

| ID | Status | Decision | Reason |
|---|---|---|---|
| D-WC1 | PROPOSED (coordinator) | **Entity identity is wire-observable.** The mirror identifies an NPC by what the client is told: static mesh name, body set, name id, class id, interaction type and position. A test that wants "template N" reads that template's `static_mesh` and `name_id` from `resources.entity_templates` and queries the mirror with them. No server introspection (no reading the cell's `SpaceManager`). | F5. Runtime entity ids change per space instance (Frost was 100402, 100659, 100705 and 100751 in four runs). Reading server state would make the wire client a server-side test again. |
| D-WC2 | **BlockedDecision** (owner). Recommended: **(a)** | **How the script reaches Frost and the Guard's body.** (a) `gmGotoXYZ` (cell 163) from a GM sentinel account (`accesslevel` 2), exactly as the capture does; a non-GM walking path is a follow-up issue. (b) `avatarUpdateExplicit` steps at walking speed from the spawn point, as a non-GM player would. | (a) matches the capture byte for byte and needs no movement validation reasoning. (b) is closer to a real player but depends on the server's speed and navmesh checks, which the capture never exercised. |
| D-WC3 | **BlockedDecision** (owner). Recommended: **(a)** | **The first-login AoI hold.** (a) The script answers `onPlayMovie` with `cancelMovie` (what a player pressing Esc sends), so introductions flush within a second. (b) The script waits out the 16 s `HOLD_DURATION`. | (a) keeps the test about 15 s faster and exercises the real client path. The timeout path is already covered by `cinematic_aoi_hold/tests.rs`. Either way the flush goes through `flush_deferred_aoi`. |
| D-WC4 | PROPOSED (coordinator) | **The semantic decoders are written independently in `cimmeria-wireclient`**, typed, from `docs/protocol/client-method-dispatch-table.md`. They do not reuse `cimmeria-wire-log`'s `serde_json` decoders. Tests cross-check both against the fixture. | An oracle that shares the server's decoder cannot catch a bug in it (F10 is one). Typed fields give the script compile-time names. |
| D-WC5 | PROPOSED (coordinator) | **Strict decoding.** Any bundle the wire client cannot decode to its last byte, and any decoder that leaves trailing bytes, fails the test with the offset and the message id. Unknown method indices are not an error (most methods have no decoder); a known index with bad bytes is. | F1. The wire client must not be fooled by a dropped tail the way the real client is. |
| D-WC6 | **BlockedDecision** (owner). Recommended: **(a)** | **How end-to-end tests run.** (a) A `wireclient-e2e` nextest profile whose override puts every `cimmeria-wireclient` integration test in the existing `live-db` group, so each running test gets its own database clone and up to 8 run at once; `retries = 0`. (b) The ADR's original plan: a serialised profile (`threads-required = "num-test-threads"`) against one database. | F16. (a) is faster and uses machinery the live-DB tier already proves. The ADR's Phase 7 section is updated to match whichever is chosen. If CI memory pressure shows up in the WC-16 shakedown, `threads-required = 2` on the override halves concurrency without changing the design. |
| D-WC7 | **BlockedDecision** (owner). Recommended: **(a)** | **When CI runs it and whether it gates.** (a) Its own workflow, `.github/workflows/wireclient.yml`, path-filtered on `crates/**` minus the client, launcher and lab crates, plus `db/**`, `entities/**`, `data/spaces/**`, the toolchain and nextest config, and the runner scripts: in effect every server change. It reports but does not gate until WC-16's shakedown (10 green runs in a row), then its job joins `ship.py`'s `CONDITIONAL`. (b) A step in the existing `test-live-db` job: every code PR, but `ship.py` never waits for that job, so it never gates. (c) Path-filtered on `crates/wireclient/**` only. | (b) cannot block a regression. (c) misses the server changes the test exists to catch. (a) gates every server change once it has shown it is not flaky. |
| D-WC8 | **BlockedDecision** (owner, through #1341). Recommended: **follow #1341** | **The flush-shape guard.** WC-15 asserts, at the wire, the invariant #1341's chosen server mitigation guarantees (for example "every single-packet flush bundle holds one message", or "the first fragment of a fragmented flush bundle holds one message"). If #1341 chooses the client patch instead, WC-15 records in the ADR why the hazard is not testable from the wire (the wire client has no residue) and closes as not applicable. | F17. Today's flush fails any such invariant, so the guard can only land with the fix it guards. |
| D-WC9 | PROPOSED (coordinator) | **No retries.** A flaky end-to-end test is a bug, fixed or `#[ignore]`d with an issue link and the owner told, never retried. No fixed sleeps: every wait is on a message or a database condition, with a timeout and a failure message that prints the last 30 observed events. | Retries hide the races these tests exist to find. |
| D-WC10 | PROPOSED (coordinator) | **Client calls put entity id 0 in the cell-method prefix**, as the real client does (F6). The builders keep the `entity_id` parameter; the Praxis script passes `CLIENT_CALL_ENTITY_ID` (0). Existing tests keep sending the player id. | Fidelity: the wire client should send what a real client sent. The server ignores the prefix today; a future server that reads it is then tested against the real value. |
| D-WC11 | PROPOSED (coordinator) | **The Praxis character is created on the wire**: account base method `createCharacter` (`0xC3`) with char def 3 (Praxis Commando, male), the first choice of every `VIS_Optional` visual group, skin tint 0. Its `player_id` is read from `sgw_player` by the sentinel account id; `onCharacterList` is not decoded. | A wire-created character has `first_login = 1`, the real start profile and the real starter kit, which an inserted row does not (F13). Decoding the character list is not needed for this stage. |
| D-WC12 | PROPOSED (coordinator) | **Fidelity sends.** The script sends what the client sent besides the ten calls: the region trigger after the first teleport (step 2a), one `requestEntityUpdate` (`0x07`, `[u32 id]`, no cache stamps) per introduced NPC, and an unreliable `AUTHENTICATE` every 250 ms while waiting (the sparbot keep-alive, which also carries piggyback ACKs). `perfStats` is not sent. | The server's answers to the ten calls are what the test checks; the extra sends keep the session shaped like a real one (acks flowing, no inactivity reap). |

## Packets

| ID | Packet | Implementer | Size | Wave | Depends on | Status |
|---|---|---|---|---|---|---|
| WC-01 | Strict bundle decode, raw sub-slot byte, message offsets | packet-coder | S | 1 | none | Ready |
| WC-02 | Tap fixture loader and client-call builders, pinned to the capture | packet-coder | M | 1 | D-WC10 (proposed) | Ready |
| WC-03 | Runner: `wireclient-e2e` nextest profile and `--wireclient` mode in `test-live-db` | packet-coder | S | 1 | D-WC6 | BlockedDecision |
| WC-04 | `tests/it` live-DB isolation: slot URL, test logging, no direct `DATABASE_URL` | packet-coder | M | 2 | WC-03 | BlockedDependency |
| WC-05 | Semantic decoder skeleton, primitives, dialog family | packet-coder | M | 2 | WC-02 | BlockedDependency |
| WC-06 | Mission family decoders | packet-coder | S | 3 | WC-05 | BlockedDependency |
| WC-07 | Ability, sequence and movie decoders | packet-coder | S | 3 | WC-05 | BlockedDependency |
| WC-08 | Inventory family decoders | packet-coder | S | 3 | WC-05 | BlockedDependency |
| WC-09 | Entity-introduction and chat decoders | packet-coder | S | 3 | WC-05 | BlockedDependency |
| WC-10 | Character creation on the wire, world entry split and recorded | packet-coder | M | 3 | WC-02, WC-04 | BlockedDependency |
| WC-11 | Entity mirror and query | packet-coder | M | 4 | WC-01, WC-05, WC-08, WC-09 | BlockedDependency |
| WC-12 | `ScriptSession`: lossless event log, ordered waits, keep-alive | rust-gameserver-dev | M | 5 | WC-10, WC-11 | BlockedDependency |
| WC-13 | CI workflow `wireclient.yml` (reporting, not gating) | packet-coder | S | 3 | WC-03, WC-04, D-WC7 | BlockedDecision |
| WC-14 | Praxis-start end-to-end test | rust-gameserver-dev | L | 6 | WC-06, WC-07, WC-12, D-WC2, D-WC3 | BlockedDecision |
| WC-15 | First-login flush-shape guard (#1341) | rust-gameserver-dev | M | 7 | WC-14, D-WC8, #1341's mitigation | BlockedDecision |
| WC-16 | Shakedown, then gate merges on the job | packet-coder (coordinator runs the shakedown) | S | 7 | WC-13, WC-14 | BlockedDependency |
| WC-17 | Close-out: ADR, TESTING.md, status docs, #281 | documentation-writer | M | 8 | all | BlockedDependency |

Size: S under about 40k tokens, M 40k to 70k, L 70k to 100k.

Waves (packets in one wave touch disjoint files and run in parallel):

1. WC-01 (`bundle.rs`), WC-02 (`tap_fixture.rs`, `calls.rs`), WC-03 (`.config/nextest.toml`, `tools/test-live-db.*`).
2. WC-04 (`tests/it/support`, the live test modules), WC-05 (`src/semantic/`).
3. WC-06, WC-07, WC-08, WC-09 (one file each under `src/semantic/`), WC-10 (`world_entry.rs`, new test module), WC-13 (workflow).
4. WC-11 (`src/mirror/`).
5. WC-12 (`src/script.rs`).
6. WC-14.
7. WC-15 (when #1341 has decided), WC-16.
8. WC-17.

## Dispatch rules

- **Workers.** One packet each, in its own worktree and test database:
  `pwsh -NoProfile -File tools/build-lane/mk-worktree.ps1 wireclient-ci/<packet>-<slug> <worktree>`.
  `packet-coder` (Haiku) for packets marked so; `rust-gameserver-dev` for
  WC-12, WC-14 and WC-15; `documentation-writer` for WC-17. The brief carries
  the worktree path, the packet section of [work-packets.md](work-packets.md),
  the contract section, and the commit subject with the attribution lines.
- **Review.** Each finished packet gets a Sonnet `packet-reviewer` on its
  commit range. WC-14 and WC-15 also get `aoi-witness-broadcast` (the flush
  and introductions) and `testing-validation-engineer` (does the guard fail
  when the fix is reverted). Review fixes go to a fresh worker or the
  coordinator, never back to the original implementer.
- **Shell.** PowerShell only: no bash, WSL or Git Bash, no direct `cargo`, no
  `git worktree prune`, no `git stash`. Every compiling command goes through
  `pwsh -NoProfile -File tools/build-lane/lane.ps1`.
- **Ship.** `python tools/build-lane/ship.py pr -C <worktree> -m <msg>`, then
  `python tools/build-lane/ship.py merge <PR> --retire <worktree>` once the
  build-proving CI jobs pass. Update this table and write
  `worknotes/<packet>.md` when anything is left over.
- **Shared files.** Only WC-17 edits `docs/architecture/wireclient.md`,
  `TESTING.md`, `crates/README.md`, `docs/gap-analysis*` and
  `docs/project-status.md`. Packets record their doc deltas in their worknote
  for WC-17 to fold in, except the rows the packet's own section names.

## Review outcomes

None yet. Where merged code differs from the packet specs, record it here;
the code is then the reference, not the spec.
