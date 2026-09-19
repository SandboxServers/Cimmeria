# Colo playtest 2026-09-18 — replacing the Discord chat with telemetry

Part of the [2026-09-18 colo playtest report](README.md). Section numbers continue from that document so existing references (code comments, PR descriptions) still resolve.

## 9. Replacing the Discord chat with telemetry

§6 lists seams per defect. This section asks a different question: **what did the chat give the investigation that
the logs could not, and what would have to be emitted so the next session can be reconstructed with no chat at
all?** The chat supplied six kinds of information.

### 9.1 What the chat contributed

| # | What the chat gave us | Example tonight | Can telemetry replace it? |
|---|---|---|---|
| 1 | **A pointer in time** — "look here, something is wrong" | "moonwalking in the air" at 7:03 PM turned 110k rows into a 3-second window | Yes — tester bookmark (§9.2) |
| 2 | **Non-events** — things that should have happened and did not | "other guard didn't aggro", "marsh doesn't follow", "cover isn't registered", "throne room doesn't recognise me", "can't dial", "no minigame" | Mostly — expectation seams + stall detectors (§9.3) |
| 3 | **What the client rendered** | backwards body, floating feet, dialog replaced too fast, heartbeat audio, "quest path not right" | Partly — log what we *told* the client (§9.4); the rest needs client telemetry (§9.6) |
| 4 | **Tester-side state that contaminates the data** | `Ghost enabled`, `.speed 300`, jumping | Yes — tag it (§9.5) |
| 5 | **Identity and build** | which account is Lomiada, which image was deployed, when | Yes — trivially (§9.5) |
| 6 | **Intent and judgement** | "should complete when crouching", "crystal should be a drop", "too many slap-packs", the work-order at 9:06 PM | No — this is feedback, not telemetry. The bookmark's free text is the right home for it |

Seven of the headline findings (H3, H4, H5, H7, H8, H9, H10) were *visible* in the logs alone; none of them was *found* until the chat
said where to look. Category 1 is therefore worth more than all the others combined, and it is the cheapest.

### 9.2 A tester bookmark: `.bug <free text>` (highest value, smallest change)

A dot-console command (GM `access_level`, same dispatcher as `.location`) that emits **one INFO event, target
`playtest.bookmark`**, carrying a server-side snapshot of everything a screenshot plus a chat line would have told us:

```
player_id, account_id, entity_id, character_name, archetype, level,
world_name, space_id, x, y, z, yaw, region_tags (all regions currently inside),
target_entity_id, target_npc_name, target_tag, target_x/y/z, target_yaw, target_ai_state, target_health,
active_missions = [(mission_id, step_id, [objective_id:status])],
nearby_npcs = [(npc_id, npc_name, tag, ai_state, x, y, z, yaw_rad, yaw_byte, nav_path_len,
                leg_seq, move_reason, target_id, follow_target_id, dist)]   -- within ~40 u, capped at 16
last_dialog_id, ms_since_last_dialog, last_interact_tag, last_interact_outcome,
gm_flags (ghost/fly if known, speed_multiplier), client_addr_hash,
note = "<free text>"
```

Every screenshot tonight would have been answerable from that one row: the guard's `yaw_byte = 0` while his bearing
to the player was negative; Zuritska's Y against the player's Y; both Zuritskas in `nearby_npcs`; Romney's position
against a region list that shows no connected region.

Two extensions that cost little:

- **`.bug` also force-enables verbose capture** for the named target (or all `nearby_npcs`) for the next 60 s —
  un-sampled `movement.npc` steps and per-tick `npc_ai` decisions for just those ids. This sidesteps the sampling
  problem entirely: full fidelity exactly where a human said it matters, nothing elsewhere.
- **Relay it to Discord** through the existing `cimmeria-discord` notifier (new `EventKind`, see the CLAUDE.md row
  for adding one) with a SigNoz deep link for ±60 s around the timestamp. The tester keeps the habit of talking in
  Discord — they just type it in-game — and every remark arrives already aligned to server time. Timestamps in the
  chat tonight were minute-granular; the defects were second-granular.

### 9.3 Non-events: expectation seams and stall detectors

Logs are naturally silent about what did not happen, and six of the tester's complaints were non-events. Two
complementary mechanisms:

**(a) Log the "no" at each decision point** — already specified in §6 and the appendices: `condition_failed` per
candidate chain, `auto_aggro_none`, `idle_not_ticked`, `follow_target_lost`, `hold_no_repath`,
`unhandled_interaction_flag`, `client_region_hint_missing`, `effect_noop`.

**(b) Detect the player being stuck, without knowing why.** These are behavioural signatures that were visible in
tonight's data in hindsight and would have flagged every soft-lock with no human involved. Target
`playtest.friction`, WARN, one event per episode (not per repeat):

