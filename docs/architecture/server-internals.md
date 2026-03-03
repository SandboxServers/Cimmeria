# Server Internals

Deep technical reference for the Cimmeria C++ server. Assumes C++ proficiency and familiarity with the [service architecture](service-architecture.md).


## Source Tree Map

```
src/
├─ authentication/              # AuthenticationServer
│  ├─ service_main.hpp/cpp      #   AuthService class, login + shard management
│  ├─ logon_connection.hpp/cpp  #   Client login connections (HTTP/SOAP)
│  ├─ frontend_connection.hpp/cpp  # Shard registration connections
│  └─ shard_client.hpp/cpp      #   BaseApp → Auth TCP client
│
├─ baseapp/                     # BaseApp service
│  ├─ base_service.hpp/cpp      #   BaseService class, main event loop
│  ├─ cell_manager.hpp          #   Space/cell registry
│  ├─ entity/
│  │  ├─ base_entity.hpp/cpp    #   BaseEntity (persistent player entity)
│  │  └─ base_entity_manager.hpp #  Entity pool with space tracking
│  └─ mercury/
│     ├─ cell_handler.hpp/cpp   #   CellApp TCP connection handler
│     └─ sgw/
│        ├─ client_handler.hpp/cpp  # Client UDP message dispatch
│        └─ resource.hpp/cpp    #   Cooked data (PAK) resource manager
│
├─ cellapp/                     # CellApp service
│  ├─ cell_service.hpp/cpp      #   CellService class
│  ├─ base_client.hpp/cpp       #   BaseApp TCP client
│  ├─ space.hpp/cpp             #   Space management, SpaceManager singleton
│  └─ entity/
│     ├─ cell_entity.hpp/cpp    #   CellEntity (spatial entity)
│     ├─ movement.hpp/cpp       #   Movement controllers (Player, Waypoint, Debug)
│     └─ navigation.hpp/cpp     #   Recast/Detour pathfinding
│
├─ common/                      # Shared infrastructure
│  ├─ service.hpp/cpp           #   Service base class, config, event loop
│  ├─ database.hpp/cpp          #   SOCI database wrapper, worker thread
│  ├─ console.hpp/cpp           #   Python console server
│  └─ vec3.hpp                  #   3D vector type
│
├─ entity/                      # Entity system (shared by Base + Cell)
│  ├─ entity.hpp/cpp            #   Entity base class, EntityManager<T>
│  ├─ defs.hpp/cpp              #   PyClassDef, PyTypeDb (entity type registry)
│  ├─ mailbox.hpp               #   Mailbox RPC framework
│  ├─ method.hpp/cpp            #   RPC method definitions
│  ├─ python.hpp/cpp            #   Python integration utilities
│  └─ world_grid.hpp            #   Spatial partitioning template
│
├─ log/
│  └─ logger.hpp/cpp            #   Async circular buffer logger
│
├─ mercury/                     # Networking layer
│  ├─ nub.hpp/cpp               #   UDP socket manager
│  ├─ channel.hpp/cpp           #   Reliable UDP channel (ARQ)
│  ├─ packet.hpp/cpp            #   Packet format and constants
│  ├─ bundle.hpp/cpp            #   Message aggregation
│  ├─ encryption_filter.hpp/cpp #   AES-256-CBC + HMAC-MD5
│  ├─ unified_connection.hpp/cpp #  TCP framing for inter-service
│  ├─ tcp_server.hpp            #   Generic TCP acceptor template
│  ├─ message.hpp/cpp           #   Message format definitions
│  ├─ memory_stream.hpp         #   Serialization buffer
│  └─ timer.hpp                 #   Timer manager
│
├─ server/                      # Server-specific utilities
│  └─ NavBuilder/               #   Offline navigation mesh generator
│
└─ util/
   ├─ singleton.hpp             #   Singleton template
   └─ crashdump.hpp/cpp         #   Windows crash reporting (dbghelp)
```


## Service Lifecycle

All services extend the `Service` base class (`src/common/service.hpp`).

### Startup Sequence

1. **Config loading** — Parse XML config file via `getConfigParameter<T>()`
2. **Database init** — Create SOCI session, start worker thread
3. **Python init** — `Py_Initialize()`, register `Atrea` module, load entity classes
4. **Network init** — Create Nub (UDP), TcpServer (TCP), bind ports
5. **Inter-service connect** — Connect to upstream services (Auth, BaseApp)
6. **Space loading** — Load zone definitions, create initial spaces
7. **Event loop** — `io_service::run()` enters the Boost.Asio reactor

