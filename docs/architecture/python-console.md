---
title: "Python Console (historical)"
type: explanation
audience: engineers
last_updated: 2026-07-25
---

# Python Console (historical)

> [!IMPORTANT]
> **Historical record. The subsystem described here does not exist and cannot
> be enabled.**
>
> The Rust server embeds no Python interpreter. There is no `console_port`, no
> `py_console_password`, no `pyo3` or `rustpython` dependency anywhere under
> [`crates/`](../../crates/), and the `config/BaseService.config` /
> `config/CellService.config` files that configured it are gone — `config/`
> holds only `discord.toml.example`.
>
> **For the current equivalents, see [What replaced it](#what-replaced-it).**

## What this was

The deprecated C++/Python server exposed two Python REPLs on both BaseApp and
CellApp: a local stdin console (always on) and a password-gated remote TCP
console on port 8989 (BaseApp) / 8990 (CellApp), disabled by default because
the shipped password was empty. Neither had a command framework — both executed
arbitrary Python in the `__main__` namespace with full access to the `Atrea`
bridge module.

Separately, an in-game GM command framework parsed `/command args` from chat
and dispatched to registered handlers with per-command access-level and target
checks. The two systems shared nothing but the word "console". That framework's
command roster is the part of this document with lasting value, and it is
preserved [below](#operator-capability-inventory-116-commands).

This page was reduced on 2026-07-25. What was dropped: the byte-level wire
format for the `py_client` TCP protocol (message IDs `0x01`–`0x06`,
authenticate / evaluate / execute), a working Python reference client, the
`Atrea` module's function tables, and the eval-vs-exec semantics. All of it
described a protocol nothing will ever speak again, and all of it remains
recoverable from the C++ source still in the repo at
[`deprecated/cpp/src/entity/py_client.cpp`](../../deprecated/cpp/src/entity/py_client.cpp)
and `py_client.hpp`. The `Atrea` bridge API is summarised in
[tech-stack-replacement.md](tech-stack-replacement.md#python-c-bridge-api).

## What replaced it

| Old mechanism | Current mechanism |
|---|---|
| In-game `/command` GM chat commands | The client's **native** `/` console, gated server-side on `access_level`. See [gm-cell-method-gating.md](gm-cell-method-gating.md). |
| GM commands with no native slash binding | The **`.`-prefixed dev console** — chat-intercept channel, registry dispatch, record-then-confirm authoring. See [dev-console-channel.md](dev-console-channel.md); live roster in [`crates/services/src/cell/console/registry.rs`](../../crates/services/src/cell/console/registry.rs). |
| Remote TCP Python REPL | The `cimmeria-admin-api` REST + WebSocket surface ([`crates/admin-api/`](../../crates/admin-api/)), documented in [../tools/admin-api.md](../tools/admin-api.md). |
| `Atrea.dbQuery` / `dbPerform` ad-hoc SQL | `sqlx` against PostgreSQL, plus the admin API's typed routes. |

## The security model, and why it still matters

This is the part of the original design that informs current work, so it is
kept in full rather than summarised.

The Python console granted **unrestricted access to the server runtime**.
Anyone who reached it could execute arbitrary code in the server process, read
and write the database directly, create or destroy any entity, read all player
data, and crash the server. Its mitigations were thin:

1. **Disabled by default** — the shipped `py_console_password` was empty, so
   the TCP listener never started.
2. **Password-gated** — TCP clients authenticated before executing anything.
3. **Plain-text password** — sent unencrypted over TCP.
4. **No TLS** — the whole protocol was unencrypted, with no wrapping option.
5. **No rate limiting** — nothing stopped brute-force password guessing.
6. **No audit trail** — executed statements were logged at TRACE only.

The standing recommendation was to firewall the port to localhost or trusted
management IPs, and in practice to leave the TCP console disabled entirely and
use the local stdin console.

**Why this is not just history.** Read items 3–6 as a checklist and point it at
today's remote-administration surface. The admin API currently ships with **no
authentication** and binds `0.0.0.0` on port 8443 (tracked as issue #439) —
which is a weaker position than the console it replaced, since the console at
least demanded a password. The 2009-era design had already identified the right
four questions: is it authenticated, is it encrypted, is it rate-limited, is it
audited. Any future remote-admin work should be able to answer all four, and
the GM action-log gap is tracked in
[gm-cell-method-gating.md](gm-cell-method-gating.md#moderation-surface-still-missing).

## Operator capability inventory (116 commands)

Preserved as the record of what the original operators could actually do. This
is the roster the Rust `.`-console and native `/gm*` surfaces were derived from
— when you want to know whether a capability existed in the original game, this
table is the answer, not the current registry.

Paths below (`python/cell/…`) now resolve under
[`deprecated/python/`](../../deprecated/python/). Access level 1 means GM;
level 0 means any player. "Target" is the entity class the command required to
be selected.

### General (3 commands)

| Command | Access | Target | Description |
|---------|--------|--------|-------------|
| `help` | 0 | -- | Shows help about console commands |
| `loglevel` | 1 | -- | Updates log level for one or more event category |
| `logclient` | 1 | -- | Enable/disable sending of server log messages to the client |

### Player Commands (24 commands)

| Command | Access | Target | Description |
|---------|--------|--------|-------------|
| `kill` | 1 | SGWBeing | Kill target |
| `revive` | 1 | SGWBeing | Revive target |
| `clearabilities` | 1 | SGWPlayer | Clear all abilities |
| `giveaddress` | 1 | SGWPlayer | Give a stargate address |
| `giveability` | 1 | SGWPlayer | Give an ability |
| `givecash` | 1 | SGWPlayer | Give cash |
| `giveitem` | 1 | SGWPlayer | Give an item |
| `giverespawner` | 1 | SGWPlayer | Give a respawn point |
| `givetp` | 1 | SGWPlayer | Give training points |
| `givexp` | 1 | SGWPlayer | Give experience points |
| `god` | 1 | SGWPlayer | Toggle god mode |
| `listabilities` | 1 | SGWPlayer | List all abilities |
| `dynamicupdate` | 1 | SGWSpawnableEntity | Dynamic property update |
| `adddialog` | 1 | SGWPlayer | Add a dialog |
| `removeaddress` | 1 | SGWPlayer | Remove a stargate address |
| `removedialog` | 1 | SGWPlayer | Remove a dialog |
| `removeitem` | 1 | SGWPlayer | Remove an item |
| `removerespawner` | 1 | SGWPlayer | Remove a respawn point |
| `reloadmap` | 1 | SGWPlayer | Reload map data |
| `save` | 1 | -- | Save player state |
| `goto` | 1 | -- | Teleport to a player |
| `summon` | 1 | -- | Summon a player to you |
| `gotolocation` | 1 | -- | Teleport to a named location |
| `gotoxyz` | 1 | -- | Teleport to coordinates |

### Entity Commands (27 commands)

| Command | Access | Target | Description |
|---------|--------|--------|-------------|
| `info` | 1 | -- | Show entity info |
| `location` | 1 | SGWSpawnableEntity | Show entity location |
| `rotation` | 1 | SGWSpawnableEntity | Show entity rotation |
| `facing` | 1 | SGWSpawnableEntity | Show entity facing direction |
| `lookat` | 1 | SGWSpawnableEntity | Make entity look at target |
| `visible` | 1 | -- | Toggle entity visibility |
| `staticmesh` | 1 | SGWSpawnableEntity | Set static mesh |
| `bodyset` | 1 | SGWSpawnableEntity | Set body set |
| `nameid` | 1 | SGWSpawnableEntity | Set name ID |
| `eventset` | 1 | SGWSpawnableEntity | Set event set |
| `interactiontype` | 1 | SGWSpawnableEntity | Set interaction type |
| `interact` | 1 | SGWSpawnableEntity | Trigger interaction |
| `initialresponse` | 1 | SGWSpawnableEntity | Set initial response |
| `tag` | 1 | SGWSpawnableEntity | Set entity tag |
| `level` | 1 | SGWBeing | Set level |
| `name` | 1 | SGWBeing | Set name |
| `alignment` | 1 | SGWBeing | Set alignment |
| `faction` | 1 | SGWBeing | Set faction |
| `speed` | 1 | SGWBeing | Set movement speed |
| `addcomponent` | 1 | SGWBeing | Add visual component |
| `delcomponent` | 1 | SGWBeing | Remove visual component |
| `setstate` | 1 | SGWBeing | Set state flag |
| `unsetstate` | 1 | SGWBeing | Unset state flag |
| `setcombatant` | 1 | SGWBeing | Set combatant flag |
| `unsetcombatant` | 1 | SGWBeing | Unset combatant flag |
| `health` | 1 | SGWBeing | Set health |
| `focus` | 1 | SGWBeing | Set focus |

### Stats Commands (7 commands)

| Command | Access | Target | Description |
|---------|--------|--------|-------------|
| `stats` | 1 | SGWBeing | Show all stats |
| `primarystats` | 1 | SGWBeing | Show primary stats |
| `speedstats` | 1 | SGWBeing | Show speed stats |
| `armorstats` | 1 | SGWBeing | Show armor stats |
| `qrstats` | 1 | SGWBeing | Show QR stats |
| `absorbstats` | 1 | SGWBeing | Show absorb stats |
| `stealthstats` | 1 | SGWBeing | Show stealth stats |

### Mob Commands (3 commands)

| Command | Access | Target | Description |
|---------|--------|--------|-------------|
| `aggression` | 1 | SGWMob | Set aggression level |
| `threaten` | 1 | SGWMob | Generate threat |
| `combatinfo` | 1 | SGWMob | Show combat info |

### Mission Commands (14 commands)

| Command | Access | Target | Description |
|---------|--------|--------|-------------|
| `missionaccept` | 1 | SGWPlayer | Accept a mission |
| `missionabandon` | 0 | SGWPlayer | Abandon a mission |
| `missionadvance` | 1 | SGWPlayer | Advance a mission step |
| `missionclear` | 1 | SGWPlayer | Clear a specific mission |
| `missionclearactive` | 1 | SGWPlayer | Clear all active missions |
| `missionclearhistory` | 1 | SGWPlayer | Clear mission history |
| `missioncomplete` | 1 | SGWPlayer | Complete a mission |
| `missionfail` | 1 | SGWPlayer | Fail a mission |
| `missionlist` | 1 | SGWPlayer | Display mission list |
| `missionlistfull` | 1 | SGWPlayer | Display full mission list |
| `missiondetails` | 1 | SGWPlayer | Display mission details |
| `missionreload` | 1 | SGWPlayer | Reload mission data |
| `missionreset` | 1 | SGWPlayer | Reset a mission |
| `missionrewards` | 1 | SGWPlayer | Display mission rewards |

### Crafting Commands (5 commands)

| Command | Access | Target | Description |
|---------|--------|--------|-------------|
| `appliedscience` | 1 | SGWPlayer | Give applied science points |
| `racialparadigm` | 1 | SGWPlayer | Set racial paradigm level |
| `learndiscipline` | 1 | SGWPlayer | Learn a crafting discipline |
| `forgetdiscipline` | 1 | SGWPlayer | Forget a crafting discipline |
| `allcraft` | 1 | SGWPlayer | Debug: unlock all crafting |

### Resource Commands (11 commands)

| Command | Access | Target | Description |
|---------|--------|--------|-------------|
| `searchitem` | 1 | -- | Search for items by name |
| `searchmission` | 1 | -- | Search for missions by name |
| `searchtemplate` | 1 | -- | Search for entity templates by name |
| `reloadres` | 1 | -- | Reload resources |
| `respawnall` | 1 | -- | Respawn all entities |
| `autosavespawn` | 1 | -- | Toggle autosave on spawn |
| `spawn` | 1 | -- | Spawn an entity |
| `spawnrandom` | 1 | -- | Spawn a random entity |
| `despawn` | 1 | SGWSpawnableEntity | Despawn an entity |
| `savespawn` | 1 | SGWSpawnableEntity | Save spawn data |
| `delspawn` | 1 | SGWSpawnableEntity | Delete spawn data |

### Network/Debug Commands (11 commands)

| Command | Access | Target | Description |
|---------|--------|--------|-------------|
| `net_dhd` | 1 | -- | Display DHD (stargate dialing device) |
| `net_seq` | 1 | SGWSpawnableEntity | Play a sequence |
| `net_seqto` | 1 | -- | Play sequence on target |
| `net_seqfrom` | 1 | SGWSpawnableEntity | Play sequence from entity |
| `net_timer` | 1 | SGWSpawnableEntity | Update timer |
| `net_timeofday` | 1 | SGWPlayer | Set time of day |
| `net_mapinfo` | 1 | SGWPlayer | Display map info |
| `net_speak` | 1 | SGWSpawnableEntity | Player communication |
| `net_minigame` | 1 | SGWPlayer | Start a minigame |
| `net_dialog` | 1 | -- | Open a dialog |
| `net_challenge` | 1 | -- | Send client challenge |

### Debug Commands (11 commands)

| Command | Access | Target | Description |
|---------|--------|--------|-------------|
| `debug_velocity` | 1 | SGWSpawnableEntity | Debug velocity display |
| `debug_controller` | 1 | SGWSpawnableEntity | Debug controller info |
| `debug_follow` | 1 | SGWSpawnableEntity | Debug follow behavior |
| `debug_paths` | 1 | SGWSpawnableEntity | Debug pathfinding |
| `debug_nav` | 1 | SGWSpawnableEntity | Debug navigation |
| `debug_events` | 1 | SGWSpawnableEntity | Debug events |
| `debug_ai` | 1 | SGWMob | Debug AI state |
| `debug_inven` | 1 | SGWPlayer | Debug inventory |
| `debug_invreload` | 1 | SGWPlayer | Reload inventory |
| `reloadscripts` | 1 | -- | Reload Python scripts |
| `players` | 1 | -- | List online players |

## Related documents

- [dev-console-channel.md](dev-console-channel.md) — the `.`-console ADR: which
  of the commands above have Rust equivalents, and their per-command status.
- [gm-cell-method-gating.md](gm-cell-method-gating.md) — how `access_level` is
  enforced today, and what moderation tooling is still missing.
- [../tools/admin-api.md](../tools/admin-api.md) — the remote-administration
  surface that replaced the TCP console.
- [service-architecture.md](service-architecture.md) — the deprecated server's
  process topology, carrying the same historical caveat.
- [tech-stack-replacement.md](tech-stack-replacement.md) — the `Atrea` bridge
  API inventory and the decision that retired all of this.