| Signal | Rule of thumb | Tonight |
|---|---|---|
| `repeat_interact_no_effect` | ≥5 interacts on the same target in 60 s, all ending in a no-op outcome | 76 clicks on the DHD |
| `repeat_item_use_no_chain` | same item used ≥2× with `no chains matched` while a mission step references it | Ambernol at 7:06 PM |
| `step_stalled` | a mission step active > N min while the player keeps acting inside the step's area | take-cover (77 s), Throne Room (14 min), boot lock (forever) |
| `objective_never_completed` | mission completes with an optional objective untouched, logged at completion | flank 2725 / 2731, armory 4647 |
| `region_dwell_no_hint` | server-side containment says the player is inside a trigger volume, no client hint arrived | Throne Room after respawn |
| `escort_separated` | an NPC with `follow_target_id` is > 3× `follow_max_distance` from its target for > 10 s, or has not moved since being armed | Marsh (never moved), Zuritska (50–95 u behind) |
| `console_reject_streak` | ≥3 rejected dot-commands in 2 min — the tester is hunting for a command that does not exist | `.missionadvance`, `.advance_step`, `.mission advance` |
| `dialog_displaced` | a dialog replaced < 3 s after display | 2516 → 5859 in 0.6 s |
| `death_then_silence` | after a respawn, a previously chatty client message type goes to zero | region hints after 7:37 PM |

### 9.4 What we told the client (the server-side half of "what did it look like")

We cannot see the client's screen, but every visual complaint tonight traced to something the server *sent* and did
not record. Log the outbound intent, not just the internal state:

- `yaw_byte`, position variant (`0x10` / `0x18`), `physics_byte` on (sampled, or bookmark-boosted) UPDATE_AVATAR sends
- every `setMovementType` outcome (`sent` / `deduped` / `cleared`)
- dialog sends with `speaker`, `screen_count`, `replaced_dialog_id`, `ms_since_previous`
- mission-log mutations actually sent (accept / step / complete / remove), with `reward_xp` and `has_rewards`
- on respawn: exactly which state blocks were resent and which were not; state flags before/after
- the respawner id list offered, not just its count

### 9.5 Identity, build and contamination tags

- **`deployment.environment` is wrong on the colo today.** Every row in this session carries
  `deployment.environment = "dev"`, although `docker/compose.yml:72` sets
  `OTEL_RESOURCE_ATTRIBUTES=deployment.environment=colo`. `crates/server/src/otel.rs:176-183` always calls
  `.with_attribute("deployment.environment", deploy_env)` with a `"dev"` default, and an explicit builder attribute
  wins over the SDK's env detector — the comment at `otel.rs:197-202` asserts the opposite precedence.
  (`service.namespace = cimmeria` from the same env var *did* arrive, which is how we know the variable was read.)
  Colo and laptop data are currently indistinguishable by resource attribute. Fix: set `CIMMERIA_DEPLOY_ENV=colo`
  in the compose file, or only add the explicit attribute when `CIMMERIA_DEPLOY_ENV` is set.
- **Build identity.** Emit `service.version` (git SHA + image digest) as a resource attribute and in one startup INFO
  line. Tonight the only record that the build changed at 6:49 PM was a Watchtower post in Discord.
- **A `play_session_id`** minted at `playCharacter` and attached to every event for that player (span attribute or
  explicit field), with a `session.start` / `session.end` pair carrying `character_name`, `archetype`,
  `access_level`, `disconnect_reason`. Today correlation hops between `player_id`, `entity_id`, `account_id`, and
  several fields exist as both string and number types in SigNoz (`player_id`, `item_id`, `dialog_id`, `seq`), which
  silently splits aggregations.
- **Contamination tags** on `movement.player` samples and on any NPC decision whose target is that player:
  `access_level`, `speed_multiplier`, and `airborne` / `vy`-vs-ΔY disagreement (the only server-visible trace of
  client `ghost`). An NPC chasing a noclipping, 3×-speed GM is not evidence about pathing.
- **Names on every NPC row.** `movement.npc` rows carry only `npc_id`; resolving "which one was the guard" needed a
  cross-scope join. Add `npc_name` and `tag` everywhere an `npc_id` appears.

### 9.6 A session journal, and the client

- **`session.journal`** — one low-volume INFO stream per player with a fixed `event` vocabulary: `world_enter`,
  `mission_accept`, `step_advance`, `objective_complete`, `mission_complete`, `dialog_shown`, `minigame_result`,
  `item_grant`, `xp_grant`, `level_up`, `death`, `respawn`, `teleport`, `gm_command`, `bookmark`, `friction`. The
  92-row timeline in the appendix was assembled by hand from ~40 scopes with different field names; it should be
  `scope_name = 'session.journal' AND play_session_id = …`.