### Tick System

The main tick fires every 10ms via a Boost.Asio deadline timer:

```
Service::TickInterval = 10ms     # Base timer resolution
tick_rate = 100ms                # Game tick (10 Hz, configurable)
nub_tickrate = 25ms              # Channel flush interval (configurable)
Nub::tickInterval_ = 50ms       # Channel management tick
```

### Shutdown

Triggered via `atexit`/`exitHandler`. Stops the Asio event loop, flushes pending database operations, and shuts down Python.

### Key Members

```cpp
class Service {
    boost::asio::io_service service_;     // Event loop
    ConfigOptions configOptions_;          // Parsed config
    Database dbMgr_;                       // Database pool
    TimerMgr<uint64_t> timers_;           // Application timers
    std::mt19937 rng_;                     // Random number generator
    static const int TickInterval = 10;    // ms
};
```

### Endpoint Types

```cpp
enum EndpointType {
    BaseMailbox = 0,
    BaseExposedMailbox = 1,
    CellMailbox = 2,
    CellExposedMailbox = 3,
    ClientMailbox = 4
};
```


## Mercury Networking Layer

Mercury is the reliable UDP protocol used for client-server communication. Inter-service communication uses TCP.

### Nub — UDP Socket Manager

`src/mercury/nub.hpp/cpp`

The Nub owns a single UDP socket and manages all client channels.

**Constants:**

| Constant | Value | Purpose |
|----------|-------|---------|
| `TxSendQueueMaxSize` | 1024 | Max pending outgoing packets |
| `MaxPacketLength` | 1472 | UDP MTU minus IP/UDP headers |
| `tickInterval_` | 50ms | Channel management tick rate |

**Key members:**

```cpp
class Nub {
    udp::socket socket_;                              // Boost.Asio UDP socket
    std::map<Address, BaseChannel::Ptr> channels_;    // Client connections by endpoint
    TimerMgr<uint64_t> timers_;                       // Retransmission/keepalive timers
    boost::circular_buffer<PendingPacket> sendQueue_;  // Outgoing packet queue
    deadline_timer m_timer;                            // Async tick timer
};
```

**Data flow (inbound):**

```
UDP packet → Nub::frameHandler() → channel lookup → BaseChannel::handlePacket()
  → EncryptionFilter::receiveMessage() → BundleUnpacker → message dispatch
```

**Data flow (outbound):**

```
App → Bundle::beginMessage()/endMessage() → Bundle::flush() → BaseChannel
  → EncryptionFilter::sendMessage() → Nub::queuePacket() → UDP send
```


### BaseChannel — Reliable UDP

`src/mercury/channel.hpp/cpp`

Implements BigWorld's reliable UDP protocol with sliding windows and automatic retransmission.

**Constants:**

| Constant | Value | Purpose |
|----------|-------|---------|
| `RxWindowMaxSize` | 64 | Receive sliding window capacity |
| `TxWindowMaxSize` | 45 | Send sliding window capacity |
| `TxQueueMaxSize` | 1024 | Overflow queue for full send window |
| `TxMaxRetries` | 20 | Max retransmissions before disconnect |
| `AckTimeout` | 700ms | Retransmission timeout |
| `InactivityKeepaliveInterval` | 1000ms | Keepalive message rate |
| `InactivityTimeout` | 10000ms | Disconnect after no packets |
| `TxBurstInterval` | 200ms | Min time between send bursts |

**Key members:**

```cpp
class BaseChannel {
    circular_buffer<RxEntry> receiveWindow_;     // In-order reliable packet buffer
    circular_buffer<TxEntry> sendWindow_;         // Unacked reliable packets + retry counters
    deque<Packet::Ptr> sendQueue_;                // Overflow when send window full
    uint32_t nextSendSequenceId_;                 // Next reliable sequence to send
    uint32_t nextReceiveSequenceId_;              // Next reliable sequence expected
    Bundle::Ptr bundle_;                          // Active reliable message bundle
    Bundle::Ptr unreliableBundle_;                // Active unreliable message bundle
    EncryptionFilter* filter_;                    // AES encryption (null = unencrypted)
};
```

**Reliable delivery:**
- Each reliable packet gets a sequence ID from the sender
- Receiver ACKs received sequences and NACKs gaps
- Sender retransmits unacked packets after 700ms, up to 20 times
- Out-of-order packets are buffered in the receive window and delivered in order
- Channel is condemned (gracefully closed) after `TxMaxRetries` exhausted


