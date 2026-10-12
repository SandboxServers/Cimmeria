# LH expansion C: issue #1660 items 8, 10, 11 (packets LH-09 .. LH-12)

Worktree: `<wt>`. Crate package: `cimmeria-lab` (`crates/lab`). All paths below are relative to the worktree. docs/**/*.md are CRLF.

Lane prefix used below (`<wt>` = the packet's own worktree):

```
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo fmt -p cimmeria-lab
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo clippy -p cimmeria-lab --all-targets -- -D warnings
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo nextest run -p cimmeria-lab <filter>
```

## Summary of findings

| Item | Root cause | Sure? |
|---|---|---|
| 10 stats empty | Not decidable statically. `stats_of` (`player_state.rs:23-33`) needs three things at once: `pairs(Stat)` yields numeric ids, `getUnitStat` returns a Lua **table**, and that table has `.id`. The RE doc says the binding sets `id`, `current`, `max`, `min`, `baseCurrent`, `baseMax`, `baseMin` (`14_standalone_named.c:207030-207042`), so the likeliest break is (a) `Stat` or the returned value not being a plain table, or (b) `type(id)=='number'` failing. The tool also hides the failure: `compact.rs` drops empty maps, so "empty stats" never says why. `Stat.Intellect` does not exist because the server and the stat docs name it `intelligence` (`docs/gameplay/stat-system.md:70`), so the client name is almost certainly `Stat.Intelligence` (not proven: the client enum was never dumped). | Cause: 40%. Name: 75%. A probe settles both. |
| 8 move_to | Three separate defects. (1) `ENTITY_ARRIVAL_M = 2.5` (`movement.rs:22`) is tighter than the interact range the briefs use (3.5-4.5 m, item 9), so a vendor already in range is walked at. (2) `MoveController::step` (`steer.rs:202`) has no divergence guard: it only detects *no progress* (`stuck_window_ms`), never *negative* progress, so a wrong learned turn sign or a wall deflection walks the avatar away until the 60 s timeout. (3) The Stuck failure (`movement.rs:293-304`) carries no path, start distance or gain, so nobody can tell snag from drift. The straight-line walk has no path-finding, so the hub's walls cause the 14.3 m "stuck". | (1) and (3) certain. (2) certain as a gap; that it is *the* cause of the 15-35 m drift: 60%. Probe P5 decides. |
| 11 chat miss | `client_chat_log` is the only thing that installs the chat wrapper (`events/mod.rs:183-201` runs the install chunk only inside a reader pump). After a relog the world UI Lua is rebuilt and the stock `ChatMod.onMessageReceived` replaces our wrapper (or the whole `_G.CimmeriaLab` goes, new `epoch`). The wrapper is back only at the next reader call, which in the failing sequence (`.bankexpand` then `client_chat_log`) is *after* the reply arrived. | 70%. Probe P2 decides. |
| 11 "click install failed, bridge_dropped in the thousands" | Two unrelated numbers in one report. `bridge_dropped` is the DLL **events ring** (`client-telemetry/src/bridge/events.rs:40`, 4096 slots, drops the newest, `DROPPED` reset on each drain). Nobody drains it between tool calls, so any quiet period with chatty `cme.event`/`cegui.log` rows overflows it. It does not touch the Lua rings (`CimmeriaLab.*`, in-client, 512 each). The click `failed` is `subscribeGlobal` raising inside `pcall` (`lua_rings.rs:238-239`): the first-ever live run of LR-10 (LR-13 never ran; `LR-10.md:80`), and the error text is thrown away. | Drops: 95%. Click: cause unknown; probe P4. |

## Live probes (the coordinator runs these before the coders start; all through a scripted lab-driver, `client_lua_eval` / `client_events_read`)

**P1 (item 10, stats shape and names).** In the world, one `client_lua_eval`:

```lua
local t = {} ; local n = 0
local okp = pcall(function() for k, v in pairs(Stat) do n = n + 1 ; t[#t + 1] = tostring(k) .. "=" .. tostring(v) end end)
table.sort(t)
local s = getUnitStat(Unit.Player, Stat.Health)
return type(Stat) .. "|pairs_ok=" .. tostring(okp) .. "|n=" .. n .. "|ret=" .. type(s) .. "|id=" .. tostring(s and s.id) .. "|cur=" .. tostring(s and s.current) .. "|intel=" .. tostring(Stat.Intelligence) .. "|intellect=" .. tostring(Stat.Intellect) .. "|" .. table.concat(t, ",")
```

Outcomes: `table|pairs_ok=true|n>0|ret=table|id=nil` means `.id` is absent, so the old guard was the bug. `ret=userdata` means the type check was the bug. `n=0` or `pairs_ok=false` means `Stat` is not enumerable, so the fallback name list is needed. The tail is the real stat-name list: **paste it into the LH-09 doc paragraph and the `FALLBACK` list below** (replace my guesses with the real names; keep PascalCase exactly as printed). `intel=<number>` confirms `Stat.Intelligence`.

**P2 (item 11, relog).** (a) Before any relog, run a `client_chat_log` once so the wrapper exists. (b) `lab_logout`, then `lab_ensure_in_world`. (c) *Before* any chat read, `client_lua_eval`:
`local L = rawget(_G, "CimmeriaLab") ; return tostring(L and L.epoch) .. "|wrapped=" .. tostring(L ~= nil and ChatMod.onMessageReceived == L.chat_wrapper)`.
Outcomes: `nil|wrapped=false` means the Lua state was rebuilt (the whole table is gone). `<epoch>|wrapped=false` means ChatMod was redefined, table kept. `wrapped=true` means the cause is elsewhere (then also run `/say hi` and a `client_chat_log` and compare with `client_ui_state` chat tail). Either of the first two confirms the "armed too late" cause.

**P3 (item 11, drop rate).** In the world, idle in Castle, two `client_events_read {max:1}` calls 10 s apart, no other tool calls between. Read `head` (store seq) and `dropped` from the second. `dropped > 0` means the ring overflowed in 10 s (more than about 400 events/s). Record the `head` delta per second: it is the real event rate, and decides whether the keeper below is enough. If the rate is above 800/s, add to the owner list: filter `cegui.log` in the keeper drain.

**P4 (item 11, click failure).** One `client_lua_eval` (restart the client afterwards if it returns `true|`: a good subscribe cannot be undone):

```lua
local r = {}
local okw, w = pcall(getWindow, "Root") ; r[#r+1] = "getWindow=" .. tostring(okw) .. "/" .. type(w)
if okw and w then
  local ok2, e2 = pcall(function() w:subscribeGlobal("Window", "MouseClick", "CimmeriaLab.noop") end)
  r[#r+1] = "subscribe=" .. tostring(ok2) .. "/" .. tostring(e2)
end
r[#r+1] = "ns=" .. tostring(CEGUI and CEGUI.Window and CEGUI.Window.EventNamespace) .. " ev=" .. tostring(CEGUI and CEGUI.Window and CEGUI.Window.EventMouseClick)
return table.concat(r, "|")
```

Outcomes decide LH-12: an error text naming the namespace/event means the constants are wrong (use the literals, drop the `CEGUI.Window.*` override); `getWindow=false` means `Root` is not a window name here (use `BackgroundWin` first); a `subscribe=true` means the failure was transient (client was not in the world at the first pump: LH-11's keeper retries every second and the problem disappears; LH-12 then only improves the message).

**P5 (item 8, drift).** Character in Castle_CellBlock hub, 10-15 m from a vendor with a straight clear line, then another with a wall between. For each: `client_move_to {name:"<vendor>", timeout_ms:20000}` with `verbose:true`; keep `path`, `turn_gain` (`sign_flips` > 0 means a mis-learned sign), and the server `server_entity_get` position of the avatar before and after. Outcomes: path moves monotonically away and `sign_flips >= 1` means a turn-model problem (LH-10 as specified aborts it; a follow-up would pin the sign from the first standing turn); path hugs the wall and stalls means plain geometry (waypoints are the answer, documented in LH-10).

## Packets

Independence: LH-09 (player_state.rs), LH-10 (movement.rs, steer.rs, tool_tests.rs), LH-11 (events/, watchdog.rs, chat_log.rs), LH-12 (lua_rings.rs) touch disjoint files. LH-11 and LH-12 both change docs/guides/live-research-lab.md in different paragraphs: merge LH-12 after LH-11 to avoid a doc rebase, or run them in either order and rebase the guide hunk.

---

### LH-09: client_player_state reads stats robustly and says why when it cannot (S)

depends-on: P1 (names only; the code is correct either way).

**Files**
- `crates/lab/src/supervisor/ui/player_state.rs` (chunk, `shape`, tests)
- `docs/guides/live-research-lab.md` (line 916 row)
- `.claude/agents/lab-driver.md` (no stat text there; skip unless the driver file lists stat names)

**Steps**

1. In `chunk()` replace everything from `local function stats_of(u)` (line 23) through its closing `end` (line 33) with the block below. Inside `format!` every brace is doubled, as in the rest of the string.

```lua
local function stat_field(s, k)
  local ok, v = pcall(function() return s[k] end)
  if ok then return v end
  return nil
end
local FALLBACK = {{ 'Health', 'Focus', 'Energy', 'Coordination', 'Engagement', 'Fortitude', 'Intelligence', 'Morale', 'Perception', 'Accuracy', 'Defense', 'QrMod', 'Damage', 'Penetration', 'Mitigation', 'Awareness', 'InCombat' }}
local function stats_of(u)
  local out, listed, read = {{}}, 0, 0
  local named = {{}}
  pcall(function()
    for name, id in pairs(Stat) do named[#named + 1] = {{ name, id }} end
  end)
  if #named == 0 then
    for _, name in ipairs(FALLBACK) do
      local ok, id = pcall(function() return Stat[name] end)
      if ok and id ~= nil then named[#named + 1] = {{ name, id }} end
    end
  end
  for _, p in ipairs(named) do
    local id = tonumber(p[2])
    if id then
      listed = listed + 1
      local s = __jcall(getUnitStat, u, id)
      if (type(s) == 'table' or type(s) == 'userdata') and stat_field(s, 'current') ~= nil then
        read = read + 1
        out[tostring(p[1])] = {{ id = id, current = stat_field(s, 'current'), max = stat_field(s, 'max'), min = stat_field(s, 'min'), base_current = stat_field(s, 'baseCurrent') }}
      end
    end
  end
  return out, {{ stat_type = type(Stat), listed = listed, read = read }}
end
```

(After P1 replace the names in `FALLBACK` with the printed list. A stat the unit does not have returns an empty table, so `current ~= nil` is the presence test; `id` is no longer required.)

2. Change line 59 `if {include_stats} then me.stats = stats_of(P) end` to:
`if {include_stats} then me.stats, me.stats_diag = stats_of(P) end`

3. Ammo block (line 68-71): replace
`if Stat and Stat.AmmoSlot1 then` ... `if type(s) == 'table' then ammo.current = s.current ammo.max = s.max end` with
```lua
    if Stat and Stat.AmmoSlot1 then
      local s = __jcall(getUnitStat, P, Stat.AmmoSlot1 + slot - 1)
      if s ~= nil and stat_field(s, 'current') ~= nil then ammo.current = stat_field(s, 'current') ammo.max = stat_field(s, 'max') end
    end
```

4. Target health (lines 89-92): replace the `if Stat then ... end` block with
```lua
  if Stat then
    local h = __jcall(getUnitStat, T, Stat.Health)
    if h ~= nil and stat_field(h, 'current') ~= nil then target.health = {{ current = stat_field(h, 'current'), max = stat_field(h, 'max') }} end
  end
```

5. In `shape()` (line 102) add, directly before `out["effect_names"] = json!(names);` (line 122):

```rust
    // An empty stat map used to look like "no stats"; say what the read saw.
    let empty = raw["stats"].as_object().is_none_or(|m| m.is_empty());
    if empty && !raw["stats_diag"].is_null() {
        let d = &raw["stats_diag"];
        out["stats_note"] = json!(format!(
            "no stat was readable: the client's Stat table is a {}, {} ids were listed and {} read. Read one directly with client_lua_eval: getUnitStat(Unit.Player, Stat.Health).current.",
            d["stat_type"].as_str().unwrap_or("nil"),
            d["listed"].as_u64().unwrap_or(0),
            d["read"].as_u64().unwrap_or(0),
        ));
    }
```

**Test** (add to the `tests` module of `player_state.rs`):

```rust
    /// An empty stat read must explain itself instead of looking like "no
    /// stats": the compact result drops an empty map, so the note is all the
    /// caller sees.
    #[test]
    fn shape_explains_an_empty_stat_read() {
        let raw = json!({
            "name": "Labone", "stats": {},
            "stats_diag": { "stat_type": "userdata", "listed": 0, "read": 0 }
        });
        let s = shape(&raw);
        assert!(s.get("health").is_none());
        let note = s["stats_note"].as_str().unwrap();
        assert!(note.contains("userdata") && note.contains("0 read"), "{note}");
        // A read that found stats adds no note.
        let ok = shape(&json!({
            "stats": { "Health": { "id": 1, "current": 5, "max": 9 } },
            "stats_diag": { "stat_type": "table", "listed": 1, "read": 1 }
        }));
        assert!(ok.get("stats_note").is_none());
        assert_eq!(ok["health"]["current"], 5);
    }

    /// The reader must not require an `id` field or a plain Lua table (the
    /// old guard `type(s) == 'table' and s.id`), and must fall back to named
    /// stats when `Stat` cannot be enumerated.
    #[test]
    fn chunk_reads_stats_without_requiring_id_or_a_plain_table() {
        let c = chunk(true, false);
        assert!(!c.contains("and s.id then"), "the id guard is back");
        assert!(c.contains("stat_field(s, 'current') ~= nil"));
        assert!(c.contains("type(s) == 'userdata'"));
        assert!(c.contains("local FALLBACK"));
        assert!(c.contains("me.stats, me.stats_diag = stats_of(P)"));
    }
```

(The first test fails without step 5; the second fails if the old guard returns. The chunk has no Lua runtime in the workspace, which is why the Lua is asserted textually, as `lua_rings.rs` does.)

**Lane checks**: the three commands above with filter `player_state`.

**Docs**: `docs/guides/live-research-lab.md`, row at line 916 (`| \`client_player_state\` | Position, facing ... the active weapon's ammo, the target. | — |`): append to the second column: ` Stats are read by the names in the client's \`Stat\` table (printed in P1; \`Stat.Intelligence\`, not \`Intellect\`); when none can be read the result has \`stats_note\` saying what the table looked like. A stat the unit lacks is left out.` Also in the line 936 UAT item 8 keep as is.

**Live smoke**: `client_player_state {"lease_id":"...","fields":["stats","stats_note","health"]}` in the world: `stats` non-empty and `stats.Health.current` equals `getUnitStat(Unit.Player, Stat.Health).current` from `client_lua_eval`; `stats.Intelligence` present (value matches the character sheet). Server evidence: `server_entity_get` for the player, attribute stats, same numbers.

**Commit**: `fix(lab): client_player_state reads stats without the id guard and explains an empty read (LH-09)`

---

### LH-10: client_move_to stops at interact range, aborts when it walks away, and fails with evidence (M)

depends-on: none (P5 informs the follow-up only).

**Files**
- `crates/lab/src/supervisor/world/steer.rs` (`MoveConfig`, `MoveOutcome`, `MoveController`, tests)
- `crates/lab/src/supervisor/world/movement.rs` (const, leg report, failure)
- `crates/lab/src/supervisor/world/tool_tests.rs` (existing 2.5 test, new tests)
- `docs/guides/live-research-lab.md` (lines 304 and 699, plus one new paragraph)

**Steps**

1. `steer.rs`, `MoveConfig` (line 90-105): add a field after `deadband_rad`:

```rust
    /// Give up when walking has taken the avatar this far (metres) beyond the
    /// closest it has been to the goal: it is walking away, not snagged.
    pub diverge_m: f64,
```
and in `Default` (line 107-118) add `diverge_m: 6.0,` after `deadband_rad: 0.05,`.

2. `steer.rs`, `MoveOutcome` (line 157-161): add a variant

```rust
    /// Farther from the goal than ever after walking: wrong turn sign, a
    /// wall deflecting the pawn, or a goal that is running away.
    Diverged { distance_m: f64, best_m: f64 },
```

3. `steer.rs`, `MoveController` struct (line 178-187): add `min_m: f64,` after `best_at: u64,`; in `new` add `min_m: f64::INFINITY,` after `best_at: 0,`.

4. `steer.rs`, in `step`, directly after the arrival block (after line 231, the `}` that closes `if dist <= self.cfg.arrival_m {`) and before the comment `// Progress is only expected while walking`, insert:

```rust
        // Never closer than this: remembered across turns and unsticks.
        self.min_m = self.min_m.min(dist);
        if was_forward && dist > self.min_m + self.cfg.diverge_m {
            out.outcome = Some(MoveOutcome::Diverged {
                distance_m: dist,
                best_m: self.min_m,
            });
            return out;
        }
```

5. `movement.rs` line 21-22: replace the const and its doc with

```rust
/// Default arrival radius for an entity target (stop next to it, not in
/// it). Inside the 3.5 to 4.5 m a right-click or a trainer window needs, so a
/// target already in range is a no-op and a walk ends in range (#1660).
pub const ENTITY_ARRIVAL_M: f64 = 4.0;

/// What a failed walk tells the caller to try. The walk is a straight line
/// with no path-finding.
const FAIL_HINT: &str = "client_move_to walks a straight line and cannot path-find. Go round the obstacle with waypoints, hold a movement key with a ui_sequence key step (hold_ms), or teleport on the server and click without walking; then stand 3.5 to 4.5 m from the target.";
```

6. `movement.rs`, in `walk`, before `for (i, (mut goal, arrival)) in legs.into_iter().enumerate() {` (line 182) add `let mut leg_in_range: Vec<bool> = Vec::new();`. Inside the loop, after `let mut unsticks = Vec::new();` (line 191) add `let mut leg_start_m: Option<f64> = None;`. After `let s = ctl.step(now, pose, goal_pos);` (line 234) add `if leg_start_m.is_none() { leg_start_m = Some(s.distance_m); }`.

7. `movement.rs`, in the `report` json (line 280-290): add `"start_distance_m": leg_start_m,` after `"arrival_m": arrival,` and add a `Diverged` arm to the `outcome` match:

```rust
                MoveOutcome::Diverged { distance_m, best_m } => json!({ "arrived": false, "diverged": true, "distance_m": distance_m, "best_m": best_m }),
```
Right after `leg_reports.push(report);` add `leg_in_range.push(leg_start_m.is_some_and(|d| d <= arrival));`.

8. `movement.rs`, replace the whole `if let MoveOutcome::Stuck { distance_m, attempts } = outcome { ... }` block (lines 293-304) with:

```rust
        let evidence = || {
            json!({
                "at": prev.to_json(),
                "goal": goal.describe(),
                "start_distance_m": leg_start_m,
                "unsticks": unsticks,
                "turn_gain": turn.to_json(),
                "path": path,
                "hint": FAIL_HINT,
            })
        };
        match outcome {
            MoveOutcome::Arrived { .. } => {}
            MoveOutcome::Stuck {
                distance_m,
                attempts,
            } => {
                return Err(steps.fail_with(
                    io.now_ms(),
                    &format!("leg_{i}"),
                    format!("stuck {distance_m:.1} m short after {attempts} unstick attempts"),
                    evidence(),
                ));
            }
            MoveOutcome::Diverged { distance_m, best_m } => {
                return Err(steps.fail_with(
                    io.now_ms(),
                    &format!("leg_{i}"),
                    format!(
                        "walking away from the goal: {distance_m:.1} m now, {best_m:.1} m at the closest; stopped instead of drifting further"
                    ),
                    evidence(),
                ));
            }
        }
```
(`unsticks` is a `Vec<&str>` declared in the leg; the closure borrows it. If the borrow checker objects to the closure borrowing `path`, inline the `json!` in both arms.)

9. `movement.rs`, in the final `out` json (line 312-319) add `"already_in_range": leg_in_range.iter().all(|b| *b),` after `"arrived": true,`.

10. `tool_tests.rs` line 415: change `assert!(d <= 2.5 + 0.1, "stopped {d} m away");` to `assert!(d <= 4.0 + 0.1, "stopped {d} m away");`.

**Tests**

In `steer.rs` tests (module at the end of the file), add:

```rust
    /// Moving away is not a snag: the controller must give up with
    /// `Diverged` long before the stuck window or the walk's timeout.
    #[test]
    fn move_controller_gives_up_when_it_walks_away() {
        let mut c = MoveController::new(MoveConfig::default(), TurnModel::new(400.0));
        let target = Vec3::new(2000.0, 0.0, 0.0); // 20 m ahead
        let mut outcome = None;
        // The pawn walks backwards 1 m a step (100 uu).
        for k in 0..30u64 {
            let s = c.step(k * 100, pose(-100.0 * k as f64, 0.0, 0.0), target);
            if s.outcome.is_some() {
                outcome = s.outcome;
                break;
            }
        }
        match outcome {
            Some(MoveOutcome::Diverged { distance_m, best_m }) => {
                assert!(distance_m > best_m + 6.0);
                assert!((best_m - 20.0).abs() < 0.1);
            }
            other => panic!("expected Diverged, got {other:?}"),
        }
    }

    /// A target that is simply approached never diverges.
    #[test]
    fn move_controller_does_not_diverge_while_closing() {
        let mut c = MoveController::new(MoveConfig::default(), TurnModel::new(400.0));
        let target = Vec3::new(2000.0, 0.0, 0.0);
        for k in 0..15u64 {
            let s = c.step(k * 100, pose(100.0 * k as f64, 0.0, 0.0), target);
            assert!(!matches!(s.outcome, Some(MoveOutcome::Diverged { .. })));
        }
    }
```
Why the first fails when reverted: without the guard the controller keeps walking (forward) until the 2.5 s stuck window, so the loop ends with `Stuck` or `None`, not `Diverged` (the variant would not even compile).

In `tool_tests.rs` add (needs the sim to walk away: put the target behind and make the sim ignore turns so W carries the pawn the wrong way; the sim has `gain` and `standing_turn` knobs but no "wrong way" knob, so use a goal behind with a wall that is actually behind the pawn is not possible; therefore test the no-op and the evidence only):

```rust
    /// A vendor already in interact range is a no-op: no key is pressed, and
    /// the result says so.
    #[tokio::test]
    async fn move_to_an_entity_already_in_range_presses_nothing() {
        let mut sim = Sim::new().with_entity(10, "Quartermaster", Vec3::new(380.0, 0.0, 0.0));
        let out = movement::run(
            &mut sim,
            MoveRequest {
                name: Some("quartermaster".into()),
                ..MoveRequest::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(out["arrived"], true);
        assert_eq!(out["already_in_range"], true);
        assert!(sim.looks.is_empty(), "turned toward a target in range");
        assert!((sim.pos.x - 0.0).abs() < 1.0, "walked {} uu", sim.pos.x);
    }

    /// A failed walk carries the evidence needed to tell a snag from drift.
    #[tokio::test]
    async fn move_to_failure_carries_path_gain_and_hint() {
        let mut sim = Sim::new();
        sim.wall_x = Some(1000.0);
        sim.jump_clears_wall = false;
        let e = movement::run(
            &mut sim,
            MoveRequest {
                point: Some(client_point(2500.0, 0.0, 0.0)),
                ..MoveRequest::default()
            },
        )
        .await
        .unwrap_err();
        let st = e.to_json()["state"].clone();
        assert!(st["start_distance_m"].as_f64().unwrap() > 20.0);
        assert!(st["path"].is_array());
        assert!(st["turn_gain"]["counts_per_rad"].is_number());
        assert!(st["hint"].as_str().unwrap().contains("waypoints"));
    }
```
(The first fails if `ENTITY_ARRIVAL_M` goes back to 2.5: the avatar is 3.8 m away and would walk 1.3 m. The second fails without step 8. Check `sim.looks` is the pub field `looks: Vec<(i32,i32,i32)>`: yes, `sim.rs:76`.)

**Lane checks**: the three commands with filter `move_to` then `move_controller`.

**Docs**
- `docs/guides/live-research-lab.md` line 699 row: replace `` `arrival_m` (1.5, or 2.5 for an entity) `` with `` `arrival_m` (1.5, or 4.0 for an entity: inside the 3.5 to 4.5 m a click or trainer needs) ``, and append to the result column: `` ; `already_in_range` when no leg had to move. A walk that gets farther from the goal than it has been by 6 m stops with a \`diverged\` failure; every failure carries \`path\`, \`turn_gain\`, \`start_distance_m\` and a \`hint\` ``.
- After the table paragraph that contains that row add this paragraph (new): `` `client_move_to` walks a straight line with `W` and mouse-look. It cannot path-find, so a wall or counter on the way (the Castle_CellBlock hub is full of them) ends it as `stuck`. Go round with `waypoints`, hold a key (`{"do":"key","key":"s","hold_ms":2500}` in `client_ui_sequence`), or teleport on the server (`gotoxyz`, `server_console_exec`) and click without walking. Check `already_in_range` first: a target within 4 m needs no walk. ``
- `.claude/agents/lab-driver.md`: no change.

**Live smoke**: character in the Castle_CellBlock hub, vendor within 4 m: `client_move_to {name:"<vendor>"}` returns `arrived:true, already_in_range:true` and `server_entity_get` shows the avatar position unchanged (to 0.1 m). Then a vendor 10 m away with a clear line: arrives at 3.5-4.0 m. Then one behind a wall: fails `stuck` or `diverged` within about 12 s with `path` in the result and the avatar not farther than 6 m from where it was closest.

**Commit**: `fix(lab): client_move_to stops at interact range, aborts when walking away, fails with evidence (LH-10)`

---

### LH-11: keep the event capture armed and drained between tool calls, and report drops honestly (M)

depends-on: P2 and P3 (they confirm the cause; the packet is safe either way).

**Design.** A per-launch background keeper (like the watchdog, same 1 s cadence, same pid gate) does two cheap things: drains the DLL events ring into the store (so it never overflows into `bridge_dropped` between calls) and runs the ring install chunk with `max = 0`, which (re)installs the chat/combat/click wrappers but returns no records. The Lua rings keep their own records (not drained: they are mark-read) until a reader pump takes them, so a chat line cannot be evicted from the store by an event flood while it waits. The keeper calls the bridge directly, not through `bridge_call`, so it does not fill the crash journal or need a lease. The drop count is no longer lost: every drain adds to `dropped_unreported`, and the next reader pump reports and clears it, plus a running total.

**Files**
- `crates/lab/src/supervisor/events/store.rs` (field + method)
- `crates/lab/src/supervisor/events/mod.rs` (`mod keeper;`, `PumpReport`, `ingest_bridge`, `pump_events`)
- `crates/lab/src/supervisor/events/keeper.rs` (new, about 110 lines with tests)
- `crates/lab/src/supervisor/watchdog.rs` (spawn)
- `crates/lab/src/supervisor/ui/chat_log.rs` (armed-now warning)
- `docs/guides/live-research-lab.md`, `docs/guides/automated-uat.md`

**Steps**

1. `store.rs`, struct `StoreInner` (line 77-88): after `pub upstream_dropped: u64,` add

```rust
    /// Of `upstream_dropped`, the part no reader pump has reported yet. The
    /// keeper drains between tool calls and sees drops no caller was there
    /// to read; the next reader pump takes them with [`Self::take_unreported_drops`].
    pub dropped_unreported: u64,
```
and after `with_cap` (line 97) add

```rust
    /// Drops the keeper or an earlier drain saw that no reader has been told
    /// about; resets to zero.
    pub fn take_unreported_drops(&mut self) -> u64 {
        std::mem::take(&mut self.dropped_unreported)
    }
```

2. `events/mod.rs`: add `mod keeper;` after `pub mod lua_rings;` (line 21; keep `fake_bridge` cfg(test) as is, the keeper tests use it). In `ingest_bridge` (line 80-81) replace
`    st.upstream_dropped += dropped;` with
```rust
    st.upstream_dropped += dropped;
    st.dropped_unreported += dropped;
```

3. `events/mod.rs`, `PumpReport` (line 44-58): after `pub bridge_dropped: u64,` add
```rust
    /// Every event the bridge ring has dropped since the lab started
    /// (`bridge_dropped` is only what no earlier reader was told).
    pub bridge_dropped_total: u64,
    /// Plain-words reading of `bridge_dropped`, when it is not zero.
    pub bridge_note: Option<String>,
```
In `to_json` after `"bridge_dropped": self.bridge_dropped,` add `"bridge_dropped_total": self.bridge_dropped_total,` and `"bridge_note": self.bridge_note,`.

4. `events/mod.rs`, `pump_events` (line 177-182): replace the block
```rust
        {
            let mut st = self.events.inner.lock().await;
            let (n, d) = ingest_bridge(&mut st, &drained);
            rep.bridge_events = n;
            rep.bridge_dropped = d;
        }
```
with
```rust
        {
            let mut st = self.events.inner.lock().await;
            let (n, _) = ingest_bridge(&mut st, &drained);
            rep.bridge_events = n;
            rep.bridge_dropped = st.take_unreported_drops();
            rep.bridge_dropped_total = st.upstream_dropped;
            if rep.bridge_dropped > 0 {
                rep.bridge_note = Some(format!(
                    "the client's event ring overflowed and dropped {} newest event(s) since the last report: cme.event, net.out, cegui.log and similar rows may be missing. chat.line, combat.hit and ui.click come from separate in-client rings and are not affected.",
                    rep.bridge_dropped
                ));
            }
        }
```

5. New file `crates/lab/src/supervisor/events/keeper.rs`:

```rust
//! The event keeper: between tool calls, drain the client's event ring and
//! keep the lab's Lua wrappers installed.
//!
//! Two losses came from nobody being there (#1660). The DLL's event ring
//! (4096, drops the newest) overflowed whenever no tool pumped for a while,
//! and the chat wrapper was only installed by a reader, so the first chat
//! line after a relog arrived before the wrapper did. The keeper ticks once a
//! second for as long as its launch lives. It reads the bridge directly (not
//! through `bridge_call`): it needs no lease and must not fill the crash
//! journal. The Lua install chunk runs with `max = 0`: it installs, and reads
//! no record, so the in-client rings keep their records for a reader pump and
//! a chat line cannot be evicted from the store while it waits.
//! `CIMMERIA_LAB_EVENT_KEEPER=0` turns it off.

use std::time::Duration;

use serde_json::json;
use tracing::Instrument;

use super::{ingest_bridge, lua_rings, BRIDGE_DRAIN_MAX};
use crate::supervisor::Supervisor;

/// Gap between keeper ticks.
pub const KEEPER_TICK: Duration = Duration::from_secs(1);
/// A tick that takes longer than this is abandoned (a loading client).
const TICK_TIMEOUT: Duration = Duration::from_secs(3);
/// Environment switch; `0`, `off` or `false` disables the keeper.
pub const KEEPER_ENV: &str = "CIMMERIA_LAB_EVENT_KEEPER";

/// Whether the keeper runs, given the environment value.
pub fn enabled_from(v: Option<&str>) -> bool {
    !matches!(
        v.map(|s| s.trim().to_ascii_lowercase()).as_deref(),
        Some("0" | "off" | "false")
    )
}

impl Supervisor {
    /// One keeper tick: drain the bridge ring into the store and make sure
    /// the Lua wrappers are installed, reading no ring record.
    pub async fn keeper_tick(&self) -> Result<(), String> {
        let drained = self
            .bridge
            .call("events_read", json!({ "max": BRIDGE_DRAIN_MAX }))
            .await
            .map_err(|e| format!("bridge: {e}"))?;
        {
            let mut st = self.events.inner.lock().await;
            ingest_bridge(&mut st, &drained);
        }
        // max = 0: install the wrappers, return no records.
        let chunk = lua_rings::pump_chunk(0, 0, 0, 0);
        let v = self
            .bridge
            .call("lua_eval", json!({ "chunk": chunk }))
            .await
            .map_err(|e| format!("bridge: {e}"))?;
        let _ = v;
        Ok(())
    }

    /// Spawn the keeper for `pid`; it ends when the launch's pid changes.
    pub(in crate::supervisor) fn spawn_keeper(&self, pid: u32) {
        if !enabled_from(std::env::var(KEEPER_ENV).ok().as_deref()) {
            return;
        }
        let this = self.clone();
        let span = tracing::info_span!("lab_event_keeper", instance = self.label(), pid);
        tokio::spawn(
            async move {
                loop {
                    tokio::time::sleep(KEEPER_TICK).await;
                    if this.state.lock().await.pid != Some(pid) {
                        return;
                    }
                    // A failed or slow tick is ignored: the next one retries.
                    let _ = tokio::time::timeout(TICK_TIMEOUT, this.keeper_tick()).await;
                }
            }
            .instrument(span),
        );
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::supervisor::events::fake_bridge::{self, empty_rings, is_ring_pump, Responder};

    /// A fake whose events_read reports these drop counts in order (then 0),
    /// with one event per read, and records every ring chunk it is sent.
    fn fake(drops: Vec<u64>) -> (Responder, Arc<Mutex<Vec<String>>>) {
        let drops = Arc::new(Mutex::new(VecDeque::from(drops)));
        let chunks = Arc::new(Mutex::new(Vec::new()));
        let c2 = chunks.clone();
        let r: Responder = Arc::new(move |method, params| match method {
            "events_read" => {
                let d = drops.lock().unwrap().pop_front().unwrap_or(0);
                Ok(json!({ "returned": 1, "dropped": d, "events": [
                    { "kind": "cme.event", "ts_ms": 1, "fields": { "event": "e" } } ] }))
            }
            "lua_eval" if is_ring_pump(params) => {
                c2.lock()
                    .unwrap()
                    .push(params["chunk"].as_str().unwrap_or_default().to_string());
                Ok(empty_rings())
            }
            other => Err(format!("unexpected {other}")),
        });
        (r, chunks)
    }

    #[tokio::test]
    async fn a_tick_drains_the_bridge_and_installs_without_reading_records() {
        let (r, chunks) = fake(vec![0]);
        let sup = fake_bridge::supervisor(r).await;
        sup.keeper_tick().await.unwrap();
        assert_eq!(sup.event_head().await, 1, "the drained event is in the store");
        let c = chunks.lock().unwrap();
        assert_eq!(c.len(), 1);
        // max = 0: the chunk installs the wrappers and returns no record.
        assert!(c[0].contains("e.seq > 0 and n < 0"), "{}", c[0]);
        assert!(c[0].contains("e.seq > 0 and m < 0"), "{}", c[0]);
    }

    /// Drops the keeper saw are told to the next reader, once, and counted.
    #[tokio::test]
    async fn drops_a_tick_saw_reach_the_next_reader_once() {
        let (r, _) = fake(vec![5, 0, 0]);
        let sup = fake_bridge::supervisor(r).await;
        sup.keeper_tick().await.unwrap();
        let first = sup.pump_events(false).await.unwrap();
        assert_eq!(first.bridge_dropped, 5);
        assert_eq!(first.bridge_dropped_total, 5);
        assert!(first.to_json()["bridge_note"].as_str().unwrap().contains('5'));
        let second = sup.pump_events(false).await.unwrap();
        assert_eq!(second.bridge_dropped, 0);
        assert_eq!(second.bridge_dropped_total, 5);
        assert!(second.to_json()["bridge_note"].is_null());
    }

    #[test]
    fn the_env_switch_reads_off_values() {
        assert!(enabled_from(None));
        assert!(enabled_from(Some("1")));
        assert!(!enabled_from(Some("0")));
        assert!(!enabled_from(Some(" Off ")));
        assert!(!enabled_from(Some("false")));
    }
}
```
Notes for the coder: `self.bridge` and `self.events` are private fields of `Supervisor`; a descendant module of `supervisor` may use them, as `events/mod.rs` already does. `Supervisor` is `Clone` (the watchdog clones it). If `tracing::Instrument` is not a dependency path in this crate it is: `watchdog.rs` imports `tracing::Instrument`. If clippy flags `let _ = v;` drop the binding: write `.map(|_| ())` after the `map_err` instead.

6. `watchdog.rs`, `spawn_watchdog` (line 19-24): after the `tokio::spawn(...)` line add `self.spawn_keeper(pid);`.

7. `chat_log.rs`: add, above `impl Supervisor` (line 194):

```rust
/// A caller-facing warning when the chat capture was not already running, so
/// lines from before this call may be missing from the log.
pub fn capture_warning(install: &[(String, String)]) -> Option<&'static str> {
    match install
        .iter()
        .find(|(r, _)| r == "chat")
        .map(|(_, s)| s.as_str())
    {
        Some("ok") => None,
        Some(s) if s.starts_with("installed") => Some(
            "the chat capture was installed during this call; lines shown before now are not in this log. Read client_ui_state's chat tail for them.",
        ),
        Some("missing") => Some(
            "the chat window is not loaded (not in the world yet): nothing is being captured.",
        ),
        _ => Some("the chat capture could not be read (see capture.lua_error)."),
    }
}
```
and in `chat_log()` after `out["capture"] = rep.to_json();` (line 232) add
```rust
        if let Some(w) = capture_warning(&rep.lua_install) {
            out["capture_warning"] = json!(w);
        }
```

**Tests** (in `chat_log.rs` `tests` module):

```rust
    #[test]
    fn capture_warning_names_a_late_or_absent_capture() {
        let s = |status: &str| vec![("chat".to_string(), status.to_string())];
        assert!(capture_warning(&s("ok")).is_none());
        assert!(capture_warning(&s("installed")).unwrap().contains("installed during this call"));
        assert!(capture_warning(&s("installed_no_resubscribe")).is_some());
        assert!(capture_warning(&s("missing")).unwrap().contains("not loaded"));
        // No install line at all (the Lua read failed).
        assert!(capture_warning(&[]).unwrap().contains("could not be read"));
    }
```
(Fails if the warning is removed. The two keeper tests fail if `take_unreported_drops`/`dropped_unreported` is reverted: the first reader would report 0.)

**Lane checks**: the three commands, filters `keeper`, `capture_warning`, `events::` (runs the existing store and flow tests; none may change).

**Docs**
- `docs/guides/live-research-lab.md`: in the "Abilities, combat and event waits" section, after the sentence that says the supervisor is the only drainer of the bridge ring (search `only drainer`; the mod doc says the same), add: `` Between tool calls a per-launch keeper ticks once a second: it drains the bridge ring into the store (so the DLL's 4096-slot ring does not overflow into `bridge_dropped` while nobody is reading) and re-runs the ring install with `max = 0`, so the chat, combat and click wrappers are back within a second of a relog or an interface reload. It reads no ring record; those stay in the client until a reader pump. Set `CIMMERIA_LAB_EVENT_KEEPER=0` to turn it off. `bridge_dropped` in a pump report is what no earlier reader was told; `bridge_dropped_total` is the lab's running total. It counts the DLL's `cme.event`/`net.out`/`cegui.log` ring only. `chat.line`, `combat.hit` and `ui.click` come from in-client rings and are not part of it. ``
- Row at line 914 (`client_chat_log`): append ` The result has \`capture_warning\` when the chat wrapper was installed by this very call (lines before it are not in the log; read the chat tail of \`client_ui_state\`).` 
- `docs/guides/automated-uat.md` line 392: replace `- the bridge ring dropped events before the store saw them (\`bridge_dropped\`);` with `- the bridge ring dropped events before the store saw them (\`bridge_dropped\`; a background keeper drains the ring every second, so this only fires on a burst of more than about 4000 events inside one second, and a drop it saw between two reads is still reported to the next read);`

**Live smoke** (needs P2 first): `lab_logout`, `lab_ensure_in_world`, wait 3 s, then immediately `client_type_text` `.bankexpand` (any command with a server reply) followed by `client_chat_log {cursor:"smoke"}`: the reply line is present and `capture_warning` is absent; `capture.lua_install` shows `chat: ok`. Server evidence: the `expand_rejected`/reply row in ClickHouse for the same second. Second step: leave the client idle 60 s in the Castle, then `client_events_read {max:1}`: `dropped` is 0 or small and `capture`-style `bridge_dropped_total` stays below the P3 figure.

**Commit**: `fix(lab): keep chat capture armed and the bridge ring drained between calls; report drops honestly (LH-11)`

---

### LH-12: say why the click ring failed to install (S)

depends-on: P4. LH-11 (the keeper retries the install each second, which also covers a first pump before the world loaded).

**Files**
- `crates/lab/src/supervisor/events/lua_rings.rs` (install chunk, one test)
- `docs/guides/live-research-lab.md`

**Steps**

1. `lua_rings.rs` line 238-239 currently:

```
  local oks = pcall(function() host:subscribeGlobal(ns, ev, "{click_handler}") end)
  if oks then L.click_host = host st_click = "installed" else st_click = "failed" end
```
Replace with (note `serr` and the 120-char clip, `clean` strips tabs and newlines so the status stays one tab-separated field):

```
  local oks, serr = pcall(function() host:subscribeGlobal(ns, ev, "{click_handler}") end)
  if oks then L.click_host = host st_click = "installed" else st_click = "failed: " .. string.sub(clean(serr), 1, 120) end
```
The pcall at line 224 (`local okc = pcall(function()`) swallows an error raised before this; change it to `local okc, cerr = pcall(function()` and line 241 `if not okc then st_click = "failed" end` to `if not okc then st_click = "failed: " .. string.sub(clean(cerr), 1, 120) end`.

2. Update the doc comment on `RingRead.install` (line 282-286): `failed` becomes `failed: <error text>`. Nothing else matches on the literal `failed` (grep of `lua_install` shows only `combat_log.rs:112`, which reads the `combat` ring status).

3. Fix per P4: if P4 shows the `CEGUI.Window.*` constants are the problem, change lines 233-237 to use only the literals (`local ns, ev = "Window", "MouseClick"`) and update the test `chunk_subscribes_the_click_ring_globally_and_reads_after_its_mark` (lines 425-427) which asserts those two `CEGUI.Window` lines. If P4 shows `Root` is the problem, swap the order so `BackgroundWin` is tried first and update `the_click_install_is_idempotent` (lines 463-468: it asserts Root before BackgroundWin). If P4 shows the subscribe succeeds, change nothing else.

**Test** (in `lua_rings.rs` `tests`):

```rust
    /// A failed click install must carry its reason: `failed` alone gave the
    /// caller nothing to act on (#1660).
    #[test]
    fn a_failed_click_install_reports_the_error_text() {
        let c = pump_chunk(0, 0, 0, READ_MAX);
        assert!(c.contains(
            r#"local oks, serr = pcall(function() host:subscribeGlobal(ns, ev, "CimmeriaLab.on_click") end)"#
        ));
        assert!(c.contains(r#"st_click = "failed: " .. string.sub(clean(serr), 1, 120)"#));
        assert!(c.contains(r#"local okc, cerr = pcall(function()"#));
        // The status stays a single tab-separated field.
        let r = parse_ring_read(&s(&["install\tclick\tfailed: bad argument #2 to 'subscribeGlobal'"]));
        assert_eq!(
            r.install,
            vec![("click".to_string(), "failed: bad argument #2 to 'subscribeGlobal'".to_string())]
        );
        assert!(r.bad_lines.is_empty());
    }
```
(Fails when reverted: the old chunk has no `serr`.)

**Lane checks**: the three commands, filter `lua_rings`.

**Docs**: `docs/guides/live-research-lab.md`: where the click ring statuses are described (search `installed_no_resubscribe` / `ui.click`, near line 697-708) add: `` A click ring that cannot install reports `failed: <the Lua error>` in `lua_install`; an `Error`-free `missing` means the host window is not up yet. `bridge_dropped` in the same report is a different ring (see the keeper paragraph). ``

**Live smoke**: `client_chat_log` (any), read `capture.lua_install`: `click: installed` (or `ok`) after P4's fix; open and close a window by `client_window_click` and `client_wait_event {kind:"ui.click"}` returns the click. If it still fails, the status now names the Lua error: record it in the LR-13 worknote.

**Commit**: `fix(lab): click ring install failure reports its error text (LH-12)`

---

## Owner decisions

1. **Entity arrival default 2.5 m to 4.0 m** (LH-10). Recommend yes: it is inside the 3.5-4.5 m the briefs and item 9 say works, and 2.5 m is where the avatar blocks its own click (item 7). Callers who pass `arrival_m` are unchanged.
2. **Keeper on by default** (LH-11). Recommend yes (it makes relog capture correct without a brief change); it costs one `events_read` and one small `lua_eval` per second per live client, with an env switch off. With five clients that is ten bridge calls a second; if the lab host is CPU-starved (SV-15) set `CIMMERIA_LAB_EVENT_KEEPER=0` there.
3. **No teleport or held-key fallback inside `client_move_to`** (LH-10). Recommend not building one: a teleport is a server action (lab-server, not the lab crate) and a held key is already a `ui_sequence` step. The failure `hint` and the guide paragraph point to both. A real fix for straight-line walking is path-finding from the navmesh, which is a separate campaign.
4. Item 11 asks to "surface drop counts honestly": done as `bridge_dropped` (unreported) plus `bridge_dropped_total` plus `bridge_note`. No DLL change (the ring still drops the newest; changing it to drop the oldest or growing it needs a client-telemetry rebuild and a DLL redeploy to every lab client).
