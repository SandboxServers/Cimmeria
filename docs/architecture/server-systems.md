# Server-Only Infrastructure Systems

> **Last updated**: 2026-03-01
> **Scope**: Server-side infrastructure with no client-facing wire format

---

## Overview

This document covers eight infrastructure systems that exist entirely on the server side. None of them have corresponding entity `.def` files, client RPC methods, or wire format documentation, because the client has no direct awareness of them. They are administrative, protective, or operational concerns that a production MMO server requires in addition to the gameplay systems.

The systems are documented in the order in which they should be implemented for a functional server.

---

## 1. Session Management

### Overview

Session management handles the lifecycle of a player connection from login through logout, including inactivity timeouts, reconnection grace periods, and duplicate login prevention. Without it, disconnected players leave stale entities in the world and reconnecting players lose all in-progress state.

### Current State

The foundation exists but the implementation is incomplete.

**Inactivity timeout** is configured in `config/BaseService.config`:

```xml
<client_inactivity_timeout>300000</client_inactivity_timeout>
```

This is 300,000 milliseconds (5 minutes). The C++ networking layer enforces it. When the timeout elapses with no traffic from the client, the connection is closed.

**Duplicate login prevention** is implemented in `python/base/Account.py`. When a client requests the character list, each character is marked as non-playable if it is already online:

```python
# python/base/Account.py, sendCharacterList()
if Config.DISALLOW_DUPLICATE_CHARACTERS and ChannelManager.isPlayerOnline(char['player_name']):
    playable = 0
```

The check runs again in `playCharacter()` before creating the entity. The `DISALLOW_DUPLICATE_CHARACTERS` flag is `True` by default and is only cleared in developer mode (`config/BaseService.config`: `<developer_mode>false</developer_mode>`).

**Developer mode** grants maximum access level to all players and allows duplicate logins:

```python
# python/common/Config.py
DEVELOPER_MODE = Atrea.config.developer_mode == "true"
DISALLOW_DUPLICATE_CHARACTERS = True and not DEVELOPER_MODE
```

**Access level** is loaded from the database on login and elevated to 65535 in developer mode:

```python
# python/base/SGWPlayer.py, load()
if Config.DEVELOPER_MODE:
    self.accessLevel = 65535
```

### Gaps

- **No reconnection grace period.** When a client disconnects, its session is immediately invalidated. A transient network drop causes full session loss. The player must log in again from the character selection screen and the cell entity is destroyed and recreated.
- **No session token.** There is no persistent session identifier. The server cannot distinguish a reconnect from a fresh login.
- **No CellApp migration handling.** Zone transitions create a new cell entity on a potentially different CellApp. There is no handoff protocol for in-flight state during the transition.
- **Duplicate login check is not continuous.** The check in `sendCharacterList()` and `playCharacter()` runs at connection time only. A malicious client that bypasses character selection could potentially create a duplicate entity if it could issue the correct native calls.

### Recommended Approach

Add a 30-60 second reconnection window. When the TCP connection drops, mark the session as `RECONNECTING` rather than destroying it immediately. Store a session token (a random 128-bit value generated at login) in the BaseApp entity. On reconnect, if the client presents a valid token within the window, restore the entity to the client channel without destroying and recreating the cell entity. Use the existing `client_inactivity_timeout` (5 minutes) as the hard limit: a session that has been in `RECONNECTING` state for longer than the inactivity timeout is destroyed.

The session token can be returned to the client in the login response and passed back in the reconnect handshake. No schema change is required; the token lives in BaseApp memory for the duration of the session window.

### Priority

**HIGH.** Network blips during zone transitions or poor connectivity will cause full disconnects under the current implementation. This is a basic quality-of-service requirement for any sustained test session, not just for production.

---

## 2. Rate Limiting

### Overview

Rate limiting prevents clients from flooding the server with requests faster than they can be legitimately generated. Without it, a modified client can spam abilities, chat messages, and trade requests at arbitrary rates, either exploiting game mechanics or degrading server performance.

### Current State

Per-ability cooldowns are implemented in `python/cell/AbilityManager.py`. When an ability is used, a timer is registered:

```python
# python/cell/AbilityManager.py
self.cooldownTimers[ability.id] = Atrea.addTimer(completeTime, lambda: self.abilityCooledDown(ability))
```