### Packet — Wire Format

`src/mercury/packet.hpp`

**Flags byte:**

| Flag | Value | Meaning |
|------|-------|---------|
| `FLAG_HAS_REQUESTS` | `0x01` | Contains request messages |
| `FLAG_HAS_ACKS` | `0x04` | Contains ACK list |
| `FLAG_ON_CHANNEL` | `0x08` | Sent on a channel (vs. off-channel) |
| `FLAG_RELIABLE` | `0x10` | Requires ACK |
| `FLAG_FRAGMENTED` | `0x20` | Part of multi-packet bundle |

**Size constants:**

| Constant | Value | Derivation |
|----------|-------|------------|
| `MAX_RECEIVE_LENGTH` | 1456 | 1472 - 16 (HMAC) |
| `MAX_LENGTH` | 1392 | 1456 - 32, rounded to AES block |
| `MAX_BODY_LENGTH` | 1348 | 1392 - 4 (seq) - 12 (fragments) - 1 (flags) |
| `MaxFragmentsPerBundle` | 64 | Max packets per fragmented bundle |

**Packet layout (after decryption):**

```
[4 bytes] Reliable sequence ID
[1 byte]  Flags
[N bytes] Message payload(s)
[optional] Fragment header (if FLAG_FRAGMENTED):
  [1 byte] Fragment index
  [1 byte] Fragment count
[optional] ACK list (if FLAG_HAS_ACKS):
  [4 bytes] First ACK sequence
  [variable] ACK bitmap
```


### Bundle — Message Aggregation

`src/mercury/bundle.hpp/cpp`

Groups multiple messages into fewer UDP packets for efficiency.

**Message ID encoding:**

| Range | Encoding | Description |
|-------|----------|-------------|
| Base RPC 0x00–0x3C | Compressed to 0x80–0xBC | 1-byte message ID |
| Cell RPC 0x00–0x3C | Compressed to 0xC0–0xFC | 1-byte message ID |
| Base RPC 0x3D–0x13C | 0xBD + extended byte | 2-byte message ID |
| Cell RPC 0x3D–0x13C | 0xFD + extended byte | 2-byte message ID |

**Key methods:**

```cpp
class Bundle {
    void beginMessage(uint8_t messageId);   // Start framing a message
    void endMessage();                       // Finalize message length
    void finalize();                         // Close all packets, set fragment indices
    void flush();                            // Push to channel/Nub for transmission
};
```


### EncryptionFilter — AES-256-CBC

`src/mercury/encryption_filter.hpp/cpp`

Encrypts/decrypts packets using AES-256-CBC with HMAC-MD5 authentication.

**Algorithm:**

1. **Send:** AES-CBC encrypt payload → PKCS#7 pad final block → encrypt padded block → append 16-byte HMAC-MD5
2. **Receive:** Verify 16-byte HMAC-MD5 → AES-CBC decrypt → strip PKCS#7 padding

**Overhead:** 16 bytes HMAC + 1–16 bytes padding per packet.

**Key exchange:** 256-bit shared key established during login handshake via the AuthenticationServer.


### UnifiedConnection — TCP Framing

`src/mercury/unified_connection.hpp/cpp`

TCP-based message transport for inter-service communication (Auth↔Base, Base↔Cell).

**Message format:**

```
[4 bytes] uint32_t length (payload only, excludes this header)
[1 byte]  uint8_t messageId
[N bytes] payload
```

**Key members:**

```cpp
class UnifiedConnection {
    tcp::socket socket_;                    // Boost.Asio TCP socket
    CircularBuffer<uint8_t> sending_;       // 1 MB send buffer
    CircularBuffer<uint8_t> received_;      // 1 MB receive buffer

    virtual void onConnected();             // Connection established
    virtual void onDisconnected(error_code); // Connection lost
    virtual void onMessageReceived(uint8_t msgId, const uint8_t* data, size_t len);
};
```


### TcpServer — Generic Acceptor

`src/mercury/tcp_server.hpp`

Template-based async TCP acceptor for service connections:

```cpp
template <typename ConnectionType>
class TcpServer {
    tcp::acceptor acceptor_;
    std::vector<typename ConnectionType::Ptr> connections_;
    uint32_t nextConnectionId_;
    // Factory creates ConnectionType instances on accept
};
```

