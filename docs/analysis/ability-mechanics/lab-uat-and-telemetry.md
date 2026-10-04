# Ability Mechanics: Full Telemetry and Lab UAT Plan

> Type: how-to (campaign plan). Audience: the ability-mechanics coordinator and packet workers.
> Updated: 2026-10-04, against `origin/main` @ `2dad804e2`. Companions: [campaign README](README.md), [work packets](work-packets.md), [automated UAT how-to](../../guides/automated-uat.md), [unified UAT guide § Ability mechanics](../../guides/unified-uat.md#ability-mechanics), [instrumentation discipline](../../architecture/instrumentation-discipline.md), [negative-logging convention](../../architecture/negative-logging-convention.md), [client telemetry](../../architecture/client-telemetry.md), [lab tooling backlog](../lab-automation/tooling-backlog.md).

## Goal

1. **Account for every ability event, on both sides of the wire, for every player, in SigNoz.**
   - This is a shipping requirement, not a lab feature. The server logs every decision a cast passes through.
   - The telemetry DLL that every player runs logs the client's half of the same cast:
     - the key press;
     - whether the client sent the press or dropped it;
     - what it sent;
     - what it received, with the payload decoded;
     - what it applied (stat, effect icon, cooldown sweep, state flag);
     - what it showed (combat text, feedback line, animation), and what it failed to show.
   - One cast is followed through all of it by one `cast_id`. Every ability-related method in the dispatch tables gets a hook on each side, and a test fails the build when one is missing.
2. **Use the game's own debugging channel.** The client has a native combat and ability debug system:
   - `toggleCombatDebug` and `toggleCombatVerboseDebug` (exposed cell methods 2 and 3, **unreachable from the stock client**: it has no event bound to them);
   - `Event_SlashCmd_CombatDebug` and `Event_SlashCmd_AbilityDebug`;
   - the GM methods the client does send, from an `SGWGmPlayer` avatar only: `gmDebugAbility` (169), `gmDebugCombat` (170), `gmDebugCombatVerbose` (171), `gmDebugHeal` (172) and `gmDebugAbilityOnMob` (176);
   - the per-player `debugAbilityList`, `debugEffectList` and `debugAbilityTargetID` properties;
   - there is **no** `onSendCombatDebug` client method (it is a server-internal cell method); the server's trace reaches the game as an `onPlayerCommunication` feedback-channel chat line.

   The server stubs all of it today. This plan wires it up, so a GM in the client can watch the QR rolls and effect decisions of any ability, live, on any target. The same lines go to SigNoz.
3. **Run the campaign's owner UAT in the real client through the lab, graded by evidence.**
   - Every press is a real hotbar key press (N1).
   - Every row is checked against the client's own telemetry (now decoded), the server's state, the packet tap and SigNoz.
   - Setup uses the native GM commands wherever the client has them.

Out of scope: rows for packets that have not merged. AB-05 (waiting on D-AB03), AB-09d (waiting on D-AB13) and AB-11 (waiting on AB-E1) get spec rows marked `blocked`, so the runner reports them BLOCKED instead of dropping them.

## Where things stand

**Merged:** AB-01, AB-02, AB-03, AB-04, AB-06, AB-07, AB-08, AB-09a-c, AB-10 and AB-12 (#1152 to #1163). The ledger's per-packet Status lines still say "Review"; AB-13 closes them out.

**UAT written down:** AB-U1 to AB-U9 in the unified guide (heals, the no-mechanics refusal, Aim, Combat Sprint, Call Target). None has been run. Milestones 3 and 4 have no rows.

**Lab:** the runner and the combat tools are on `main`, but they were tested against a fake bridge only. No live client has run them. There is no `uat-specs/abilities.toml`.

**Server telemetry:** about 80 `abilities` rows and one metric (`abilities_los_refused_total`). Missing:

- **Correlation:** no per-cast id. `effect_seq` exists and is logged nowhere, and a warmed-up cast fires outside the launch span.
- **Targets:** core rows log under module paths, not `abilities`.
- **Silent paths:** about 30, including a dropped short `useAbility`, unknown effect ids and `let _ = tx.send`.
- **Decision rows:** no QR roll row, no effect-timer start or clear row, no `pulse_ended`.
- **Wire sends:** none of the client-bound sends logs.

**Client telemetry:**

- **What it has:** `client.cme.event` names every inbound method the client routed, with no payload. It also has Lua errors, `Debug:log` and sequence drops.
- **What it lacks:**
  - outbound method arguments (backlog C6 is undecoded);
  - inbound payloads (C5);
  - the client's own press gates: the in-flight queue gate `0x00d2b020` and the argument check `0x00aa2910` drop a press without trace;
  - the effect bar, the cooldown sweep and attributed state-flag changes;
  - combat text, outside the lab-only Lua wrapper.

### Gaps that stop a graded run

| # | Gap | Fixed by |
|---|---|---|
| G1 | The client logs names, not payloads, and nothing about its own press gates | AB-C1 to AB-C4 |
| G2 | `server_entity_get` has health and `state_field` only | AB-T5, AB-L1 |
| G3 | No cooldown reset, effect list or forced QR outcome; native GM indices 142, 153, 154 and 169-176 are unimplemented | AB-N1, AB-N2, AB-L2 |
| G4 | No target that holds still and doesn't fight back | AB-L2 (`.dummy`) |
| G5 | Ally rows need a second player | AB-L6 |
| G6 | The colo lab endpoint answered 403 on 2026-09-29; its allowed-hosts setting is now configured | AB-L0 confirms (D-AU8) |
| G7 | No per-cast correlation on the server | AB-T1 to AB-T5 |
| G8 | `docs/commands.md:285` marks `/gmInvokeAbility` implemented; there is no handler | AB-N2 |

## Decisions

PROPOSED rows are adopted at their default unless the owner objects. Record a change as a new row.

| ID | Status | Decision | Reason |
|---|---|---|---|
| D-AU1 | PROPOSED | Graded runs use a **local** server and its `cimmeria-lab-mcp`, as D-AP2 did. A confirmation pass on the colo runs after AB-13's release. | The colo endpoint refuses the lab. |
| D-AU2 | **NEEDS OWNER** | Add `.qr <hit\|miss\|crit\|graze\|off>`: a GM-only override of the caster's QR rolls for 60 s. Every forced roll logs `qr_forced = true` and is echoed to the GM audit channel. | Miss rows (AB-06) need it to be deterministic. It is a new GM power, so the owner decides. The alternative is to assert misses statistically. |
| D-AU3 | PROPOSED | Ally rows use a second lab client (`p2`, account `lab2`). | Only the ally's own client can show that it rendered the heal. |
| D-AU4 | PROPOSED | **The ability client hooks ship in the base telemetry DLL, to every player**, not behind `lab-bridge`. Each is fingerprint-gated and throttled per name. They reach SigNoz through the existing launcher telemetry path, and each event is also pushed to the lab event ring. | The owner rule is that telemetry is first class and a playtest is debuggable from SigNoz alone. A colo tester's bug must be diagnosable without their local files. The lab reads the same events, so UAT and production see one truth. |
| D-AU5 | PROPOSED | Volume budget. Server rows (per cast, per decision, per pulse, per send) are DEBUG. A 1 Hz regen sample is TRACE and off by default. Client rows are throttled per name: a burst of 8, then 4 per second, with a `suppressed` count. A pulse storm thins out but never goes silent. Metrics use enumerated labels only. | Full accounting without drowning the colo collector. |
| D-AU6 | PROPOSED | `.dummy` spawns a lab target with no AI attack tick and no leash, 1,000,000 Health and readable Defense and Accuracy. It despawns after 10 min or when its spawner logs out. | `.spawn` plus `.aggro off` stops a mob noticing you, not hitting back. |
| D-AU8 | **DECIDED** (owner, 2026-10-04) | **Supersedes D-AU1.** There is no local run: every graded run is live on the colo. Each packet reaches the colo through the normal release, since the colo DB is rebuilt from the seed on every deploy. Colo rule 6 applies: touch only the lab characters and what they spawn. The colo lab endpoint has `CIMMERIA_LAB_MCP_ALLOWED_HOSTS` set as of 2026-10-04, so server clauses should verify; AB-L0 confirms it. | Owner's call. |
| D-AU7 | PROPOSED | Use the native GM and debug commands wherever the client has an index or a slash command. Dot commands are only for things the client never had: `.effects`, `.cooldowns`, `.dummy` and `.qr`. | The project rule for GM commands (#518, #523). |

## The correlation model

One cast, one `cast_id`, on every row on both sides:

```text
client                                   wire                       server
key press (hotbar slot, ability)                                    
client gates pass / drop (reason)                                   
useAbility sent ──── mercury packet seq ─────────────────────────► useAbility recv (packet seq)
                                                                    launch: cast_id = effect_seq
                                                                    gates, warmup, fire, QR, effects,
                                                                    ledger, pulses
onEffectResults / onSequence recv ◄── effect id = cast_id ──────── wire sends (cast_id)
onTimerUpdate recv (id, secondary)  ◄─────────────────────────────
applied: stat, effect bar, cooldown, state flag
shown: combat text, feedback line, sequence played or dropped
```

- The server mints `cast_id` from `effect_seq` (`handle.rs:527`). It is already the effect id the client receives in `onEffectResults`, so client receive rows carry it with no wire change.
- **Press to cast:** the press joins its cast through the Mercury packet sequence number. The client logs the outbound packet seq that carried `useAbility`; the server logs the inbound packet seq on the receipt row, then mints `cast_id`.
- **Timers to cast:** they join through `(player_id, ability_id)` for cooldowns and `(player_id, effect_id, secondary_id)` for effect timers. The server's send row carries `cast_id` too.
- **Fallback join:** `(player_id, ability_id)` within 2 s, for presses the client dropped before sending.

## Part 1: server telemetry (`AB-T`)

Rules: [instrumentation-discipline.md](../../architecture/instrumentation-discipline.md) and the [negative-logging convention](../../architecture/negative-logging-convention.md). That means a stable `event =`, a dotted target, `account_id` and `player_id` on every player row, numbers as numbers, no `let _ = tx.send`, and a `LogCapture` guard per row.

**Core fields on every row:**

- `cast_id`, `ability_id`, `ability_name`, `effect_id` (per-effect rows);
- `caster_id`, `caster_kind`, `account_id`, `player_id`;
- `wire_target_id`, `target_id`, `target_player_id`;
- `stage`: `recv`, `gate`, `launch`, `warmup`, `fire`, `qr`, `route`, `apply`, `pulse`, `ledger`, `wire` or `end`.

**Targets** (all pass `OTEL_FILTER`'s `abilities=debug` prefix row):

| Target | Covers |
|---|---|
| `abilities` | Receipt, launch, gates, refusals and fire. Rows that log under a module path move here. |
| `abilities.qr` | Rolls and cover |
| `abilities.effect` | Per-effect plan, dispatch, NVP damage, routing |
| `abilities.ledger` | Timed and held entries, absorb pools, state flags |
| `abilities.pulse` | Pulsing effects |
| `abilities.wire` | Every client-bound send |
| `abilities.debug` | The native combat-debug lines (AB-N1) |
| `vitals` | Regen, plus the TRACE sample |

Each packet is one PR through the lane and `ship.sh`. `rust-gameserver-dev` writes the code, `combat-systems-advisor` reviews AB-T1 and AB-T3, and `testing-validation-engineer` reviews every packet's guards.

### AB-T1. Cast correlation

- **Carry `cast_id`** on:
  - the `combat.use_ability` span;
  - `PendingCast` and the warmup state;
  - `ActiveEffect`, `TimedEffect` and `InterruptRequest`;
  - the absorb pools.
- **New spans:** `combat.cast_fire` (warmup fire) and `combat.effect_tick` (per pulse), each opened with the `cast_id`.
- **Receipt row:** the `useAbility` receipt logs the inbound Mercury packet seq.
- **Rule 5:** add `account_id` and `player_id` to every player row in `use_ability/`.
- **Guards:** a warmup cast's fire and ledger rows and a DoT's tick rows carry the launch row's `cast_id`.

### AB-T2. Silent paths and failed sends

Each early return logs one DEBUG row naming the system, what was expected, what happened and what the player saw.

- **Receipt and launch:**
  - a short `useAbility` (`cell-methods/.../combat/mod.rs:86`);
  - a dead caster (`handle.rs:170`);
  - `get_entity_mut` returning `None` (`:431`);
  - `launch_target` returning `None` (`:103`).
- **Warmup:** `resolve_warmups`' `continue`s (`warmup/tick.rs:56/62/65`).
- **Fire:** `fire_at_target` with no target, or with every effect routed away.
- **Effects:**
  - unknown effect ids (`effect_scripts.rs:87/158/241`, `damage_apply/mod.rs`);
  - the unknown-ability 15 HP fallback (WARN);
  - `NvpPlanner::add`'s zero return;
  - a dispatch with no script name;
  - the damage scripts' twelve silent returns. Demote their per-hit INFO rows to DEBUG.
- **Pulses and ledger:**
  - `register_active_effect` returning false, and its discarded bool;
  - `remove_timed_effects` matching nothing;
  - the `let _` on `remove_lock` and on the shield `on_remove`;
  - the shield ledger refusal (`shield/mod.rs:137`).
- **Regen:** the skips at `regen.rs:93/100/106`, at TRACE.
- **Failed sends:** `messaging.rs:49/75/145/189` replace `let _ = tx.send` with a WARN.

**Guards:** one `LogCapture` test per site family.

### AB-T3. Decision rows

- **QR:** `qr_rolled` with `qr`, `roll`, `result_code`, `cover`, `dont_use_qr` and `forced`.
- **Effect plan:** `effect_planned`, one row per effect per target.
  - Path: `script`, `nvp`, `routed_to_user`, `ally_fanout`, `ledger`, `pulse` or `skipped`.
  - Reason: `miss`, `damage_script`, `no_script`, `not_reachable`, `dont_use_qr` or `group_aura_pending_d_ab12`.
- **NVP damage:** `nvp_damage_resolved` gains `result_code`, `damage_type`, the pools before and after, and `absorbed`.
- **Shield row:** `pipeline.rs:165`'s shield row gets the core fields.
- **Pulses:** a `pulse_ticked` row per pulse, and `pulse_ended` on a natural end (today nothing logs it).
- **Guards:**
  - a forced miss logs `qr_rolled` with `miss` and `effect_planned` with `reason = miss`;
  - Pistol Shot logs exactly one damage path.

### AB-T4. Wire-send ledger

One `abilities.wire` row per send, logged after the send:

| Message | Fields |
|---|---|
| `onEffectResults` | `result_code`; the results (`stat_id`, `delta`); the effect id sent |
| `onStatUpdate` | recipient, stat ids and values, witness count |
| `onTimerUpdate` | `timer_type`, `id`, `secondary_id`, `complete_at`, `start` or `clear`. Cooldowns, effect durations and category timers: none of them logs today. |
| `onErrorCode` | `code`, `reason` |
| `onStateFieldUpdate` | flags set and cleared, refcounts after |
| `onSequence`, `Ability_Interrupt` | `sequence_id`, `source_id`, `target_id`, `reason` |
| `onSendCombatDebug` | line count only (the text is on `abilities.debug`) |

**Guards:** a `LogCapture` assert per send family.

### AB-T5. State snapshot

`ability_state(entity)` returns:

- the pending cast;
- cooldowns with time remaining;
- pulsing effects (effect, invoker, `cast_id`, pulses left);
- ledger entries (deltas, expiry or held, monikers, flags);
- absorb pools;
- `state_field` refcounts per bit;
- every stat, current and max.

It feeds `.effects`, the lab's `server_ability_state` tool, and an `abilities.snapshot` INFO row. That row is written on every `.bug` bookmark (so each UAT anchor carries the state at its start), on death and on logout.

### AB-T6. Metrics

**Counters**, with enumerated labels and `world`:

- `abilities_cast_total{outcome}`;
- `abilities_refused_total{reason}`;
- `abilities_effect_applied_total{path}`;
- `abilities_ledger_removed_total{reason}`;
- `abilities_qr_total{result}`;
- `abilities_wire_send_failed_total{message}`.

**Histograms:**

- `abilities_press_to_fire_ms`;
- `abilities_damage_dealt{pool}`;
- `abilities_heal_done{pool}`.

The client-derived histograms are AB-C6's.

## Part 2: client telemetry, shipped to every player (`AB-C`)

These hooks live in `cimmeria-client-telemetry`'s base build (D-AU4). Each one:

- is anchored in the QA `SGW.exe` and added to the fingerprint gate, so a different build installs nothing;
- is throttled per name;
- is emitted as a `client.ability.*` event to the uploader and to the lab ring.

`game-archaeology-specialist` resolves each anchor first, through headless Ghidra, with a finding under `docs/reverse-engineering/findings/`. `rust-gameserver-dev` writes the hook. Each hook ships with a primitive test (patch, call, unpatch) like the existing inline hooks.

| Packet | Seam | Event and fields | Anchor and RE state |
|---|---|---|---|
| AB-C1 | **Outbound methods with arguments.** Backlog C6, scoped first to the ability methods: `useAbility` (68), `useAbilityOnGroundTarget` (69), `petInvokeAbility` (88), `petAbilityToggle` (89), `confirmationResponse` (4), `resetMyAbilities` (72, sent as `Event_NetOut_RespecAbility`) and `trainAbility` (77). `toggleCombatDebug` (2, 3) is not hookable: the client never sends it. | `client.ability.sent` `{method, ability_id, target_id, ground_xyz, mercury_seq, client_target_id}`. `client_target_id` is what the UI had targeted, to compare with what was sent (B-15). | **Resolved by AB-C0.** `RouteOutgoingEntityRpc` `0x00c6fc40` (`stdcall(entity, desc, method, args)`, `ret 0x10`), reached only from the shared callback `0x00d43dc0`; `args` is a name-keyed event bag read with `GetInt` `0x00e3cba0`. The packet seq is **not** available at `Channel::send` (it is assigned on the network thread at `0x0158bb40` inside `Nub::send` `0x01582160`); join by a seq range. [Finding](../../reverse-engineering/findings/ability-client-hook-anchors.md) |
| AB-C2 | **Client press gates.** The hotbar press, and every client-side reason not to send. | `client.ability.press` `{slot, ability_id, key}`, then either `client.ability.sent` or `client.ability.press_dropped` `{ability_id, reason}`. Reasons are only those the client really has: `no_action`, `bad_args`, `not_known`, `pet_missing`, `not_connected`, `class_mismatch`. The client checks no cooldown, range, dead or target state, so those are not reasons. | **Resolved by AB-C0.** `0x00d2b020` is not a gate (`addAbilityIfAbsent`); `0x00aa2910` is the `useAbility` thunk, which the hotbar does not use (it goes `useAction` `0x00aa94e0`). The press has no cooldown, range, dead or target check on the client; it drops silently at `0x00ad959e` (empty slot), `0x00d2afcf` (ability not in the client's `AbilitySet`) and the `Route` branches `0x00c6fc68` to `0x00c6fd41`. Reasons to report: `no_action`, `bad_args`, `not_known`, `pet_missing`, `not_connected`, `class_mismatch`. [Finding](../../reverse-engineering/findings/ability-client-hook-anchors.md#ab-c2-the-press-path-and-its-drop-branches) |
| AB-C3 | **Inbound payloads** for the ability allowlist (backlog C5): `onEffectResults`, `onTimerUpdate`, `onErrorCode`, `onStatUpdate` and its variants, `onStateFieldUpdate`, `onSequence` (including `Ability_Interrupt`, an `onSequence` with event id 1002, not a method), and the ability-list updates. `onSendCombatDebug` is not a client method and has no hook. | `client.ability.recv` `{method, entity_id, decoded args}`. `onEffectResults` carries the effect id, which is the `cast_id`. | **Resolved by AB-C0.** Two seams: the wire bytes at `onEntityMethod` `0x00dd2b80` (non-consuming read of the `MemoryIStream`, decoded with the existing wire decoders; the only way to get the array arguments), and the event at the central dispatch `0x00a372f0` with the game's own `GetInt` / `GetFloat` / `GetByte` for scalars. `Ability_Interrupt` is an `onSequence` with event id 1002, not a method; `onSendCombatDebug` is not a client method. |
| AB-C4 | **What the client applied.** Effect bar add, refresh and remove (effect id, remaining, total). Cooldown applied to a hotbar button. A stat property applied to the local player or the current target. The state-flag dispatcher, now attributed to an entity and a bit. | `client.ability.applied` `{kind: effect_bar\|cooldown\|stat\|state_flag, ...}` | **Resolved by AB-C0.** Effect bar: `EffectSet` timer handler `0x00e09160` (type 5). Cooldown: `CooldownManager` handler `0x00ea6af0` and its callback caller `0x00ea62b0`. Stats: per-stat functors `0x00e004e0` (current) and `0x00e005b0` (base). State flag: `0x00e01c90` with the old value at `[this+0x158]` and the entity id at `[this+0xc]`. Removal: `FUN_00e0a810` posts a generic event (`CategoryId` 9, `Key` = `SecondaryId`) and is reached only from `FUN_00e0a9e0`, whose single caller is the `0x00e09160` new-entry path; there is no separate native expiry call, so the bar's expiry is clock-based (INFERRED), and a hook reports add, refresh and this removal event. |
| AB-C5 | **What the client showed, or failed to show.** Floating combat text, natively, not the lab Lua wrapper. The feedback chat line from `onErrorCode`. The sequence played or dropped (C15 extended to the `onSequence` handler's drop branch, with the `sequence_id` tied to its cast). The animation notify for the cast. Lua errors from `ActionButtons.lua`, `Effect.lua` and `SCTMod` tagged as ability UI. | `client.ability.shown` `{kind, ...}`, `client.sequence.dropped` with `cast_id` where known | **Partly resolved by AB-C0.** Combat text and chat lines are Lua (`SCTMod.createEventWindow`, `ChatMod.onMessageReceived`); there is no native add function, so hook the Lua functions or the `Event_UI_*` dispatch. The `onSequence` drop branch is `0x00d0585c` (no client entity for `SourceID`). The `onErrorCode` handler is `0x00cf33e0`; the UI event field layouts and the `writeLocalFeedback` link are still open. |
| AB-C6 | **Client timing.** Press-to-send, send-to-first-response, and response-to-applied, all measured on the client clock. | Fields on `client.ability.recv` and `.applied`; histograms on the uploader | none (derived) |
| AB-C7 | **Coverage gate.** `tools/telemetry-coverage/abilities.py` reads the ability sections of the four `docs/protocol/*-dispatch-table.md` files and writes a matrix: every ability-related method × {server recv row, server send row, client send hook, client recv hook}. A test in `cimmeria-client-telemetry` and one in `cimmeria-cell-combat` fail when a method in the table has no hook or row on a side where it travels. The matrix is committed as `docs/analysis/ability-mechanics/telemetry-coverage.md`. | none | none |

**Shipping:** AB-C1 to AB-C5 go out in the next launcher release after they merge. The release notes name the new hooks. The colo's public telemetry ingest (port 8081) already accepts `client.*` events, so the server side needs no change.

## Part 3: native in-game debugging (`AB-N`)

### AB-N0. RE: the native debug surface

- **Questions:**
  - the client-method index of `onSendCombatDebug`, and where it renders (chat channel or window);
  - the slash-command names behind `Event_SlashCmd_CombatDebug`, `_AbilityDebug` and `_DebugAbilityOnMob` (the `SGWTextCommandMgr` runtime map);
  - the arguments each `Event_NetOut_*` handler sends (`docs/reverse-engineering/decompiled/01_sgw_game_classes.c:6906-6984`);
  - whether the client sends `toggleCombatDebug` for non-GM accounts.
- **Output:** a finding, plus the rows added to the client-method dispatch table.
- **Owner:** `game-archaeology-specialist`.
- **Result (2026-10-04).** [native-combat-debug.md](../../reverse-engineering/findings/native-combat-debug.md). `onSendCombatDebug` and `onSendEventDebug` are **server-internal cell methods, not client methods**: the client has no handler or renderer, so AB-N1 cannot send them. `toggleCombatDebug` and `toggleCombatVerboseDebug` have no client event. The client sends four `gmDebug*` methods through `Event_NetOut_AbilityDebug`, `_CombatDebug`, `_CombatDebugVerbose` and `_DebugAbilityOnMob`, only from an `SGWGmPlayer` avatar, and the slash keywords are `/gmdebugability`, `/gmdebugcombat`, `/gmdebugcombatverbose`, `/gmdebugabilityonmob`. Debug text can only reach a player as an `onPlayerCommunication` chat line on the feedback channel.

### AB-N1. Combat and ability debug, as designed

> **AB-N0 correction.** The design below sends `onSendCombatDebug(simple, verbose)` to the client. That method is server-internal and the client cannot receive it. Deliver the simple line as `onPlayerCommunication` on `CHAN_FEEDBACK`, keep the verbose detail on `abilities.debug`, and key the toggles on `gmDebugCombat` (170), `gmDebugCombatVerbose` (171), `gmDebugAbility` (169) and `gmDebugAbilityOnMob` (176), not on cell 2 and 3.

- **Toggles:** `gmDebugCombat` (170) and `gmDebugCombatVerbose` (171), sent by the client from a GM avatar, flip `bCombatDebug` and `bCombatVerboseDebug`. Cells 2 and 3 stay for crafted callers (GM-gated in `gm_gate.rs`) but the stock client cannot send them.
- **Ability debug:**
  - `gmDebugAbility` (169) and `toggleAbilityDebugging` maintain `debugAbilityList`;
  - `gmDebugAbilityOnMob` (176) makes a mob's casts debuggable;
  - `setAbilityDebugTarget` and `clearAbilityDebug` set or clear `debugAbilityTargetID`.
- **Heal:** `gmDebugHeal` (172); its client event `Event_NetOut_HealDebug` is verified (bind `0x00dc56cc`, thunk `0x00d71da0`, ctor `0x00d61cb0`); the keyword `/gmdebugheal` is name-derived like the others.
- **What gets sent:**
  - When a cast involves a debugged player, a debugged ability, or a mob with debug on, the server sends the simple line to the debug target as an `onPlayerCommunication` (client method 28) chat line on `CHAN_FEEDBACK`.
  - The simple line holds the cast, the target resolution, the QR roll and the result.
  - The verbose line adds every `effect_planned`, apply and ledger decision from AB-T3.
- **Shared text:** one formatter builds the line from the same data the AB-T rows log, and the line also goes to `abilities.debug`. The in-game trace and SigNoz therefore never disagree.
- **Guards:** a byte-exact test for the `onPlayerCommunication` debug line, and pipeline tests that a debugged cast sends the line and an undebugged one doesn't.

### AB-N2. Native GM commands for ability testing

Implement the native indices the UAT needs:

| Index | Command | Notes |
|---|---|---|
| 136 | `gmGiveAbility` | No-debit variant |
| 142 | `gmSetGodMode` | The damage-seam gate |
| 153 | `gmResetAbilities` | |
| 154 | `gmGiveAllAbilities` | The archetype's tree |
| 158 | `gmSetMobAbilitySet` | |

Then correct `docs/commands.md`: `/gmInvokeAbility` is not implemented (G8). Follow the #518 pattern (native index, `SGWGmPlayer` class flip). `server-authority-enforcer` reviews.

## Part 4: lab tools and the UAT run (`AB-L`, `AB-R`)

A UAT step presses the real hotbar key (N1). Everything below is setup, readback or evidence.

| Packet | What | Tier |
|---|---|---|
| AB-L0 | Rebuild and install `cimmeria-lab` and the telemetry DLL from `main`, point it at the colo, confirm `server_sessions` answers over WireGuard (no 403), and run `lab_uat_run` with `plan_only`. Record the SHAs here. No code unless the smoke fails. | none |
| AB-L1 | `server_ability_state` (`LabQuery::AbilityState`) over AB-T5; `focus` and `stats` added to `server_entity_get`. | read |
| AB-L2 | Dot commands, GM-gated and audited: `.effects [target]`, `.cooldowns reset [id]` (it sends the clear timer, so the sweep stops), `.dummy [hostile\|friendly]` (D-AU6), `.cleareffects [target]` (teardown), and `.qr` if D-AU2 is approved. | G |
| AB-L3 | Runner support: `${cast_id}` captured from the press (the `client.ability.sent` packet seq joined with the server's receipt row), a `client_event` clause source that reads the decoded `client.ability.*` events from the ring, a `server` clause over `server_ability_state`, and `@cooldowns_reset` and `@ability_state` capabilities. | runner |
| AB-L4 | `packet` clauses: tap from the anchor to teardown, and assert a message's fields. They cross-check AB-C3: the tap and the client's own decode must agree. | runner |
| AB-L6 | Two-client rows: the runner drives `p2` (`lab2`), with a `@target_player` capability. | runner |

AB-L5 (lab-only Lua readers) from the first draft is dropped: AB-C3 to AB-C5 give the same facts natively, to every player.

### AB-R0. `docs/guides/uat-specs/abilities.toml`

- **Section:** `ability-mechanics`, GM account, fresh character.
- **Setup:** `/gmGiveAllAbilities` (or `.giveability`), hotbar placement (`place: true`, N3 in setup only), `.dummy`, and `/combatdebug` on (AB-N1), so every row's in-game debug lines land in the chat log as evidence.
- **Every row has:**
  - a `.bug uat <row>` anchor;
  - an N1 press;
  - a `client_event` clause (sent, recv, applied, shown);
  - a client UI clause (stat, effect bar, hotbar sweep, combat text);
  - a `server` clause;
  - a `packet` clause;
  - a SigNoz clause on `cast_id`;
  - a teardown.
- **Human clauses:** only for art and animation.

AB-U1 to AB-U9 keep their guide ids. AB-U10 onwards are new and go into the unified guide in the same PR.

| Row | Packet | Press | Evidence beyond the base set |
|---|---|---|---|
| AB-U1 | AB-01 | Heal Focus (597): no target, self, dummy, `p2` | `client.ability.sent` target versus `client_target_id` (B-15); own Focus rises; dummy and `p2` never; `resolution` |
| AB-U2 | AB-01 | Health Heal (1646) on `p2` | `p2`'s own client applies the Health stat |
| AB-U3 | AB-01 | Health Heal on self, dummy, nothing | `fallback_to_caster` |
| AB-U4 | AB-01/02 | Recuperation (1218) on `p2` | 25 `pulse_ticked` rows with one `cast_id`, then `pulse_ended`; `p2`'s client applies each |
| AB-U5 | AB-01 | Heal Focus at the dummy, out of combat | No `BSF_InCombat` on either side; empty threat table |
| AB-U6 | AB-12 | A no-mechanics ability, twice | `client.ability.shown` feedback line twice; no cooldown applied on the client or the wire |
| AB-U7 | AB-04 | Aim (637), again at 5 s | Accuracy +200; effect bar shows 15 s, then refreshes; `replaced`; clear at expiry, on both sides |
| AB-U8 | AB-04/07 | Combat Sprint (1619) | Run speed +50 % and Accuracy -100, 10 s |
| AB-U9 | AB-04 | Call Target (847) on the dummy, then die with Aim up | Dummy Defense -100 via `server_ability_state`; `died` removal; client effect bar clears |
| AB-U10 | AB-06 | Pistol Shot with `.qr miss`, then `.qr hit` | No pool moves on the miss; exactly one damage path on the hit. BLOCKED without D-AU2. |
| AB-U11 | AB-03 | Each archetype's tree root at the dummy | Damage equals the seeded NVP (`server_db_query`); combat text shows it |
| AB-U12 | AB-03 | A DoT root | Per-pulse damage × pulses; `pulse_ended` |
| AB-U13 | AB-07 | Morale Boost, `p2` in and out of range | `ally_fanout` only in range |
| AB-U14 | AB-08 | A toggle on, off, on | Held entry comes and goes; stats restore exactly |
| AB-U15 | AB-08 | Stance A, then B | A `RemovedByMoniker`; only B held |
| AB-U16 | AB-08 | A passive (1450, 1731 or 1574); relog | Applied at login, no icon; a press is refused |
| AB-U17 | AB-09a | Takedown on the dummy | `BSF_MovementLock` set, then cleared with refcount 0, on both sides |
| AB-U18 | AB-09a | Stun a live mob mid-warmup | Warmup interrupted; no movement or fire for the duration |
| AB-U19 | AB-09b | Snare Shot (1462) on a mob | `movementSpeedMod` -30; slower chase in server positions |
| AB-U20 | AB-09c | Interrupting Shot during the dummy's warmup | `Ability_Interrupt` sent and received |
| AB-U21 | AB-10 | Personal Shield (4306), then take damage | Absorb drains before Focus; `Drained`; a second press while full is refused with feedback |
| AB-U22 | AB-10 | A cleanse (2827) after a debuff | Exactly the named count removed, `Cleansed` |
| AB-U23 | AB-N1 | `/gmdebugcombat` from a GM character, then Pistol Shot | The feedback-channel debug line in chat matches the `abilities.debug` row for the same `cast_id` |
| AB-U24 | AB-05 | Regen | `blocked = "D-AB03"` |
| AB-U25 | AB-11 | Floating heal number on `p2` | `blocked = "AB-E1, AB-11"` |

Before the spec merges, the coordinator confirms each row's ids against the seed and the packet's live-DB tests.

### AB-R1. AB-E1 by telemetry

AB-C1 and AB-C2 answer B-15 (does a 597 press leave the client, and with which target) directly. A hand-built `onEffectResults` on the lab server, read through AB-C3, the packet tap and a screenshot, answers B-62. Output: `docs/reverse-engineering/findings/beneficial-cast-client-evidence.md`. That unblocks AB-11.

### AB-R2. Full graded run, colo

`lab_uat_run` in batches of about 5 rows on one `run_dir`. Attest the SigNoz clauses with the SigNoz MCP, then run `lab_uat_report { ledger: true }`. A FAIL becomes a fix packet `AB-F<n>`, with its `cast_id` forensics query as the evidence.

### AB-R3. Owner confirmation

After AB-13's `/release` and the launcher release carrying AB-C1 to AB-C5:

- a full re-run of the section on the release build;
- the owner UAT;
- a SigNoz read of the owner's own session, joined client to server on `cast_id`.

## Order and dependencies

```text
AB-N0 RE ──────────────► AB-N1 combat debug ─┐
AB-C0 RE anchors ─┬────► AB-C1 ─► AB-C2 ─────┤
                  ├────► AB-C3 ─► AB-C4 ─────┤
                  └────► AB-C5 ──────────────┤
AB-T1 cast_id ─┬─► AB-T2, AB-T3, AB-T4 ──────┼─► AB-T6, AB-C6, AB-C7 coverage gate ─► SigNoz views and docs (AB-T7)
               └─► AB-T5 ─► AB-L1, AB-L2 ────┤
AB-N2 native GM ─────────────────────────────┤
AB-L0 smoke, AB-L4, AB-L6 ───────────────────┴─► AB-L3 ─► AB-R0 ─► AB-R1 ─► AB-R2 ─► AB-13 + launcher release ─► AB-R3
```

- **First wave, in parallel worktrees:**
  - AB-L0;
  - AB-T1;
  - AB-N0 and the AB-C anchor RE (AB-C0: one `game-archaeology-specialist` pass covering the C1 to C5 anchors; headless Ghidra runs one at a time);
  - AB-N2, AB-L4 and AB-L6.
- **Second wave:** AB-T2 to AB-T5, AB-C1, AB-C3, AB-C5, AB-N1 and AB-L1/L2.
- **Third wave:** AB-C2, AB-C4, AB-C6, AB-C7, AB-T6, AB-T7 and AB-L3.
- **Live work:** AB-L0 and AB-R1 to AB-R3 take the lab lock, one at a time.
- **`use_ability/handle.rs` (612 lines):** AB-T1 lands first. AB-T2 moves its logging into the guard modules rather than growing the file.
- **`messaging.rs`:** AB-T2 lands before AB-T4.

## Acceptance

- **Full accounting:** for any AB-R2 row, one SigNoz query on its `cast_id` (with the press joined on `mercury_seq`) returns, in order:
  1. the client press;
  2. the client gate result;
  3. the send;
  4. the server receipt;
  5. every gate, launch, warmup and fire;
  6. the QR roll;
  7. every effect plan, apply, pulse and ledger row;
  8. every wire send;
  9. the client receipt of each send, decoded;
  10. what the client applied and showed;
  11. the end.

  No gap in the chain is explained by "it returned early" without a row saying so.
- **Coverage:** the AB-C7 matrix has no empty cell for a method that travels in that direction, and its tests pass in CI.
- **Native debug:** a GM who types the combat-debug slash command sees, in game, the same decisions SigNoz holds for that cast.
- **UAT:** every non-blocked row is PASS, or NEEDS_HUMAN with the human clause answered.
- **Docs:**
  - this ledger;
  - the unified guide (rows AB-U10 to AB-U25 and the SigNoz recipe);
  - `observability-target-catalog.md` and `client-telemetry.md`;
  - the dispatch tables (AB-N0);
  - `docs/commands.md`;
  - `automated-uat.md` (the new clause sources) and `live-research-lab.md`.

## Ledger

| Packet | Status | PR | Notes |
|---|---|---|---|
| AB-T1 | Review | #1168 | `cast_id` via a cast scope on `SpaceManager`; spans `combat.cast_fire`, `combat.effect_tick`. Not reached: the receipt row's packet seq (needs a `packet_seq` on `BaseToCellMsg::CellMethodCall`, threaded from `receive_in_order`'s per-bundle seq through `dispatch_cell_method` and the cell's method dispatch: AB-T2), and zero-warmup ground-cast AoE secondaries (they resolve after the launch returns, outside the scope) |
| AB-T3 | Review | #1173 | `qr_rolled`, `effect_planned` (hit plan + landings + support shot), `nvp_damage_resolved` pools/`result_code`/`absorbed`, the shield row with ids, `pulse_ticked` / `pulse_ended` on `abilities.pulse` (renamed from AB-T1's `effect_pulse_fired` / `active_effect_ended`). `forced` is always false: the D-AU2 hook is `qr_gate::forced_roll`. Not covered: a strip (death, duel end, cleanse, channel cancel) logs its existing row, not `pulse_ended`; per-effect rows carry no `ability_name` (join on `cast_id` to `ability_launched`) |
| AB-T5 | Merged 2026-10-04 | #1172 | `CellEntity::ability_state` (`cimmeria-entity`, serde) and the `abilities.snapshot` INFO row (`cimmeria-cell-world` `effects::ability_snapshot`) on `.bug` (tester and selected target, on the `bookmark_id`), a player's death (before the clear-on-death strip) and logout. Deaths and logouts of NPCs write no row by design. Moniker cooldowns and owed timer clears are in the snapshot too |
| AB-T2 | Review | #1176 | Every listed silent return logs one row (launch gates in `use_ability/gate_rows.rs`, the hit plan in `damage_apply/silent_rows.rs`, the scripts in `effects/script_rows.rs`); damage-script per-hit rows demoted to DEBUG; `scripts.rs` tests moved to `scripts_tests.rs` (file cap). Receipt seq landed: `RxDelivery::bundle_seqs` (a fragmented bundle takes its first fragment's seq) through `dispatch_client_bundle` / `dispatch_cell_method` and `BaseToCellMsg::CellMethodCall::packet_seq` to the `use_ability_recv` row's `mercury_seq`. Deviations: `warmup/tick.rs`'s not-yet-due `continue` logs at TRACE (`warmup_pending`, it fires every 100 ms per warming cast), the moved-off-anchor and fire-time `continue`s already log `warmup_interrupted`; a `remove_timed_effects` sweep that matches nothing is TRACE. Not in scope: the `messaging.rs` `let _ = tx.send` WARNs (AB-T4), `support_shot.rs`'s discarded `register_active_effect` bool (the refusal now logs inside `register_active_effect`) |
| AB-T4 | Ready | | |
| AB-T6, AB-T7 | BlockedDependency | | |
| AB-C0 (anchor RE) | Done (static; live checks listed in the finding) | | [ability-client-hook-anchors.md](../../reverse-engineering/findings/ability-client-hook-anchors.md): VERIFIED seams for C1, C2, C3, C4 (effect bar, cooldown, stat, state flag) and the `onSequence` drop; UNRESOLVED: bag arrays, UI event layouts, effect removal body, `onErrorCode` tail |
| AB-C1 | InReview (anchors static, live status UNVERIFIED) | #1179 | `client.ability.sent` from the existing router hook: the allowlist (the seven methods plus `gmDebug*` 169-172, 176) decoded with `GetInt` / `GetFloat` / `GetByte` using the descriptor's own argument-name strings; `client_target_id` with `client_target_inferred = true`; sent only when a `start*Message` ran (`0x00dd6a60`, `0x00dd6980` hooked; `0x00dd8010` is a wrapper). Mercury seq as a follow-up `client.ability.sent_seq` joined by `send_id`: `Channel::send` tags the bundle before the hand-off, `Nub::send` claims it, the counter `0x0158bb40` supplies one seq per packet; 28-bit modular range. Joins the server's `use_ability_recv` `mercury_seq` (AB-T2, #1176) by modular range membership. [client-telemetry.md](../../architecture/client-telemetry.md#ability-presses-and-sends-clientability) |
| AB-C2 | InReview (anchors static, live status UNVERIFIED) | #1179 | `client.ability.press` (`press_id`, `source` = `hotbar` \| `lua`, `slot`, `ability_id`, `target_id`; no `key` field: native code sees the action id, not the key) then one `press_dropped` or a `sent` carrying the `press_id`. Seven hooks on the chain (`0x00aa94e0`, `0x00aa2910`, `0x00ad9580`, `0x00d2afc0`, `0x00d2ae40`, `0x00e3cf40`, `0x00d3a820`). Reasons from the finding plus three GamePet send branches found while implementing (`not_known` for the pet's set, `pet_state_flag`, `pet_ability_flag`; added to the finding). The Ability window and a script both call `useAbility`, so both are `source = lua` |
| AB-C3 to AB-C5 | Ready (AB-C0 done; effect removal resolved to `0x00e0a810`, see the AB-C4 row) | | |
| AB-C6, AB-C7 | BlockedDependency | | |
| AB-N0 | Done | | [native-combat-debug.md](../../reverse-engineering/findings/native-combat-debug.md): premise corrected, AB-N1 design must change |
| AB-N1 | BlockedDependency (AB-T3); redesign from the AB-N0 result | | |
| AB-N2 | Review | #1170 | 136, 142, 153, 154, 158 native handlers; G8 fixed in `docs/commands.md`. God mode restores Health/Focus at the hit and pulse seams (decision 36); reset is to the archetype's `char_creation_abilities` starters with the spend refunded |
| AB-L0 | Ready | | |
| AB-L4 | Merged 2026-10-04 | #1167 | `source = "packet"` clauses and the `approx` op (`value` ± `tolerance`); one tap per row from the anchor to teardown, stopped on every path; rows kept as the `packet_tap` attachment; UNVERIFIED when the endpoint is unreachable. Guide: automated-uat.md "Packet clauses". |
| AB-L6 | InReview | #1169 | `players = 2` rows drive the second lab instance (`lab-account.p2.json`, default `p2`) through an in-process supervisor; `client = "p2"` on actions, clauses and evidence; `@target_player` = real-input `client_target` on the other player's character; still BLOCKED, with the reason, when no p2 is configured. gm-parity M1-2 uses it. Guides: automated-uat.md "Two-player rows", live-research-lab.md "Two clients". |
| AB-L1 | Merged 2026-10-04 | #1174 | `LabQuery::AbilityState` + `server_ability_state` (cimmeria-lab-mcp) over AB-T5's snapshot; `server_entity_get` gains `focus_cur`/`focus_max` and every stat (`#[serde(default)]`, so an older cell still reads); `server_entity_query` gets focus but no full stat block (256 snapshots × ~80 stats). |
| AB-L2 | Review (`.qr` BlockedDecision, D-AU2) | | `.effects`, `.cooldowns [reset [id]]`, `.dummy [hostile\|friendly\|clear] [templateId]`, `.cleareffects` in `cell-console` `console/abilities/`. The dummy is a template NPC (default 34) with a `LabDummy` extension that `ai_driven_npc_entity_ids` skips, so it gets no AI turn at all; 1,000,000 Health; despawned by a 1 Hz sweep after 10 min and on its owner's `DisconnectEntity`; at most 4 per GM; `.dummy clear` removes the caller's only. The cooldown clear is the warmup interrupt's zero `onTimerUpdate` (type 2), always sent by the one-ability form. `.cleareffects` = `remove_timed_effects(Cleansed)` + pulses off with `on_remove` + `flush_stat_buff_timers` + `onStatUpdate`. Each change logs one `abilities.gm` INFO row beside the dispatcher's audit row. The disposition sets the faction (10 hostile, 9 Friendly_Ambient), since the player attack gate reads the faction, not the override; every despawn first drains the dummy from players' `threatened_mobs` and broadcasts the `BSF_InCombat` clears (Copilot review). Known gap: a GM who changes world keeps their dummies until expiry (only logout sweeps them). |
| AB-L3 | Review | (this PR) | `source = "client_event"` clauses over the lab event store (`event` = the telemetry target, `match_fields`, `since`, `min_rows`/`max_rows`, `field` `op` `value`), read through `client_wait_event` with an explicit `since_seq` from a runner mark (never `client_events_read`, whose cursor is the operator's); a store `gap` or a throttle `suppressed` count makes an upper-bound PASS UNVERIFIED. `server` clauses on `@ability_state` default to the lab character's entity, and pointers take `[key=value]` selectors (`/state/stats[stat_id=22]/cur`). `${cast_id}` per press, in order: the client `client.ability.recv` `onEffectResults` for the ability (`client_recv`); `client.ability.sent` → `.sent_seq` range → the server `use_ability_recv` `mercury_seq` → that entity's next `ability_launched`, from `server_log_tail` (`seq_join`); the `(entity, ability)` launch within 2 s of the press on the anchor-corrected server clock (`press_window`). `@dummy` (stores `${dummy_id}`), `@cooldowns_reset` and `@clear_effects { name }` type the AB-L2 lines at G and wait for their feedback line; a refusal or silence fails the action. Guide: automated-uat.md "Client event clauses and `${cast_id}`" |
| AB-R0 | Ready (draft rows; clauses fill in as tools land) | | |
| AB-R1 to AB-R3 | BlockedDependency | | |