Monikered cooldowns (grouped cooldowns shared across ability families) are tracked separately in `self.monikerCooldowns`. These are per-player, per-ability timers enforced on the server.

The chat system in `python/base/Chat.py` routes messages between channels with no rate limit check. The trade proposal system uses a version counter to prevent desync but has no request-rate cap.

### Gaps

- **No per-player action rate tracking.** The server tracks cooldowns per-ability but does not track the rate at which a player sends any category of request.
- **No chat flood protection.** A client can send an unlimited number of `sendPlayerCommunication` calls per second. The ChannelManager will route all of them.
- **No movement command throttle.** Position updates arrive from the client and are accepted without rate checking. A client sending hundreds of position updates per tick is not detected.
- **No ability use rate cap beyond individual cooldowns.** A client can attempt to use abilities that are on cooldown repeatedly; the server rejects each attempt but processes the validation for each one.
- **No trade request spam protection.** The trade proposal flow has no cap on how many proposals can be initiated per minute.
- **No mail rate limit.** The gate mail system (`SGWMailManager`) has no send-rate cap.

### Recommended Approach

Implement per-player, per-category token bucket rate limiting in the CellApp entity tick. The categories are: chat, ability use attempts, trade requests, and mail sends. Each category has a bucket with a refill rate and a burst capacity. For example, chat might allow 5 messages per second with a burst of 10. Ability use attempts (distinct from successful uses, which are governed by cooldowns) might allow 20 per second to accommodate latency-driven retries.

Implement in Python before enforcing: log violations at the `WARN` level for a period before activating hard refusals. This allows calibration against legitimate player behavior before blocking anyone.

The tick rate is 100ms (`<tick_rate>100</tick_rate>` in `config/BaseService.config`), giving 10 ticks per second as the measurement resolution.

### Priority

**MEDIUM.** With a small player base during development and testing, chat flood and ability spam are unlikely to be serious problems. The existing per-ability cooldown system covers the most important abuse vector. Rate limiting becomes more important before any open testing phase.

---

## 3. Anti-Cheat and Server-Side Validation

### Overview

Server-side validation detects and rejects client inputs that violate game rules. Movement is the most critical area: if the client is the authority on its own position, a modified client can teleport anywhere on the map. Damage validation prevents abilities from dealing unlimited damage if ability parameters are manipulated client-side.

### Current State

Several validation layers exist.

**Position bounds checking** is performed by the `PlayerController` in C++. The player's position is validated to be within the space bounds before being accepted.

**Target validation** in `python/cell/AbilityManager.py` verifies that the target entity exists and is alive before an ability is applied. Abilities that require a target check `self.target is not None` and `not self.target.isDead()`.

**Inventory ownership** is validated in the inventory and trade systems before items are transferred.

**God mode** is a server-side debug flag (`cheatFlags` in `python/cell/SGWBeing.py`) that prevents health loss:

```python
# python/cell/SGWBeing.py, onHealthChanged()
if newHealth < oldHealth and self.cheatFlags & Constants.CHEAT_Invulnerable:
    return oldHealth
```

This flag is set only by the `god` console command (`python/cell/commands/Player.py`). It is not client-settable.

**Authentication challenge** is issued post-login in the `AuthenticationServer`. This is the only anti-cheat measure applied at connection time.

### Gaps

- **No movement speed validation.** The client sends position updates and the server accepts them. The `movementSpeedMod` stat exists (`Atrea.enums.movementSpeedMod`, defined in `python/Atrea/enums.py` as stat ID 6), and the server tracks it, but no code compares reported position deltas against the expected speed.
- **No teleport detection.** A client that sends a position update placing the player across the map will be accepted without error.
- **No damage sanity cap.** Ability damage is calculated by the server based on stats and ability definitions, so the damage value itself is server-computed. However, there is no maximum cap check to detect cases where a stat modifier bug or configuration error allows implausible damage values.
- **No action-at-distance validation.** The `MAX_INTERACT_DISTANCE` constant (defined as 5 units in `python/common/Constants.py`) is enforced for interactions, but ability range is not verified server-side against the actual entity separation distance.

### Recommended Approach

Server-side position prediction using the `movementSpeedMod` stat. On each position update from the client, compute the maximum distance the player could have traveled since the last accepted position given `movementSpeedMod * elapsed_time * tolerance_multiplier`. If the submitted position exceeds this bound, log a warning at the `WARN` level rather than hard-rejecting. Flag the player after a configurable number of violations within a rolling window. Apply position correction only for extreme violations (e.g., over 10x expected distance).