Usage: `TcpServer<CellAppConnection>`, `TcpServer<LogonConnection>`, `TcpServer<FrontendConnection>`.


## Entity System

### Entity Base Class

`src/entity/entity.hpp/cpp`

Abstract base for all game entities:

```cpp
class Entity {
    uint32_t entityId_;       // Runtime entity ID (1-based)
    int32_t databaseId_;      // Persistent database ID
    PyClassDef* class_;       // Entity type definition
    bp::object object_;       // Python instance

    void setup(uint32_t id, PyClassDef* cls);
    void rpc(const std::string& method, ...);  // Call Python method
    void persist();
    void load();
    void destroy();
};
```

### EntityManager — Generic Pool

`src/entity/entity.hpp`

Template class for managing entity instances with ID reuse:

```cpp
template <typename T>
class EntityManager {
    std::vector<typename T::Ptr> entities_;  // ID-1 = array index
    std::vector<uint32_t> freeIds_;          // Recycled entity IDs

    typename T::Ptr create();     // Allocate entity with new/recycled ID
    void destroyed(uint32_t id);  // Reclaim ID for reuse
    typename T::Ptr get(uint32_t id);  // Lookup by ID
};
```

### BaseEntityManager — Space-Aware Pool

`src/baseapp/entity/base_entity_manager.hpp`

Extends EntityManager with space tracking:

```cpp
class BaseEntityManager : public EntityManager<BaseEntity> {
    std::multimap<uint32_t, BaseEntity::Ptr> spaceMap_;  // Entities by space

    void addToSpace(uint32_t spaceId, BaseEntity::Ptr entity);
    void removeFromSpace(uint32_t spaceId, BaseEntity::Ptr entity);
    void tick(uint32_t spaceId);  // Tick all entities in a space
};
```


### PyClassDef — Entity Type Definition

`src/entity/defs.hpp`

Parsed from XML `.def` files. Represents an entity type:

```cpp
class PyClassDef {
    std::string name_;                     // Entity class name
    uint8_t index_;                        // Class ID (0–255)
    bool isInterface_;                     // Is abstract interface
    bool persistent_;                      // Can be saved to DB
    std::vector<MethodDef> baseMethods_;   // Base RPC methods
    std::vector<MethodDef> cellMethods_;   // Cell RPC methods
    std::vector<MethodDef> clientMethods_; // Client RPC methods
    bp::object class_;                     // Python class object

    static PyClassDef* fromDefinitionFile(const std::string& path);
    MethodDef* findMethod(EndpointType ep, uint8_t id);
    template <typename T> T* create();     // Instantiate Python class
};
```

### PyTypeDb — Entity Registry

`src/entity/defs.hpp`

Singleton managing all entity type definitions:

```cpp
class PyTypeDb {
    std::multimap<std::string, PyClassDef*> classes_;  // By-name lookup
    std::map<std::string, PyTypeInfo*> types_;         // Custom type registry

    void loadClass(const std::string& name);
    PyClassDef* findClass(const std::string& name);
    PyClassDef* findClass(uint8_t index);
    void registerType(const std::string& name, PyTypeInfo* info);
    template <typename T> void setupClasses();  // Expose all to Python
};
```


### Mailbox — RPC Framework

`src/entity/mailbox.hpp`

Python-exposed mailbox for entity RPC calls:

```cpp
class Mailbox {
    MailboxClass* class_;    // Mailbox type (Base, Cell, Client)
    uint32_t entityId_;      // Target entity

    bp::object getattr(const std::string& name);  // Python __getattr__
};

class MailboxClass {
    PyClassDef* class_;
    EndpointType mailboxType_;   // BaseMailbox, CellMailbox, ClientMailbox
    std::vector<MethodDef> methods_;

    virtual void doRpc(uint32_t entityId, uint8_t methodId, ...);
};
```

**Specializations:**
- `ClientMailboxClass` — Delivers RPC to game client via UDP channel
- `CellMailboxClass` — Delivers RPC to CellApp via TCP connection, supports distribution flags

**Distribution flags (CellMailbox):**

| Flag | Effect |
|------|--------|
| `DISTRIBUTE_CLIENT` | Single target client |
| `DISTRIBUTE_SPACE` | All clients in space |
| `DISTRIBUTE_LOD` | By LOD distance |
| `DISTRIBUTE_RECIPIENT` | Specific recipient entity |


## BaseApp

### BaseService

`src/baseapp/base_service.hpp/cpp`