- **Client telemetry was absent, and nothing said so.** No `launcher.*` rows exist for this session. Add a
  `session.start` field `client_telemetry = true/false` (did a dev-session token accompany this login?) and a
  friction-style WARN when a GM-level tester plays without it. Facing, grounding, audio and UI layout are only ever
  *provable* from the client side; everything in §9.4 is the server's best proxy.

### 9.7 Order of work

1. `.bug` bookmark with snapshot (§9.2) — replaces the chat's most valuable function outright.
2. Fix `deployment.environment`; add `service.version` and `play_session_id` (§9.5).
3. `session.journal` (§9.6) — makes reconstruction one query.
4. `playtest.friction` detectors (§9.3b), starting with `repeat_interact_no_effect`, `step_stalled`,
   `escort_separated`.
5. The per-defect seams in §6, with bookmark-boosted verbosity replacing id-based sampling.
6. Outbound-intent logging (§9.4), then Discord relay of bookmarks.

### 9.8 What has shipped (PR `feat/playtest-bug-bookmark`)

| Item | Status |
|---|---|
| `.bug <note>` bookmark — header row + one row per nearby entity, joined on `bookmark_id` (§9.2) | **Shipped.** Targets `playtest.bookmark` / `playtest.bookmark.entity`; 60 u radius, nearest 32, selected target always included. The note also reaches the GM Discord channel through the existing console audit relay |
| `yaw_byte` — the facing actually transmitted — on bookmark rows and sampled `movement.npc` steps (§9.4) | **Shipped** |
| Step sampler observes every NPC (global counter, not `npc_id % 10`) (§6) | **Shipped** |
| Missing navmesh → WARN `movement.navmesh` (§4.3) | **Shipped** |
| `follow_no_path`, `follow_target_lost`, `follow_dropped_no_target` (§6) | **Shipped**, with regression guards |
| `repath_degenerate`, `hold_no_repath` (§6) | **Shipped** |
| Colo rows tagged `dev` (§9.5) | **Fixed** — `CIMMERIA_DEPLOY_ENV: "colo"` in `docker/compose.yml` |
| 60 s verbose capture after `.bug`; Discord deep link | Not yet |
| `playtest.friction` detectors (§9.3b) | **Shipped: all 9.** Episode counters — `repeat_interact_no_effect`, `repeat_item_use_no_chain`, `console_reject_streak` (plus a DEBUG row per rejected command), `escort_separated`. Time-based, re-evaluated every 2 s on movement — `step_stalled`, `region_dwell_no_hint` (server-side point-in-polygon vs client hints), `death_then_silence`. Event-driven, independent of movement — `dialog_displaced` (at dialog display), `objective_never_completed` (at mission force-complete) |
| `session.journal`, `play_session_id`, `service.version` (§9.5–9.6) | Not yet |
| Unified `decision_outcome` (log + span + counter from one helper); `issue_move_order` + `leg_seq` | Not yet |
| Outbound-intent logging (§9.4) | **Partly shipped:** `movement.movement_type` (every `setMovementType` outcome, including the `cleared` case that sends nothing), `wire.out.avatar_update` (1-in-100 record of the position and `yaw_byte` a witness was sent), and the reanchor log now lists `resent` / `not_resent`. Not yet: dialog / mission-log sends, respawner id list |
| `condition_failed` — which condition stopped a chain whose trigger matched (`content.resolve`) | **Shipped** |
| Cover: `cover.detection` edge rows (node, distance, crouched), miss logs on `fire_cover_left` / `fire_cover_duration` / both flank dispatchers, `in_cover` + `cover_sets` + `crouched` on `.bug` | **Shipped** |
| `mission.step_context` — what is already true when a step activates (regions, cover, crouch, combat): the edge-trigger ordering seam | **Shipped** |
| Console reject logging, region-hint containment | **Shipped** with the friction detectors. `unhandled_interaction_flag` is covered by `repeat_interact_no_effect` |

No behaviour was changed in this PR — it is logging only. The defects in §2 are all still present.

Reading a bookmark in SigNoz: `scope_name = 'playtest.bookmark'` lists the notes; take a `bookmark_id` and query
`scope_name = 'playtest.bookmark.entity' AND bookmark_id = <id>`. For "the NPC is facing the wrong way", sort by
`wire_facing_vs_caller_deg` — a chasing NPC near 180 with `yaw_byte = 0` is H1. For "it is floating", read
`y_above_ground` (meshed worlds) or `dy_vs_caller` (meshless worlds such as Castle, where `ground_y` is absent and
`navmesh_loaded = false` on the header says why).

Saved SigNoz views (category `playtest`): **Playtest: bookmarks (.bug notes)**, **Playtest: friction (stuck-player detectors)**, and **Playtest: position trail** — player samples, NPC waypoints/steps, `wire.out.avatar_update` and `setMovementType` rows interleaved with position and `yaw_byte` columns. Narrow the time range to ±30 s around a bookmark to see where everyone was, where they were going and what the client was told.