Start with a generous tolerance multiplier (e.g., 1.5) to accommodate packet loss and latency. Tune it down only after observing false positive rates in test sessions.

For ability range, add a distance check in `AbilityManager` before applying targeted abilities. Use the entity positions from the server's own spatial database, not client-reported values.

### Priority

**MEDIUM.** With a small and trusted player base during development, active cheating is not an immediate concern. Position validation becomes important before any open testing phase because movement hacking is one of the simplest exploits to attempt with a game client.

---

## 4. Economy Sinks and Faucets

### Overview

Economy balance ensures that currency (naquadah) enters and leaves the game at sustainable rates. Without managed sinks, players accumulate unlimited currency over time, which devalues rewards and breaks progression pacing.

### Current State

**Currency faucets** (money entering the economy):

- Mission completion rewards: `mission.rewardNaq` is set from the `resources.mission_rewards` table and paid out on mission completion (`python/common/defs/Mission.py`, line 82: `self.rewardNaq = row['reward_naq']`).
- Loot drops: the `LOOT_Cash` loot type (value 2, defined in `entities/editor/enumerations.xml`) places currency directly into the player inventory.
- Vendor sell-back: players receive currency when selling items to vendors.

**Currency sinks** (money leaving the economy):

- Vendor purchases: players spend currency to buy items from vendors. Vendor types are categorized in `python/common/Constants.py` (`INTERACTION_VendorArmor`, `INTERACTION_VendorWeapons`, etc.).
- Crafting: items and components are consumed (indirect sink via item destruction, not direct currency loss).
- Trade: player-to-player transfer; zero-sum, not a true sink.

### Gaps

- **No vendor price scaling.** Prices are static values from the database item definitions. There is no mechanism to adjust prices based on server economy state.
- **No repair cost formula.** Item durability and repair mechanics are not documented or implemented. The cost of repairs (a classic MMO sink) is not calculated anywhere.
- **No cash flow logging.** The `LC_Cash` log category exists in `python/common/Logger.py` but currency transactions are not logged through it in any current code path.
- **No auction house fees.** The black market (`python/base/SGWBlackMarket.py`, `python/cell/SGWBlackMarket.py`) is entirely stubbed. Listing fees and transaction cuts are not implemented.
- **No mail COD or postage.** Gate mail (`SGWMailManager`) has no currency attachment or postage cost.
- **No travel costs.** Stargate travel and ring transporter use are free.

### Recommended Approach

Start with static pricing as currently implemented. The priority for economy work is instrumentation, not balance. Add logging through `LC_Cash` for every currency gain and loss event, with the source type tagged (mission, loot, vendor purchase, vendor sell). This creates the data needed to understand the current flow before attempting to adjust it.

When the black market system is implemented, use a standard auction house fee model: a non-refundable listing fee (e.g., 1-2% of listing price) plus a transaction cut on successful sales (e.g., 5%). These are standard MMO sink mechanics that are well-understood and easy to tune.

Repair costs, if implemented, should scale with item level and the extent of durability loss, not a flat fee.

### Priority

**LOW.** There is no functioning economy to balance yet. The black market is stubbed, and the player base is small enough that currency accumulation is not a practical problem. Economy instrumentation is the correct first step, not active balancing.

---

## 5. World State Persistence

### Overview

World state persistence saves the condition of interactive world objects between server restarts. Gates, doors, destructible objects, and event triggers that were modified during a session should not silently reset to defaults when the server restarts.

### Current State

**Player position** persists. The `sgw_player` table in `db/sgw.sql` stores `pos_x`, `pos_y`, `pos_z`, and `world_location` for each character. These are saved on logout.

**Entity templates** are loaded from `resources.entity_templates` on startup. Template-based entities spawn in their default state.

**Space scripts** (`python/cell/spaces/`) handle per-space event logic. Eleven space scripts are present: `Castle.py`, `Castle_CellBlock.py`, `Harset.py`, `Harset_CmdCenter.py`, `Lucia.py`, `Menfa_Dark.py`, `Omega_Site.py`, `Omega_Site_CmdCenter.py`, `SGC_W1.py`, `SandBox.py`. These scripts subscribe to player events and fire kismet sequences, but they hold all state in instance variables and do not persist anything.