Main BaseApp service:

```cpp
class BaseService : public Service {
    Nub* clientNub_;                              // UDP for client connections
    TcpServer<CellAppConnection>* cellServer_;     // TCP listener for CellApps
    TcpServer<MinigameConnection>* minigameServer_; // TCP for minigame services
    ShardClient::Ptr authClient_;                  // TCP to AuthenticationServer
    SGW::ResourceManager resources_;               // Cooked data manager
    MinigameRequestManager minigameManager_;       // Minigame request router
};
```

### BaseEntity

`src/baseapp/entity/base_entity.hpp/cpp`

Persistent player entity on BaseApp:

```cpp
class BaseEntity : public Entity {
    Mailbox::Ptr client_;           // RPC mailbox to game client
    Mailbox::Ptr cell_;             // RPC mailbox to cell entity
    BaseChannel::Ptr clientChannel_; // UDP channel to client
    Controller* controller_;        // Client controller/handler
    uint32_t spaceId_;              // Current space
    Vec3 lastPosition_;             // Cached position

    // Cell state snapshots for persistence
    std::vector<uint8_t> backup_;
    std::vector<uint8_t> pythonBackup_;

    void setupClient(BaseChannel::Ptr channel);
    void attachedToController(Controller* ctrl);
    void cellEntityCreated();
    void createCellPlayer();
    void unpackBackup(const uint8_t* data, size_t len);
    void packBackup(std::vector<uint8_t>& out);
};
```

### ClientHandler — Client Message Dispatch

`src/baseapp/mercury/sgw/client_handler.hpp/cpp`

Routes client UDP messages to entity RPC calls:

```cpp
class ClientHandler : public Controller {
    BaseEntity::Ptr entity_;
    uint32_t accountId_;
    std::string accountName_;
    uint8_t accessLevel_;
    bool entitySystemEnabled_;

    void onBaseMessage(uint8_t msgId, const uint8_t* data, size_t len);
    void onEntityMessage(uint8_t msgId, ...);   // 0x80–0xBF range
    void onBaseEntityMessage(uint8_t msgId, ...); // 0xC0+ range

    // Visibility updates to client
    void createEntity(CellEntity& entity);
    void enterAoI(uint32_t entityId);
    void leaveAoI(uint32_t entityId);
    void moveEntity(uint32_t entityId, Vec3 pos, Vec3 vel);

    // Resource delivery
    void sendResource(uint32_t categoryId, uint32_t elementId);
};
```

### CellAppConnection — Inter-Service

`src/baseapp/mercury/cell_handler.hpp/cpp`

TCP connection between BaseApp and CellApp:

**Outbound messages (Base → Cell):**

| Method | Purpose |
|--------|---------|
| `sendCreateEntity()` | Request cell entity creation |
| `sendDestroyEntity()` | Request cell entity destruction |
| `sendConnectEntity()` | Attach client controller |
| `sendDisconnectEntity()` | Detach client controller |
| `sendEntityMove()` | Forward client position update |
| `sendRestoreEntity()` | Restore cell entity from backup |
| `sendInternalMessage()` | Internal RPC to cell |
| `sendExposedMessage()` | Client-exposed RPC to cell |
| `sendRequestEntityUpdate()` | Request position update |

**Inbound messages (Cell → Base):**

| Handler | Purpose |
|---------|---------|
| `onEntityCreateAck()` | Cell creation confirmed |
| `onEntityDeleteAck()` | Cell deletion confirmed |
| `onSpaceData()` | Space availability info |
| `onGameTick()` | Cell game tick clock |
| `onEntityMove()` | Cell position updates |
| `onEntityBackup()` | Cell state snapshot for persistence |
| `onBaseAppMessage()` | RPC from cell to base |
| `onClientMessage()` | RPC from cell to client (forwarded) |
| `onSwitchSpace()` | Entity move request |


## CellApp

### CellService

`src/cellapp/cell_service.hpp/cpp`

Main CellApp service:

```cpp
class CellService : public Service {
    uint32_t cellId_;                // Assigned cell ID (1-based)
    BaseAppClient* client_;          // TCP connection to BaseApp
    Nub* clientNub_;                 // UDP socket (not actively used)
    CellMessageWriter writer_;       // Message dispatcher

    void loadCellClasses();          // Load entity definitions
    void onSpaceEvent(uint32_t spaceId, const std::string& worldName);
};
```

### CellEntity — Spatial Entity

`src/cellapp/entity/cell_entity.hpp/cpp`

