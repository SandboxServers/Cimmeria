# Python Console Reference

Complete reference for the Python console system available in BaseApp and CellApp. Both services expose two interfaces: a **local stdin console** and a **remote TCP console** -- each providing a raw Python REPL with full access to the `Atrea` module and all imported game modules.

Neither interface has a pre-registered command framework. They execute arbitrary Python code in the `__main__` namespace. For the in-game GM command system (which is separate), see [GM Commands vs Python Console](#gm-commands-vs-python-console).

---

## Configuration

Both services read two config parameters to control the remote TCP console:

```xml
<!-- Python console port -->
<console_port>8989</console_port>

<!--
    Password to Python console
    Console server is only started if a password is set
-->
<py_console_password></py_console_password>
```

| Service | Config File | Default Port | Password |
|---------|-------------|-------------|----------|
| BaseApp | `config/BaseService.config` | 8989 | (empty -- disabled) |
| CellApp | `config/CellService.config` | 8990 | (empty -- disabled) |

To enable the remote console, set `py_console_password` to a non-empty string. Use a `.local` override file for real deployments (e.g. `BaseService.config.local`) -- never commit passwords to the repository.

The local stdin console is always enabled and requires no configuration.

---

## Local Stdin Console

**Source:** `src/common/console.cpp`

A background `boost::thread` started during service initialization. Reads lines from `stdin` and executes them as Python statements via `bp::exec()` in the `__main__` module's global namespace.

### Startup

Both services start the console thread at the end of their initialization:

```cpp
boost::thread * t = new boost::thread(&ConsoleThread);
```

On Windows, the thread sleeps 1500ms before activating (`Sleep(1500)`) to let the service finish initialization output.

> **Known Bug:** The Linux path has `usleep(1500000000)` (1500 seconds) instead of `usleep(1500000)` (1.5 seconds). This is a 1000x error that delays the Linux console prompt by 25 minutes. See `src/common/console.cpp:18`.

### Prompt Loop

1. The first `std::getline()` blocks until the user presses Enter (any input is discarded -- it just activates the REPL).
2. Log level is temporarily reduced to `LL_WARNING` to suppress server noise.
3. The `>>> ` prompt is displayed in bright white (Windows: `SetConsoleTextAttribute`; Linux: ANSI `\033[37m`).
4. Each line is executed via `bp::exec(cmd, global, global)`.
5. During execution, log level is briefly restored to its original value so the executed code can produce log output, then re-suppressed.
6. Python exceptions are displayed in yellow (Windows: `SetConsoleTextAttribute(FOREGROUND_GREEN | FOREGROUND_RED | FOREGROUND_INTENSITY)`; Linux: ANSI `\033[33m`).
7. Typing `exit` or `quit` breaks the inner loop, restores the original log level, and returns to step 1.

### Example Session

```
>>> import Atrea
>>> Atrea.getGameTime()
>>> Atrea.findAllEntities()
>>> exit
```

---

## Remote TCP Console

**Source:** `src/entity/py_client.hpp`, `src/entity/py_client.cpp`

A password-authenticated TCP service using the UnifiedConnection protocol framing (same wire format as all Mercury connections). The server is only started if `py_console_password` is non-empty.

### Enabling

1. Set `py_console_password` in the service config (or `.local` override):
   ```xml
   <py_console_password>my_secret</py_console_password>
   ```
2. Restart the service. The log will show `Accepted connection` when a client connects.

### Server Startup

During service initialization, the console server is conditionally created:

```cpp
// BaseApp: base_service.cpp:246-251
if (!Service::instance().getConfigParameter("py_console_password").empty())
{
    consoleServer_ = new Mercury::TcpServer<py_client>(
        boost::lexical_cast<unsigned int>(
            Service::instance().getConfigParameter("console_port")),
        &py_client::create);
}
```

`TcpServer<py_client>` (`src/mercury/tcp_server.hpp`) is a Boost.Asio TCP acceptor that listens on the configured port with a backlog of 30, creates a `py_client` instance for each connection, and manages connection lifetime via `boost::shared_ptr`.

### Connection Lifecycle

```
Client                          Server
  |                                |
  |-------- TCP connect ---------> |  State: PY_CLIENT_AUTH_WAIT
  |                                |
  |-- PYS_REQ_AUTHENTICATE ------> |  Server checks password
  |<- PYS_ACK_AUTHENTICATE ------- |  0 = OK, 1 = fail
  |                                |  State: PY_CLIENT_OK (if success)
  |                                |
  |-- PYS_REQ_EVALUATE ----------> |  Evaluate expression
  |<- PYS_ACK_EVALUATE ----------- |  Result type + value
  |                                |
  |-- PYS_REQ_EXECUTE -----------> |  Execute statement
  |<- PYS_ACK_EXECUTE ------------ |  Success or exception
  |                                |
  |  (repeat eval/exec as needed)  |
  |                                |
  |-------- TCP close -----------> |
```

If a client sends an evaluate or execute request before authenticating, the server logs a warning and closes the connection immediately.

### Wire Format

All messages use the UnifiedConnection framing: a 5-byte header followed by the message payload.

#### Message Header

```
Offset  Size  Type      Field
------  ----  --------  -----------
0x00    4     uint32    length       Total message size in bytes (header + payload)
0x04    1     uint8     messageId    Protocol message ID
```

The header is packed (`#pragma pack(push, 1)` on MSVC, `__attribute__((packed))` on GCC) -- no padding between fields. The `length` field includes the 5-byte header itself, so the payload size is `length - 5`.

#### String Encoding

Strings are length-prefixed: a `uint32` byte count followed by the raw UTF-8 bytes (no null terminator).

```
Offset  Size    Type      Field
------  ------  --------  -----------
0x00    4       uint32    length      String length in bytes
0x04    length  bytes     data        Raw string bytes
```

### Protocol Messages

#### 0x01 -- PYS_REQ_AUTHENTICATE (Client -> Server)

Send the console password to authenticate.

```
Header (5 bytes)
+--------+--------+--------+--------+--------+
| length (uint32, little-endian)    | 0x01   |
+--------+--------+--------+--------+--------+

Payload
+--------+--------+--------+--------+--------+--- - -+
| password_length (uint32)          | password bytes   |
+--------+--------+--------+--------+--------+--- - -+
```

**Example** (password = `"test"`):

```
0D 00 00 00  01  04 00 00 00  74 65 73 74
|            |   |            |
|            |   +-- strlen   +-- "test"
|            +-- msg id
+-- total length = 13
```

#### 0x02 -- PYS_ACK_AUTHENTICATE (Server -> Client)

Authentication result.

```
Header (5 bytes)
+--------+--------+--------+--------+--------+
| length (uint32, little-endian)    | 0x02   |
+--------+--------+--------+--------+--------+

Payload
+--------+--------+--------+--------+
| result (uint32)                   |
+--------+--------+--------+--------+
```

| Result | Meaning |
|--------|---------|
| `0x00000000` | Success -- state transitions to `PY_CLIENT_OK` |
| `0x00000001` | Failure -- wrong password, connection stays in `PY_CLIENT_AUTH_WAIT` |

#### 0x03 -- PYS_REQ_EVALUATE (Client -> Server)

Evaluate a Python expression and return the result. Uses `bp::eval()` -- the expression must be a single Python expression (not a statement).

```
Header (5 bytes)
+--------+--------+--------+--------+--------+
| length (uint32, little-endian)    | 0x03   |
+--------+--------+--------+--------+--------+

Payload
+--------+--------+--------+--------+--------+--- - -+
| expression_length (uint32)        | expression bytes |
+--------+--------+--------+--------+--------+--- - -+
```

**Example** (expression = `"2+2"`):

```
0C 00 00 00  03  03 00 00 00  32 2B 32
```

#### 0x04 -- PYS_ACK_EVALUATE (Server -> Client)

Evaluation result. The response format depends on the result type.

```
Header (5 bytes)
+--------+--------+--------+--------+--------+
| length (uint32, little-endian)    | 0x04   |
+--------+--------+--------+--------+--------+

Payload
+--------+--- - - - - - - - - ---+
| type   | type-specific data    |
+--------+--- - - - - - - - - ---+
```

**Response types** (`py_exec_response`):

| Value | Name | Additional Payload |
|-------|------|-------------------|
| `0x00` | `PY_EXEC_NONE` | (none) -- result was `None` or unserializable type |
| `0x01` | `PY_EXEC_EXCEPTION` | `uint32` string length + error string bytes |
| `0x02` | `PY_EXEC_INTEGER` | `int32` value (4 bytes, little-endian) |
| `0x03` | `PY_EXEC_FLOAT` | `float` value (4 bytes, IEEE 754 single) |
| `0x04` | `PY_EXEC_STRING` | `uint32` string length + string bytes |

**Serialization rules:**
- `bool` values are converted to `int32` (0 or 1) and sent as `PY_EXEC_INTEGER`
- `int` values are sent as `PY_EXEC_INTEGER` (`int32`)
- `float` values are sent as `PY_EXEC_FLOAT` (single-precision)
- `str` values are sent as `PY_EXEC_STRING`
- `NoneType` is sent as `PY_EXEC_NONE`
- Any other type logs a warning and is sent as `PY_EXEC_NONE`

**Example** -- integer result `4` from `"2+2"`:

```
0A 00 00 00  04  02  04 00 00 00
|            |   |   |
|            |   |   +-- int32 value = 4
|            |   +-- PY_EXEC_INTEGER
|            +-- msg id
+-- total length = 10
```

**Example** -- exception:

```
xx xx xx xx  04  01  1A 00 00 00  50 79 74 68 6F 6E ...
|            |   |   |            |
|            |   |   +-- strlen   +-- "Python syntax/runtime error"
|            |   +-- PY_EXEC_EXCEPTION
|            +-- msg id
+-- total length
```

#### 0x05 -- PYS_REQ_EXECUTE (Client -> Server)

Execute a Python statement. Uses `bp::exec()` -- supports full statements including imports, assignments, loops, etc. No return value.

```
Header (5 bytes)
+--------+--------+--------+--------+--------+
| length (uint32, little-endian)    | 0x05   |
+--------+--------+--------+--------+--------+

Payload
+--------+--------+--------+--------+--------+--- - -+
| statement_length (uint32)         | statement bytes  |
+--------+--------+--------+--------+--------+--- - -+
```

#### 0x06 -- PYS_ACK_EXECUTE (Server -> Client)

Execution result. Same format as `PYS_ACK_EVALUATE` but only two possible response types:

```
Header (5 bytes)
+--------+--------+--------+--------+--------+
| length (uint32, little-endian)    | 0x06   |
+--------+--------+--------+--------+--------+

Payload (success)
+--------+
| 0x00   |   PY_EXEC_NONE -- success
+--------+

Payload (exception)
+--------+--------+--------+--------+--------+--- - -+
| 0x01   | error_length (uint32)    | error bytes      |
+--------+--------+--------+--------+--------+--- - -+
```

### Eval vs Execute

The distinction mirrors Python's `eval()` vs `exec()`:

| | Evaluate (`0x03`) | Execute (`0x05`) |
|---|---|---|
| Python function | `bp::eval()` | `bp::exec()` |
| Input | Single expression | Statement(s) |
| Returns result | Yes (typed) | No (always `PY_EXEC_NONE` on success) |
| Assignments | No | Yes |
| Imports | No | Yes |
| Loops/conditionals | No | Yes |
| Multi-line | No | Yes (use `\n` in the string) |

**Evaluate examples:**
```python
"2 + 2"                          # -> PY_EXEC_INTEGER(4)
"Atrea.getGameTime()"            # -> PY_EXEC_INTEGER(...)
"str(Atrea.findAllEntities())"   # -> PY_EXEC_STRING(...)
"len(Atrea.findAllEntities())"   # -> PY_EXEC_INTEGER(...)
"True"                           # -> PY_EXEC_INTEGER(1)
"None"                           # -> PY_EXEC_NONE
```

**Execute examples:**
```python
"import cell"                                      # -> PY_EXEC_NONE (success)
"x = Atrea.getGameTime()"                          # -> PY_EXEC_NONE (success)
"for e in Atrea.findAllEntities(): print(e)"       # -> PY_EXEC_NONE (success)
"Atrea.reloadClasses()"                            # -> PY_EXEC_NONE (success)
"x = 1/0"                                          # -> PY_EXEC_EXCEPTION("Python syntax/runtime error")
```

### Reference Client

A minimal Python client for the remote TCP console:

```python
#!/usr/bin/env python3
"""Cimmeria Python Console client. Requires Python 3.10+."""

import socket
import struct

class CimmeriaConsole:
    def __init__(self, host: str, port: int, password: str):
        self.sock = socket.create_connection((host, port))
        self._authenticate(password)

    def _send(self, msg_id: int, payload: bytes):
        length = 5 + len(payload)
        self.sock.sendall(struct.pack('<IB', length, msg_id) + payload)

    def _recv_msg(self) -> tuple[int, bytes]:
        header = self._recv_exact(5)
        length, msg_id = struct.unpack('<IB', header)
        payload = self._recv_exact(length - 5) if length > 5 else b''
        return msg_id, payload

    def _recv_exact(self, n: int) -> bytes:
        data = b''
        while len(data) < n:
            chunk = self.sock.recv(n - len(data))
            if not chunk:
                raise ConnectionError("Connection closed")
            data += chunk
        return data

    def _pack_string(self, s: str) -> bytes:
        encoded = s.encode('utf-8')
        return struct.pack('<I', len(encoded)) + encoded

    def _authenticate(self, password: str):
        self._send(0x01, self._pack_string(password))
        msg_id, payload = self._recv_msg()
        assert msg_id == 0x02, f"Unexpected msg id: {msg_id:#x}"
        result = struct.unpack('<I', payload)[0]
        if result != 0:
            raise RuntimeError("Authentication failed")

    def evaluate(self, expression: str) -> object:
        self._send(0x03, self._pack_string(expression))
        msg_id, payload = self._recv_msg()
        assert msg_id == 0x04, f"Unexpected msg id: {msg_id:#x}"
        resp_type = payload[0]
        if resp_type == 0:    return None
        elif resp_type == 1:  raise RuntimeError(self._read_string(payload[1:]))
        elif resp_type == 2:  return struct.unpack('<i', payload[1:5])[0]
        elif resp_type == 3:  return struct.unpack('<f', payload[1:5])[0]
        elif resp_type == 4:  return self._read_string(payload[1:])
        else:                 raise ValueError(f"Unknown response type: {resp_type}")

    def execute(self, statement: str):
        self._send(0x05, self._pack_string(statement))
        msg_id, payload = self._recv_msg()
        assert msg_id == 0x06, f"Unexpected msg id: {msg_id:#x}"
        resp_type = payload[0]
        if resp_type == 1:
            raise RuntimeError(self._read_string(payload[1:]))

    def _read_string(self, data: bytes) -> str:
        length = struct.unpack('<I', data[:4])[0]
        return data[4:4 + length].decode('utf-8')

    def close(self):
        self.sock.close()


if __name__ == '__main__':
    import sys
    host = sys.argv[1] if len(sys.argv) > 1 else 'localhost'
    port = int(sys.argv[2]) if len(sys.argv) > 2 else 8989
    password = sys.argv[3] if len(sys.argv) > 3 else 'test'

    console = CimmeriaConsole(host, port, password)
    print(f"Connected to {host}:{port}")
    try:
        while True:
            cmd = input(">>> ")
            if cmd in ('exit', 'quit'):
                break
            try:
                # Try evaluate first (returns a value)
                result = console.evaluate(cmd)
                if result is not None:
                    print(result)
            except RuntimeError:
                # If eval fails, try exec (for statements)
                try:
                    console.execute(cmd)
                except RuntimeError as e:
                    print(f"Error: {e}")
    except (EOFError, KeyboardInterrupt):
        print()
    finally:
        console.close()
```

**Usage:**
```bash
python console_client.py localhost 8989 my_password
```

---

## Python Namespace

Both consoles execute in the `__main__` module namespace. The `Atrea` module is registered during service initialization via `PyRegisterModule()` (`src/entity/python.cpp`), which provides:

### Common (Both Services)

| Symbol | Type | Description |
|--------|------|-------------|
| `Atrea.Vector3(x, y, z)` | class | 3D vector with `x`, `y`, `z` fields and `length()`, `normalize()` methods |
| `Atrea.log(level, category, msg)` | function | Write to the server log |

### BaseApp Functions (`Atrea.*`)

Registered in `src/baseapp/base_service.cpp:205-221`.

| Function | Signature | Description |
|----------|-----------|-------------|
| `dbQuery` | `dbQuery(sql)` | Execute SQL query, return list of row dicts |
| `dbPerform` | `dbPerform(sql)` | Execute SQL statement (no return) |
| `sendResource` | `sendResource(entity, categoryId, elementId)` | Send cooked data resource to a client entity |
| `createBaseEntity` | `createBaseEntity(className)` | Create a new base entity by class name |
| `createBaseEntityFromDb` | `createBaseEntityFromDb(className, dbid)` | Create base entity from database record |
| `createCellEntity` | `createCellEntity(entityId, spaceId, dbid, x, y, z, yaw, pitch, roll, worldName)` | Create a cell entity at position |
| `findAllEntities` | `findAllEntities()` | Return all base entities |
| `switchEntity` | `switchEntity(entityId, newEntityId)` | Switch controller between entities |
| `registerMinigameSession` | `registerMinigameSession(...)` | Register a minigame session |
| `cancelMinigameSession` | `cancelMinigameSession(entityId)` | Cancel a minigame session |
| `getGameTime` | `getGameTime()` | Get current game time |
| `addTimer` | `addTimer(completeTime, callback)` | Register a timer |
| `cancelTimer` | `cancelTimer(timerId)` | Cancel a timer |
| `reloadClasses` | `reloadClasses()` | Hot-reload Python entity classes |

### CellApp Functions (`Atrea.*`)

Registered in `src/cellapp/cell_service.cpp:116-133`.

| Function | Signature | Description |
|----------|-----------|-------------|
| `dbQuery` | `dbQuery(sql)` | Execute SQL query, return list of row dicts |
| `dbPerform` | `dbPerform(sql)` | Execute SQL statement (no return) |
| `createCellPlayer` | `createCellPlayer(entityId)` | Create a cell player entity |
| `createCellEntity` | `createCellEntity(className, dbid)` | Create a cell entity |
| `destroyCellEntity` | `destroyCellEntity(entityId)` | Destroy a cell entity |
| `createSpace` | `createSpace(worldName, creatorId)` | Create a new space |
| `destroySpace` | `destroySpace(spaceId)` | Destroy a space |
| `findSpace` | `findSpace(worldName)` | Find space by world name |
| `findEntity` | `findEntity(entityId)` | Find entity by ID |
| `findEntityOnSpace` | `findEntityOnSpace(entityId, spaceId)` | Find entity on specific space |
| `findEntitiesByDbid` | `findEntitiesByDbid(databaseId, spaceId)` | Find entities by database ID |
| `findEntities` | `findEntities(spaceId)` | Find all entities on a space |
| `getGameTime` | `getGameTime()` | Get current game time |
| `addTimer` | `addTimer(completeTime, callback)` | Register a timer |
| `cancelTimer` | `cancelTimer(timerId)` | Cancel a timer |
| `findPath` | `findPath(spaceId, start, destination)` | Find navmesh path |
| `findDetailedPath` | `findDetailedPath(spaceId, start, dest, ...)` | Find detailed navmesh path with extents |
| `reloadClasses` | `reloadClasses()` | Hot-reload Python entity classes |

### Practical Examples

**Query game time:**
```python
>>> Atrea.getGameTime()
```

**Find all entities on a space (CellApp):**
```python
>>> entities = Atrea.findEntities(1)
>>> for e in entities: print(e.id, e.__class__.__name__)
```

**Run a database query:**
```python
>>> rows = Atrea.dbQuery("SELECT character_id, name FROM sgw_player LIMIT 5")
>>> for r in rows: print(r)
```

**Hot-reload Python classes after editing scripts:**
```python
>>> Atrea.reloadClasses()
```

**Modify entity state (CellApp):**
```python
>>> entity = Atrea.findEntity(12345)
>>> entity.health = 100
>>> entity.position = Atrea.Vector3(100.0, 0.0, 200.0)
```

**Execute a database update:**
```python
>>> Atrea.dbPerform("UPDATE sgw_player SET access_level = 1 WHERE name = 'TestPlayer'")
```

---

## GM Commands vs Python Console

These are two completely separate systems:

### GM Commands (In-Game Chat)

- **Entry point:** Player types `/command args` in the game chat window
- **Parsed by:** `SGWPlayer.onConsoleCommand()` -> `ConsoleCommands.Command.exec()`
- **Access control:** Each command has a required `accessLevel` (0 = all players, 1 = GM)
- **Target system:** Many commands operate on the player's targeted entity
- **Source:** `python/cell/ConsoleCommands.py` + `python/cell/commands/*.py`

### Python Console (Server-Side REPL)

- **Entry point:** Stdin prompt or TCP client
- **Parsed by:** Raw `bp::exec()` / `bp::eval()` -- no command framework
- **Access control:** Anyone with the TCP password or server process access
- **Target system:** None -- direct Python API access

### Invoking GM Command Logic from the Console

GM commands are just Python functions. You can call them directly from the console if you have access to a player entity:

```python
# CellApp console
>>> import cell.ConsoleCommands as CC
>>> player = Atrea.findEntity(12345)   # Must be an SGWPlayer
>>> target = player.space.findEntity(player.targetId)
>>> cell.commands.Player.giveCash(player, target, amount=1000)
```

Or use the Command dispatcher directly:

```python
>>> CC.Command.exec(player, "givecash 1000")
```

### Complete GM Command Table

All 116 commands registered in `python/cell/ConsoleCommands.py`. All commands with access level 1 require GM privileges.

#### General (3 commands)

| Command | Access | Target | Description |
|---------|--------|--------|-------------|
| `help` | 0 | -- | Shows help about console commands |
| `loglevel` | 1 | -- | Updates log level for one or more event category |
| `logclient` | 1 | -- | Enable/disable sending of server log messages to the client |

#### Player Commands (24 commands)

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

#### Entity Commands (27 commands)

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

#### Stats Commands (7 commands)

| Command | Access | Target | Description |
|---------|--------|--------|-------------|
| `stats` | 1 | SGWBeing | Show all stats |
| `primarystats` | 1 | SGWBeing | Show primary stats |
| `speedstats` | 1 | SGWBeing | Show speed stats |
| `armorstats` | 1 | SGWBeing | Show armor stats |
| `qrstats` | 1 | SGWBeing | Show QR stats |
| `absorbstats` | 1 | SGWBeing | Show absorb stats |
| `stealthstats` | 1 | SGWBeing | Show stealth stats |

#### Mob Commands (3 commands)

| Command | Access | Target | Description |
|---------|--------|--------|-------------|
| `aggression` | 1 | SGWMob | Set aggression level |
| `threaten` | 1 | SGWMob | Generate threat |
| `combatinfo` | 1 | SGWMob | Show combat info |

#### Mission Commands (14 commands)

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

#### Crafting Commands (5 commands)

| Command | Access | Target | Description |
|---------|--------|--------|-------------|
| `appliedscience` | 1 | SGWPlayer | Give applied science points |
| `racialparadigm` | 1 | SGWPlayer | Set racial paradigm level |
| `learndiscipline` | 1 | SGWPlayer | Learn a crafting discipline |
| `forgetdiscipline` | 1 | SGWPlayer | Forget a crafting discipline |
| `allcraft` | 1 | SGWPlayer | Debug: unlock all crafting |

#### Resource Commands (11 commands)

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

#### Network/Debug Commands (11 commands)

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

#### Debug Commands (11 commands)

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

---

## Security Considerations

The Python console provides **unrestricted access** to the server runtime. Anyone with console access can:

- Execute arbitrary Python code in the server process
- Query and modify the database directly via `dbQuery`/`dbPerform`
- Create, modify, and destroy any entity
- Access all player data
- Crash the server (intentionally or accidentally)

**Mitigations:**

1. **Remote console is disabled by default** -- `py_console_password` is empty in the shipped config.
2. **Password-gated** -- TCP clients must authenticate before executing code.
3. **Plain-text password** -- The password is sent unencrypted over TCP. **Never expose the console port to the internet.** Use the console only on `localhost` or over a trusted LAN/VPN.
4. **No TLS** -- The entire protocol is unencrypted. There is no option for TLS wrapping.
5. **No rate limiting** -- No protection against brute-force password guessing.
6. **No audit logging** -- Executed commands are logged at TRACE level but there is no dedicated audit trail.

**Recommendation:** Keep the console port firewalled to `localhost` or trusted management IPs only. For production deployments, leave `py_console_password` empty to disable the TCP console entirely, and use the local stdin console instead.

---

## Source File Reference

| File | Description |
|------|-------------|
| `src/entity/py_client.hpp` | Protocol enums, `py_client` class declaration |
| `src/entity/py_client.cpp` | Auth, eval, exec message handlers |
| `src/common/console.hpp` | `ConsoleThread()` declaration |
| `src/common/console.cpp` | Stdin REPL implementation |
| `src/mercury/unified_connection.hpp` | `Writer`, `Reader`, `MessageHeader` struct, string serialization |
| `src/mercury/unified_connection.cpp` | `beginMessage()`, `endMessage()`, send/receive loop |
| `src/mercury/tcp_server.hpp` | `TcpServer<T>` template -- Boost.Asio TCP acceptor |
| `src/entity/python.cpp` | `PyRegisterModule()` -- Atrea module init, Vector3, log() |
| `src/entity/pyutil.hpp` | `PyGilGuard` RAII wrapper, `PyUtil_ShowErr()` |
| `src/baseapp/base_service.cpp` | BaseApp Python init + Atrea function registration |
| `src/cellapp/cell_service.cpp` | CellApp Python init + Atrea function registration |
| `python/cell/ConsoleCommands.py` | GM command framework + all 86 command registrations |
| `python/cell/commands/*.py` | GM command implementations (Crafting, Entity, Misc, Mission, Net, Player, Resource) |
| `config/BaseService.config` | BaseApp console config (port 8989) |
| `config/CellService.config` | CellApp console config (port 8990) |