**Space event system** uses `space.fire('event_name')` to trigger kismet sequences. The event system is stateless; there is no record of which events have fired.

### Gaps

- **No world state table.** There is no database table for interactive object state. All non-player entity state resets on restart.
- **Gate states are not persisted.** A gate that was left open at server shutdown reopens in closed state. `python/cell/GateTravel.py` and the DHD interaction (`python/cell/interactions/DHD.py`) manage gate state in memory only.
- **Door and elevator states are not persisted.** Space scripts that modify entity `interactionType` flags (as seen in `python/cell/spaces/SGC_W1.py`, for example, setting and clearing `interactionType | 256` for an elevator button) do not save these changes.
- **Destructible object state is not persisted.** Objects that have been destroyed or activated do not record this in the database.
- **No world event journal.** The server has no record of what happened in a space before the current session started.

### Recommended Approach

Add a `sgw_world_state` table to the database schema:

```sql
CREATE TABLE sgw_world_state (
    space_id    VARCHAR(64)   NOT NULL,
    entity_tag  VARCHAR(64)   NOT NULL,
    state_key   VARCHAR(64)   NOT NULL,
    state_value TEXT          NOT NULL,
    updated_at  TIMESTAMP     NOT NULL DEFAULT NOW(),
    PRIMARY KEY (space_id, entity_tag, state_key)
);
```

Space scripts save critical state (gate open/closed, key event flags) on change and load it during the `__init__` setup phase. The `Script` base class (`python/cell/Script.py`) provides the right place to add `saveState(key, value)` and `loadState(key, default)` helpers.

Avoid persisting all entity state. Most objects should reset on restart; only objects whose state is meaningful across sessions (gates, mission-critical interactables, door states that affect area access) need persistence.

### Priority

**MEDIUM.** Gate state is the most visible case: players dialing a gate and then restarting the server will find the gate closed without explanation. This is a testability issue as much as a persistence issue. Implementing the table and adding save/load to the gate system is a small, well-scoped task.

---

## 6. Event and Scheduler System

### Overview

A scheduler system handles timed world events, daily resets, vendor restocks, and any content that needs to happen on a schedule rather than in response to player action. Without it, all timed behavior is tied to individual entity lifetimes and cannot survive server restarts.

### Current State

**Per-entity timers** are available via `Atrea.addTimer(completeTime, callback)` (defined in `python/Atrea/__init__.py`, line 472). This is used extensively:

- Crafting timers in `python/cell/Crafter.py`
- Ability cooldowns and pulse timers in `python/cell/AbilityManager.py`
- Ring transporter animation timers in `python/cell/RingTransporter.py`
- Mob AI action timers in `python/cell/SGWMob.py`
- Minigame tick timers in `python/base/minigame/`

These timers are fire-once or self-rescheduling callbacks. They are tied to the entity that creates them and are lost when the entity is destroyed or the server restarts.

**The `onTimeOfDayTick` method** is declared on `SGWSpawnRegion` in the entity def system. The cell-side `SGWSpawnRegion` (`python/cell/SGWSpawnRegion.py`) has only a stub `__init__`. The method is not implemented.

There is no global scheduler entity, no cron-like mechanism, and no persistent event store.

### Gaps

- **No global event scheduler.** All timed events are attached to entity lifetimes. There is no singleton that survives independently to schedule server-wide events.
- **No periodic world events.** Dynamic content (invasions, patrols, world-state changes on a schedule) is not possible without a global scheduler.
- **No daily reset.** Daily mission refresh, vendor restock, and other periodic content resets have no infrastructure to run on.
- **No maintenance window scheduling.** There is no mechanism to announce and enforce scheduled downtime.
- **No holiday or seasonal event system.** Time-based content activation is not possible.

### Recommended Approach

Implement a scheduler as a Python singleton entity on the BaseApp or as a module-level object initialized at server startup. Register events with a schedule description: a fixed timestamp, a recurring interval, or a cron-like specification (day of week, time of day). Store scheduled events in a database table so they survive restarts:

```sql
CREATE TABLE sgw_scheduled_events (
    event_id    SERIAL        PRIMARY KEY,
    event_name  VARCHAR(64)   NOT NULL,
    next_run    TIMESTAMP     NOT NULL,
    interval_s  INTEGER,      -- NULL for one-shot events
    enabled     BOOLEAN       NOT NULL DEFAULT TRUE
);
```