Entity with position, movement, and visibility:

```cpp
class CellEntity : public Entity {
    static const int PositionTicksOnForcedUpdate = 5;

    Vec3 position_;
    Vec3 rotation_;
    Vec3 velocity_;
    float maxSpeed_;
    uint8_t movementType_;
    bool isPlayer_;

    Mailbox::Ptr client_;       // RPC to controlling client
    Mailbox::Ptr witnesses_;    // RPC to all witness clients
    Mailbox::Ptr base_;         // RPC to base entity
    Space* space_;              // Containing space
    MovementController* controller_;  // Movement handler

    int forcedPositionTicks_;   // Lockout after forced position
    bool hasDynamicProperties_; // Different appearance per client
    bool cacheable_;            // Can cache on BaseApp

    void enterSpace(Space* space);
    void leaveSpace();
    void tick();
    void createOnClient(ClientHandler& client);
    void setController(MovementController* ctrl);
    void enteredAoI(CellEntity& other);
    void updatedPosition();
    void backup(std::vector<uint8_t>& out);
    void restore(const uint8_t* data, size_t len);
    void moveTo(uint32_t spaceId);  // Request space transfer
};
```


### Movement Controllers

`src/cellapp/entity/movement.hpp/cpp`

**TickResult enum:**

| Value | Meaning |
|-------|---------|
| `NoMovement` (0) | No position change |
| `LinearMovement` (1) | Predictable velocity-based movement |
| `NonlinearMovement` (2) | Direction or velocity change |
| `ForcedMovement` (3) | Server-forced position correction |

**PlayerController** — Processes client position updates:

```cpp
class PlayerController : public MovementController {
    CellEntity* entity_;
    uint32_t lastTick_;
    uint64_t lastUpdateTime_;
    TickResult tick() override;
};
```

**WaypointController** — Moves entity along navigation mesh paths:

```cpp
class WaypointController : public MovementController {
    std::vector<Waypoint> waypoints_;
    size_t nextWaypoint_;
    NavigationPath* path_;
    float currentDistance_;
    TickResult tick() override;
};
```

**DebugController** — Development-only manual movement controller.


### Navigation

`src/cellapp/entity/navigation.hpp/cpp`

Recast/Detour integration for pathfinding:

```cpp
class NavigationMesh {
    rcPolyMesh* mesh_;              // Recast polygon mesh
    rcPolyMeshDetail* detail_;      // Detail mesh
    dtNavMesh* navMesh_;            // Detour navigation mesh
    dtNavMeshQuery query_;          // Detour query object

    bool load(const std::string& path);
    NavigationPath* findPath(const Vec3& start, const Vec3& end);
    float findPathLength(const Vec3& start, const Vec3& end);
};

class NavigationPath {
    static const int MaxPathLength = 64;         // Max polygons in path
    static const int MaxSmoothingSteps = 256;    // Max smoothed waypoints

    void smooth();  // Apply string-pulling to path
};
```


### Space Management

`src/cellapp/space.hpp/cpp`

**Space** — Represents a playable zone/instance:

```cpp
class Space {
    uint32_t spaceId_;
    std::string worldName_;
    uint32_t creatorId_;     // Entity that created space (instanced)

    std::map<uint32_t, CellEntity::Ptr> entities_;  // All entities
    std::map<uint32_t, CellEntity::Ptr> players_;   // Player entities
    NavigationMesh* navMesh_;                        // Pathfinding data
    WorldData world_;                                // Zone extents

    void addEntity(CellEntity::Ptr entity);
    void removeEntity(uint32_t entityId);
    void promoteToPlayer(uint32_t entityId);
    void demoteToEntity(uint32_t entityId);
    void tick();
    NavigationPath* findPath(const Vec3& start, const Vec3& end);
    bool isValidPosition(const Vec3& pos);
};
```

**SpaceManager** — Singleton managing all spaces:

```cpp
class SpaceManager {
    std::map<uint32_t, Space*> spaces_;
    std::vector<uint32_t> freeIds_;
    std::map<std::string, WorldData> worlds_;
    uint64_t time_;
    uint32_t tickRate_;

    Space* create(const std::string& worldName, bool instanced);
    void destroy(uint32_t spaceId);
    Space* get(uint32_t spaceId);
    Space* get(const std::string& worldName);
    void loadSpaces();
    void initTicks(uint32_t tickRate);
    uint32_t addEntityTimer(CellEntity* entity, float delay, float repeat);
    void cancelEntityTimer(uint32_t timerId);
};
```


## World Grid — Spatial Partitioning

`src/entity/world_grid.hpp`

Template-based chunked visibility system using CRTP:

```cpp
template <typename T>
class WorldGrid {
    struct Params {
        float width, height;         // World dimensions
        float minX, minY;            // World origin
        float sizeX, sizeY;          // Chunk size (default 50 units)
        int visionDistance;           // AoI range in chunks (default 3)
        int hysteresis;              // Hysteresis in chunks (default 1)
    };

    struct Chunk {
        std::vector<T*> objects;     // Entities in this chunk
        std::vector<T*> witnesses;   // Visibility controllers
    };

    void add(T* entity);
    void remove(T* entity);
    void move(T* entity, const Vec3& newPos);  // Triggers visibility events
    void visitWitnesses(T* entity, Visitor& v);
    void makeVisible(T* target, T* witness);
    void makeInvisible(T* target, T* witness);
    void addVisionException(T* entity, T* exception);
};
```

**AoI algorithm:**
1. World is divided into chunks of `sizeX × sizeY` units
2. Each chunk tracks entities and witnesses (players with vision)
3. When an entity moves to a new chunk, the grid compares old and new visibility sets
4. Entities entering vision range trigger `makeVisible()` → client receives `enterAoI`
5. Entities leaving vision range (plus hysteresis) trigger `makeInvisible()` → client receives `leaveAoI`
6. Hysteresis prevents flapping when entities are near chunk boundaries


## Entity ID Allocation

| Range | Owner | Purpose |
|-------|-------|---------|
| `0x00000001`–`0x00FFFFFF` | CellApp | Local-only entities (not visible to Base) |
| `0x01000000`–`0xEFFFFFFF` | BaseApp | Persistent entities (players, NPCs), partitioned by `cell_id` (16M per cell) |
| `0xF0000000`–`0xFFFFFFFF` | Reserved | System entities |


## Database Layer

`src/common/database.hpp/cpp`

SOCI 3.2.1 wrapper with async operations:

```cpp
class Database {
    static const int KeepaliveInterval = 60000;  // ms

    soci::session* session_;      // Database connection
    std::thread thread_;          // Worker thread for async queries
    std::queue<DbRequest> queue_; // Pending requests
    std::mutex queueLock_;        // Thread-safe queue

    // Async single-row query with callback
    void fetchRow(const std::string& sql, DbRowFunction callback, DbErrorFunction error);

    // Blocking query returning result list
    std::vector<soci::row> synchronousQuery(const std::string& sql);

    // Blocking non-query
    void synchronousPerform(const std::string& sql);

    // Fire-and-forget non-query
    void asyncPerform(const std::string& sql);
};
```

**Python bridge:**

The `Atrea` module exposes `dbQuery()` and `dbPerform()` to Python scripts. These route through the Database worker thread to avoid blocking the main event loop.


## Authentication Server

`src/authentication/service_main.hpp/cpp`

Login and shard management:

```cpp
class AuthenticationService : public Service {
    TcpServer<LogonConnection>* loginServer_;       // Port 8081 — client login
    TcpServer<FrontendConnection>* frontendServer_; // Port 13001 — shard registration

    std::map<uint32_t, FrontendConnection*> shards_;           // Registered shards
    std::map<uint32_t, FrontendConnection*> onlineAccounts_;   // Per-shard online tracking
    std::map<uint64_t, LoginSession> sessions_;                // Active login sessions

    struct LoginSession {
        uint32_t accountId;
        uint8_t accessLevel;
        std::string accountName;
    };

    void registerShard(FrontendConnection* shard);
    void accountOnline(uint32_t accountId, FrontendConnection* shard);
    void accountOffline(uint32_t accountId);
    LoginSession* getSession(uint64_t sessionId);
};
```

**Login flow:**
1. Client sends HTTP/SOAP request to `LogonConnection` on port 8081
2. Auth validates credentials against database
3. Auth creates a `LoginSession` with session token
4. Client receives shard list and session token
5. Client connects to BaseApp on port 32832 with session token
6. BaseApp validates token with Auth via `FrontendConnection` on port 13001


## Logging

`src/log/logger.hpp/cpp`

Thread-safe async logger with circular buffer:

**Log levels:**