Space scripts and game systems subscribe to named events from the scheduler (e.g., `scheduler.subscribe('daily_reset', self.onDailyReset)`). The scheduler fires the event and reschedules it based on the interval.

Use `Atrea.addTimer` as the underlying mechanism for the scheduler's own tick: the scheduler entity reschedules itself each time it fires, checking the `sgw_scheduled_events` table for events that are due.

### Priority

**LOW.** No scheduled content exists yet. The scheduler system is needed before daily missions, vendor restocks, or periodic world events can be designed, but none of those systems are currently implemented. Build this when the first feature that requires it is being designed.

---

## 7. Admin and GM Tools Backend

### Overview

The GM tools backend provides server administration capability: player lookup, moderation actions (ban, mute), in-game announcements, entity inspection, and action logging. This is required for testing and for managing a live server.

### Current State

The infrastructure exists; the implementations are absent.

**Python console** is available on port 8989. It is gated by `<py_console_password>` in `config/BaseService.config`. The password field is currently empty, so the console server is not started by default.

**Console command framework** is fully implemented in `python/cell/ConsoleCommands.py`. Commands are registered with a name, handler function, minimum access level, and required target type. The `Command.call()` method validates access level, argument count and types, and target type before dispatching to the handler. Over 50 commands are registered covering player manipulation, entity editing, mission management, crafting, resource management, and networking diagnostics.

**Access level system** is present: `account.access_level` is stored in the `account` table in `db/sgw.sql` and loaded into the player entity in `python/base/SGWPlayer.py`. Commands require `accessLevel >= 1` for all current entries.

**SGWGmPlayer** (`python/base/SGWGmPlayer.py`) is a subclass of `SGWPlayer` that is instantiated when `access_level > 0`. Currently it only overrides `loadConstants()` to reload Python scripts.

**Existing working commands** (access level 1) include: `kill`, `revive`, `goto`, `summon`, `giveitem`, `givecash`, `givexp`, `giveability`, `god`, `spawn`, `despawn`, `missionaccept`, `missioncomplete`, `missionfail`, `save`, `players`, `reloadscripts`, and many more. These are functional and cover most development testing needs.

### Gaps

- **No action logging.** GM actions are not recorded anywhere. There is no `sgw_gm_log` table or equivalent. A GM granting items or currency leaves no audit trail.
- **No ban or mute system.** There are no database fields or server commands to ban an account or mute a player. `chatBan` and `chatMute` in `python/base/SGWPlayer.py` are stubs that only call `trace()`.
- **No announcement command.** There is no server-wide broadcast mechanism. `announcePetition` is a stub.
- **No player investigation tools.** There is no command to look up a player's account details, current location, or recent activity from the console.
- **The Python console is disabled.** The empty `py_console_password` means the console is not started unless a password is configured.
- **No rollback capability.** There is no mechanism to reverse the effects of a GM action or a player exploit.

### Recommended Approach

Enable the console by setting a password in `config/BaseService.config` or a local override file. This is the single highest-leverage action available: dozens of existing working commands become usable immediately.

For action logging, add a `sgw_gm_log` table and wrap the `Command.call()` dispatch to log the command name, player name, target name, and arguments for any command executed by a player with access level > 0.

For ban and mute, add `is_banned` and `is_muted` columns to the `account` table, check them at login (for ban) and in the chat routing path (for mute), and implement `ban` and `mute` console commands.

For announcements, add a `broadcast` command that calls `onPlayerCommunication` on all connected clients with a system channel message.

### Priority

**HIGH.** The console command framework already works; enabling the console (one config value) and the existing 50+ commands are immediately useful for testing. GM tools are required before any collaborative test session: the ability to grant items, reset missions, teleport, and inspect entities is necessary for content testing.

---

## 8. Metrics and Telemetry

### Overview

Metrics provide visibility into server health and player behavior: how many players are connected, which areas are populated, what the tick rate is under load, and whether anything is failing silently. Without metrics, problems are diagnosed reactively from log files after the fact.

### Current State

**Logging** is the primary observability mechanism. `python/common/Logger.py` provides category-based logging with 18 defined categories: `LC_Mission`, `LC_Item`, `LC_Location`, `LC_Ability`, `LC_Interact`, `LC_Cash`, `LC_Mob`, `LC_Chat`, `LC_LogInOut`, `LC_CharManagement`, `LC_Resources`, `LC_Minigame`, `LC_Stargate`, `LC_Mail`, `LC_Crafting`, `LC_Event`, `LC_Visuals`, and `LC_Uncategorized`. Each category has a configurable log level, adjustable at runtime via the `loglevel` console command.

**Client logging relay** is implemented: `setClientLogging` (`python/cell/ConsoleCommands.py`) can route server log messages to the connected GM client's feedback channel.

**Performance data from clients** is received. The `perfStats` method on `SGWPlayer` (`python/base/SGWPlayer.py`) receives `fpsAvg`, `fpsMin`, `fpsMax`, `bpsIn`, `bpsOut`, `packetsIn`, `packetsOut`, `lagAvgMS`, `lagMinMS`, `lagMaxMS`, `resends`, and `appearanceJobs` from each client. The method body is `pass`; the data is discarded.

**Server tick rate** is configured at 100ms (`<tick_rate>100</tick_rate>` in `config/BaseService.config`). Actual tick performance is not measured.

**Watcher system** is documented in `docs/engine/watcher-system.md` but is not implemented in Cimmeria. The BigWorld watcher system would have provided runtime variable inspection and metrics exposure over a UDP/TCP endpoint.

**`players` command** is registered in `ConsoleCommands.py` and dispatches to `cell.commands.Misc.players`. This likely lists connected players, though the implementation is in `python/cell/commands/Misc.py`.

### Gaps

- **No player count tracking.** There is no persistent counter of how many players are currently online. The `players` command may provide a list, but there is no time-series record.
- **No area population monitoring.** It is not known at a glance which spaces have players and how many.
- **No gameplay metrics.** Kills, deaths, items looted, missions completed, and abilities used are not counted.
- **No server performance metrics.** Actual tick duration, message queue depth, entity count, and memory usage are not exposed.
- **`perfStats` data is silently dropped.** Client-reported performance data (including average lag, resends, and bandwidth) is received and thrown away.
- **No crash reporting.** Exceptions in Python scripts are caught and logged, but there is no structured crash report or alert mechanism.
- **No anomaly alerting.** Nothing monitors for conditions that indicate a problem (e.g., tick rate degrading below threshold, entity count spiking unexpectedly).

### Recommended Approach

Start with the data already being received. `perfStats` is called by every connected client. Store the per-client lag average and resend count in the player entity and expose a summary via a `netstats` console command. This gives immediate visibility into client connection quality without any new infrastructure.

For server-side metrics, add a counter module that increments named counters for key events: logins, logouts, ability uses, entity spawns, entity destroys. Expose the counters via a `stats` console command. This is a few dozen lines of Python.

For ongoing monitoring, implement a periodic log flush (every 5 or 10 minutes) that writes a structured metrics line to the log: current player count, entity count in each space, and uptime. This creates a time-series record in the log that can be grepped after the fact.

The BigWorld watcher system (`docs/engine/watcher-system.md`) is the reference design for a more sophisticated monitoring approach, but its full implementation is not necessary for a development environment.

### Priority

**LOW.** Metrics are valuable but not blocking. A server can be tested and debugged with log files and the console alone. Metrics become important when the server is running continuously and operators need to understand behavior without being present at all times.

---

## Implementation Order Summary

| Priority | System | Rationale |
|----------|--------|-----------|
| HIGH | GM Tools Backend | Enable the console immediately (one config value); 50+ working commands become available |
| HIGH | Session Management | Reconnection grace period is needed for any sustained test session |
| MEDIUM | Rate Limiting | Required before any open testing to prevent abuse |
| MEDIUM | Anti-Cheat Validation | Required before any open testing; movement validation is the most critical gap |
| MEDIUM | World State Persistence | Gate state reset is a visible and disruptive problem during testing |
| LOW | Economy Sinks/Faucets | No economy to balance yet; add logging first |
| LOW | Event/Scheduler System | Build when the first scheduled-content feature is designed |
| LOW | Metrics and Telemetry | Log-based debugging is sufficient for early development |

The GM tools note warrants emphasis: the console command framework in `python/cell/ConsoleCommands.py` is fully implemented and the commands work. Setting `<py_console_password>` to any non-empty string in `config/BaseService.config` starts the console on port 8989 and makes all existing commands available. This is the single most impactful administrative step available without writing any new code.