| Level | Value | Macro |
|-------|-------|-------|
| TRACE | 0 | `LOG(LL_TRACE, ...)` |
| DEBUG2 | 1 | `LOG(LL_DEBUG2, ...)` |
| DEBUG1 | 2 | `LOG(LL_DEBUG1, ...)` |
| INFO | 3 | `LOG(LL_INFO, ...)` |
| WARNING | 4 | `LOG(LL_WARNING, ...)` |
| ERROR | 5 | `LOG(LL_ERROR, ...)` |
| CRITICAL | 6 | `LOG(LL_CRITICAL, ...)` |

**Categories:**

| Category | Purpose |
|----------|---------|
| `CATEGORY_ENTITY` | Entity lifecycle events |
| `CATEGORY_MERCURY` | Network events |
| `CATEGORY_MESSAGES` | Network message content |
| `CATEGORY_SPACE` | Space events |

**Usage:**

```cpp
LOG(LL_INFO, "Player %s logged in", playerName.c_str());
LOGC(LL_DEBUG1, CATEGORY_MERCURY, "Packet received from %s", addr.toString().c_str());
SGW_ASSERT(entity != nullptr);  // Debug assertion
```


## Python Integration

`src/entity/python.hpp/cpp`

### Module Registration

The `Atrea` Python module is registered at startup via `PyRegisterModule()`. This exposes C++ functions and classes to Python:

- `Atrea.findEntity(entityId)` — Locate entity in memory
- `Atrea.dbQuery(sql)` — Execute SQL query
- `Atrea.addTimer(delay, repeat, callback)` — Register timer
- `Atrea.cancelTimer(timerId)` — Cancel timer
- `Atrea.reloadClasses()` — Hot-reload Python scripts
- `Atrea.CellEntity` / `Atrea.BaseEntity` — Base classes for Python entities

### GIL Management

Python calls from C++ use `PyGilGuard` RAII to acquire/release the GIL:

```cpp
class PyGilGuard {
    PyGILState_STATE state_;
    PyGilGuard() : state_(PyGILState_Ensure()) {}
    ~PyGilGuard() { PyGILState_Release(state_); }
};
```

### Entity Class Exposure

Entity types are loaded from XML and bound to Python classes via Boost.Python. The `PyClassDef::expose<T>()` method registers entity classes with the Python interpreter, and `PyClassDef::create<T>()` instantiates Python objects bound to C++ entity instances.


## Threading Model

```
┌─────────────────────────────────────────────────────────────┐
│                    Main Thread (ASIO Reactor)                │
│  • All game logic                                           │
│  • All network I/O (async)                                  │
│  • Python script execution (under GIL)                      │
│  • Timer callbacks                                          │
│  • Entity ticks                                             │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────┐
│          DB Worker Thread           │
│  • Blocking SOCI operations         │
│  • Async query execution            │
│  • Results posted back to main      │
└─────────────────────────────────────┘

┌─────────────────────────────────────┐
│         Logger Worker Thread        │
│  • Circular buffer drain            │
│  • File/console output              │
└─────────────────────────────────────┘

┌─────────────────────────────────────┐
│         Console Thread              │
│  • Python console I/O (port 8989)   │
│  • Acquires GIL for Python eval     │
└─────────────────────────────────────┘
```

Each service (Auth, Base, Cell) runs as a **separate process** with its own set of threads. All game logic runs single-threaded on the ASIO reactor — there is no concurrent mutation of entity state.


## Key Design Patterns

| Pattern | Where Used | Purpose |
|---------|-----------|---------|
| **Reactor** (Boost.Asio) | All services | Single-threaded async I/O event loop |
| **CRTP** | `WorldGrid<T>`, `WorldGridMember<T>` | Compile-time polymorphism for spatial partitioning |
| **Template Metaprogramming** | `EntityManager<T>`, `TcpServer<T>` | Generic entity pools and TCP acceptors |
| **Singleton** | `Service`, `SpaceManager`, `PyTypeDb` | Global service instances |
| **Observer** | Witness system, AoI events | Entities observe each other entering/leaving visibility |
| **RPC Abstraction** | `Mailbox`, `MailboxClass` | Transparent remote method invocation |
| **Factory** | `PyClassDef::create<T>()`, `TcpServer` | Entity and connection creation |
| **Sliding Window** | `BaseChannel` | Reliable UDP with ARQ |
| **RAII** | `PyGilGuard`, `shared_ptr` everywhere | Resource and lock management |
| **Command** | `Bundle::beginMessage()`/`endMessage()` | Message construction as serialized commands |
