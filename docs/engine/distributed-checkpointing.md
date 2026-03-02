# Distributed Checkpointing & Crash Recovery

> **Last updated**: 2026-03-01
> **Phase**: 5 -- BigWorld Engine Subsystems
> **RE Status**: Documented from BigWorld 2.0.1 reference source; partially implemented in Cimmeria
> **Sources**: BW `lib/server/backup_hash.hpp`, BW `lib/server/backup_hash.cpp`, BW `lib/server/backup_hash_chain.hpp`, BW `lib/server/backup_hash_chain.cpp`, BW `lib/server/auto_backup_and_archive.hpp`, BW `lib/server/auto_backup_and_archive.cpp`, BW `lib/server/reviver_subject.hpp`, BW `lib/server/reviver_subject.cpp`, BW `lib/server/reviver_common.hpp`, BW `server/baseapp/backup_sender.hpp`, Cimmeria `src/cellapp/base_client.cpp`, Cimmeria `src/baseapp/entity/base_entity.hpp`, Cimmeria `src/baseapp/mercury/cell_handler.cpp`, Cimmeria `src/mercury/base_cell_messages.hpp`, Cimmeria `src/cellapp/entity/cell_entity.hpp`

---

## Overview

BigWorld's distributed checkpointing system provides crash recovery for a live MMO cluster. Each BaseApp periodically backs up its entities to one or more other BaseApps, so if a BaseApp process crashes, a surviving BaseApp can restore the dead process's entities from backup. This is complemented by a health-monitoring reviver system that detects process failures and triggers recovery.

The system has three main components:

1. **BackupHash** -- deterministic mapping of entity IDs to backup BaseApp addresses
2. **BackupSender** -- per-BaseApp coordinator that incrementally backs up entities each tick
3. **ReviverSubject** -- health-check ping system with priority-based election

Cimmeria implements basic entity backup for **space transitions** (teleporting between zones) but does NOT implement the full distributed crash recovery system. Entities are backed up when moving between spaces and restored on the other side, but there is no periodic backup to other BaseApps, no hash-based backup routing, and no reviver process.

---

## BackupHash Algorithm

The `BackupHash` class maps entity IDs to backup BaseApp addresses using a hash function with power-of-2 virtual bucketing. Each BaseApp maintains its own hash table, with a randomly chosen prime to ensure hash functions differ across BaseApps (preventing degenerate distributions after failures).

### MiniBackupHash (Base Class)

From `backup_hash.hpp`:

```cpp
class MiniBackupHash
{
public:
    MiniBackupHash( uint32 prime = 0, uint32 size = 0 );

    uint32 prime() const;
    uint32 size() const;        // Actual number of buckets
    uint32 virtualSize() const; // Smallest power of 2 >= size

    uint32 hashFor( EntityID id ) const;
    uint32 virtualSizeFor( uint32 index ) const;

protected:
    void handleSizeChange( uint32 newSize );

    uint32 prime_;

private:
    uint32 size_;
    uint32 virtualSize_;
};
```

### Hash Function

From `backup_hash.cpp`:

```cpp
uint32 MiniBackupHash::hashFor( EntityID id ) const
{
    if (size_ > 0)
    {
        // Multiply by prime, discard lower 8 bits, modulo virtual size
        uint32 hash = ((uint32(id) * prime_) >> 8) % virtualSize_;

        // If hash lands outside actual size, fold it back
        if (hash >= size_)
        {
            hash -= virtualSize_/2;
        }

        return hash;
    }
    return uint32( -1 );
}
```

The algorithm:

1. Multiply entity ID by a large prime modulo 2^32
2. Right-shift by 8 bits to discard poorly-randomised low bits
3. Take modulo `virtualSize_` (smallest power of 2 >= bucket count)
4. If the result exceeds the actual bucket count, subtract `virtualSize_/2` to fold back

This design ensures that when a bucket is added or removed, only entities in one existing bucket are affected (the bucket being split or merged), minimizing re-backup work.

### Virtual Size Bucketing

The virtual size mechanism ensures smooth growth/shrink behaviour:

```cpp
void MiniBackupHash::handleSizeChange( uint32 newSize )
{
    size_ = newSize;
    if (size_ > 0)
    {
        virtualSize_ = 1;
        while (virtualSize_ < size_)
        {
            virtualSize_ *= 2;
        }
    }
    else
    {
        virtualSize_ = 0;
    }
}
```

Example with 6 buckets (`virtualSize_ = 8`):

| Hash % 8 | Actual Bucket | Virtual Size for Bucket |
|:-:|:-:|:-:|
| 0 | 0 | 8 |
| 1 | 1 | 8 |
| 2 | 2 | 4 |
| 3 | 3 | 4 |
| 4 | 4 | 8 |
| 5 | 5 | 8 |
| 6 | 2 (folded) | -- |
| 7 | 3 (folded) | -- |

Buckets 2 and 3 each receive entities from two hash values (smaller virtual size), while buckets 0, 1, 4, 5 each receive from one hash value.

### Random Prime Selection

Each BaseApp chooses a random prime at construction to create a unique hash function. This prevents all BaseApps from having identical backup distribution, which would cause degenerate allocation after multiple failures:

```cpp
uint32 BackupHash::choosePrime()
{
    static const uint32 primes[] =
    {
        0x9e377e55, 0x9e377e53, 0x9e377e43, 0x9e377e41, 0x9e377e37,
        0x9e377e11, 0x9e377e07, 0x9e377de1, 0x9e377db7, 0x9e377da5,
        // ... 50 primes total, all near 0x9e377___
    };
    return primes[ rand() % (sizeof( primes )/sizeof( primes[0])) ];
}
```

All 50 primes are clustered near `0x9e3779__` (close to the golden ratio constant `0x9e3779b9`), which provides good hash distribution for multiplicative hashing.

### BackupHash (Derived Class)

The full `BackupHash` adds an address vector mapping bucket indices to BaseApp addresses:

```cpp
class BackupHash : public MiniBackupHash
{
public:
    BackupHash();

    Mercury::Address addressFor( EntityID id ) const;

    // Diff-based change detection
    class DiffVisitor
    {
    public:
        virtual void onAdd( const Mercury::Address & addr,
                uint32 index, uint32 virtualSize, uint32 prime ) = 0;
        virtual void onChange( const Mercury::Address & addr,
                uint32 index, uint32 virtualSize, uint32 prime ) = 0;
        virtual void onRemove( const Mercury::Address & addr,
                uint32 index, uint32 virtualSize, uint32 prime ) = 0;
    };

    void diff( const BackupHash & other, DiffVisitor & visitor );
    void push_back( const Mercury::Address & addr );
    bool erase( const Mercury::Address & addr );

private:
    typedef std::vector< Mercury::Address > Container;
    Container addrs_;
};
```

The `addressFor()` method is the primary lookup:

```cpp
Mercury::Address BackupHash::addressFor( EntityID id ) const
{
    if (!addrs_.empty())
    {
        return addrs_[ this->hashFor( id ) ];
    }
    return Mercury::Address( 0, 0 );
}
```

The `diff()` method compares two hash states and calls visitor methods for adds, changes, and removals -- used when the backup topology changes (BaseApp added/removed) to determine which entities need re-backing-up.

### Erase Strategy

When a BaseApp is removed, the erase method moves the last element into the gap to minimize hash disruption:

```cpp
bool BackupHash::erase( const Mercury::Address & addr )
{
    Container::iterator iter = std::find( addrs_.begin(), addrs_.end(), addr );
    if (iter != addrs_.end())
    {
        *iter = addrs_.back();    // Swap with last
        addrs_.pop_back();
        this->handleSizeChange( addrs_.size() );
        return true;
    }
    return false;
}
```

---

## BackupHashChain

The `BackupHashChain` resolves addresses through a chain of dead BaseApps. When a BaseApp dies, its backup hash is recorded. If the backup BaseApp also dies, the chain allows following through multiple failures to find a live BaseApp that holds the data.

### Class Definition

From `backup_hash_chain.hpp`:

```cpp
class BackupHashChain
{
public:
    BackupHashChain();

    void adjustForDeadBaseApp( const Mercury::Address & deadApp,
                               const BackupHash & hash );
    Mercury::Address addressFor( const Mercury::Address & address,
                                 EntityID entityID ) const;

private:
    typedef std::map< Mercury::Address, BackupHash > History;
    History history_;   // Maps dead BaseApp address -> its backup hash at death
};
```

### Chain Resolution

When looking up an entity's backup location, the chain follows through any dead intermediate addresses:

```cpp
Mercury::Address
BackupHashChain::addressFor( const Mercury::Address & address,
                             EntityID entityID ) const
{
    History::const_iterator firstMatch = history_.find( address );
    if (firstMatch == history_.end())
    {
        return address;  // Address is live, no redirection needed
    }

    // Loop detection via visited set
    std::set< Mercury::Address > visited;
    History::const_iterator match = firstMatch;
    Mercury::Address position = address;

    while (match != history_.end())
    {
        if (visited.count( position ))
        {
            WARNING_MSG( "BackupHashChain::addressFor( %s, %u ): "
                         "Infinite loop\n", address.c_str(), entityID );
            return address;  // Loop detected, return original
        }
        visited.insert( position );

        // Follow the chain: where would this entity have been backed up?
        position = match->second.addressFor( entityID );
        match = history_.find( position );
    }

    return position;  // Found a live BaseApp
}
```

The chain handles cascading failures. For example, if BaseApp A backed up to BaseApp B, and B backed up to BaseApp C, and both A and B die:

1. Look up A's address in history -- find A's backup hash
2. A's hash says entity X was backed up to B
3. Look up B's address in history -- find B's backup hash
4. B's hash says entity X (now a backup) was backed up to C
5. C is not in history, so C is live -- entity X's backup is on C

Loop detection (via `std::set<Address> visited`) prevents infinite recursion if backup routing forms a cycle (which should not happen in practice but is guarded against).

### Serialization

The chain is serializable for transmission between server processes:

```cpp
BinaryOStream & operator<<( BinaryOStream & os,
        const BackupHashChain & hashChain )
{
    os << uint16( hashChain.history_.size() );
    for (auto iHash = hashChain.history_.begin();
         iHash != hashChain.history_.end(); ++iHash)
    {
        os << iHash->first << iHash->second;
    }
    return os;
}
```

---

## AutoBackupAndArchive

This policy enum controls whether an entity is automatically backed up and/or archived to the database.

From `auto_backup_and_archive.hpp`:

```cpp
namespace AutoBackupAndArchive
{
    enum Policy
    {
        NO        = 0,  // Do not auto-backup this entity
        YES       = 1,  // Auto-backup every cycle
        NEXT_ONLY = 2,  // Backup on the next cycle only, then revert to NO
    };
}
```

The `NEXT_ONLY` policy is exposed to Python scripting via `addNextOnlyConstant()`, allowing game logic to trigger a single backup (e.g., after a significant state change like completing a quest):

```cpp
void AutoBackupAndArchive::addNextOnlyConstant( PyObject * pModule )
{
    PyObjectPtr pNextOnly( Script::getData( AutoBackupAndArchive::NEXT_ONLY ),
        PyObjectPtr::STEAL_REFERENCE );
    PyObject_SetAttrString( pModule, "NEXT_ONLY", pNextOnly.get() );
}
```

The `Script::setData` converter validates values are 0--2:

```cpp
int setData( PyObject * pObj,
        AutoBackupAndArchive::Policy & value,
        const char * varName )
{
    int intVal = int(value);
    int result = Script::setData( pObj, intVal, varName );

    if ((result == 0) && (0 <= intVal) && (intVal <= 2))
    {
        value = AutoBackupAndArchive::Policy( intVal );
        return 0;
    }
    else
    {
        PyErr_Format( PyExc_ValueError,
                "%s must be set to an integer between 0 and 2", varName );
        return -1;
    }
}
```

---

## BackupSender

The `BackupSender` is the per-BaseApp coordinator that manages incremental entity backup. It does not back up all entities every tick -- instead, it spreads the work across multiple ticks to amortize the cost.

From `backup_sender.hpp`:

```cpp
class BackupSender
{
public:
    BackupSender( BaseApp & baseApp );

    void tick( const Bases & bases,
               Mercury::NetworkInterface & networkInterface );

    Mercury::Address addressFor( EntityID entityID ) const
    {
        return entityToAppHash_.addressFor( entityID );
    }

    void addToStream( BinaryOStream & stream );
    void handleBaseAppDeath( const Mercury::Address & addr );
    void setBackupBaseApps( BinaryIStream & data,
                            Mercury::NetworkInterface & networkInterface );

    void restartBackupCycle( const Bases & bases );
    void startOffloading() { isOffloading_ = true; }

    bool autoBackupBase( Base & base,
                         Mercury::BundleSendingMap & bundles,
                         Mercury::ReplyMessageHandler * pHandler = NULL );
    bool backupBase( Base & base,
                     Mercury::BundleSendingMap & bundles,
                     Mercury::ReplyMessageHandler * pHandler = NULL );

private:
    void ackNewBackupHash();

    int offloadPerTick_;                // How many entities to offload per tick
    float backupRemainder_;             // Fractional overflow for next tick
    std::vector<EntityID> basesToBackUp_; // Queue of entities pending backup

    BackupHash entityToAppHash_;        // Current active hash
    BackupHash newEntityToAppHash_;     // Hash being transitioned to
    bool isUsingNewBackup_;             // True during hash transition
    bool isOffloading_;                 // True during shutdown/offload
    BaseApp & baseApp_;
};
```

### Tick-Based Incremental Backup

The `basesToBackUp_` vector holds the list of entity IDs that still need backing up in the current cycle. Each `tick()` call processes a fraction of this list. The `backupRemainder_` accumulates fractional overflow to ensure even distribution across ticks.

### Hash Transition

When the cluster topology changes (BaseApp added/removed), the BackupSender receives a new hash via `setBackupBaseApps()`. During the transition period:

1. `newEntityToAppHash_` holds the target hash
2. `isUsingNewBackup_` is set to true
3. Entities are re-backed-up to their new destinations
4. Once all entities have been re-backed-up, `ackNewBackupHash()` is called and the old hash is replaced

### Integration Points

- `handleBaseAppDeath()` is called when the BaseAppMgr reports a dead BaseApp, triggering hash recalculation
- `restartBackupCycle()` refills the `basesToBackUp_` queue from the current entity list
- `startOffloading()` enters offload mode during graceful shutdown, where all entities are backed up and transferred

---

## ReviverSubject

The reviver system monitors server process health using a ping-based protocol. A dedicated reviver process sends periodic pings to all server processes. If a process fails to respond, the reviver triggers recovery.

### Reviver Constants

From `reviver_common.hpp`:

```cpp
typedef uint8 ReviverPriority;
const ReviverPriority REVIVER_PING_NO  = 0;
const ReviverPriority REVIVER_PING_YES = 1;
```

### Class Definition

From `reviver_subject.hpp`:

```cpp
class ReviverSubject : public Mercury::InputMessageHandler
{
public:
    ReviverSubject();
    void init( Mercury::NetworkInterface * pInterface,
               const char * componentName );
    void fini();

    static ReviverSubject & instance() { return instance_; }

private:
    virtual void handleMessage( const Mercury::Address & srcAddr,
            Mercury::UnpackedMessageHeader & header,
            BinaryIStream & data );

    Mercury::NetworkInterface * pInterface_;
    Mercury::Address   reviverAddr_;     // Address of the accepted reviver
    uint64             lastPingTime_;    // Timestamp of last accepted ping
    ReviverPriority    priority_;        // Priority of the accepted reviver (lower = better)
    int                msTimeout_;       // Timeout in milliseconds

    static ReviverSubject instance_;     // Singleton
};
```

The macro `MF_REVIVER_PING_MSG()` registers the ping message handler:

```cpp
#define MF_REVIVER_PING_MSG() \
    MERCURY_VARIABLE_MESSAGE( reviverPing, 2, &ReviverSubject::instance() )
```

### Ping Protocol

Each server process (BaseApp, CellApp, etc.) embeds a `ReviverSubject` singleton. The reviver process sends periodic pings containing its priority level. The subject responds with `REVIVER_PING_YES` or `REVIVER_PING_NO`:

```cpp
void ReviverSubject::handleMessage( const Mercury::Address & srcAddr,
        Mercury::UnpackedMessageHeader & header,
        BinaryIStream & data )
{
    uint64 currentPingTime = timestamp();
    ReviverPriority priority;
    data >> priority;

    bool accept = (reviverAddr_ == srcAddr);

    if (!accept)
    {
        if (priority < priority_)
        {
            // New reviver has better (lower) priority
            accept = true;
        }
        else
        {
            // Check if current reviver has timed out
            uint64 delta = (currentPingTime - lastPingTime_) * uint64(1000);
            delta /= stampsPerSecond();
            int msBetweenPings = int(delta);

            if (msBetweenPings > msTimeout_)
            {
                // Current reviver timed out, accept new one
                accept = true;
            }
        }
    }

    Mercury::Bundle bundle;
    bundle.startReply( header.replyID );
    if (accept)
    {
        reviverAddr_ = srcAddr;
        lastPingTime_ = currentPingTime;
        priority_ = priority;
        bundle << REVIVER_PING_YES;
    }
    else
    {
        bundle << REVIVER_PING_NO;
    }
    pInterface_->send( srcAddr, bundle );
}
```

### Priority-Based Election

Multiple reviver processes can run simultaneously for redundancy. Each has a priority (lower = higher precedence). A subject only accepts a new reviver if:

1. The new reviver has a **lower** priority number than the current one, OR
2. The current reviver has **timed out** (no ping received within `msTimeout_`)

The timeout is configurable per component type via `BWConfig`:

```cpp
msTimeout_ = int(BWConfig::get( buf,
            BWConfig::get( "reviver/subjectTimeout", 0.2f ) ) * 1000);
```

Default timeout is 200ms. The initial priority is `0xff` (worst possible), ensuring any reviver will be accepted on first contact.

---

## Cimmeria's Existing Backup

Cimmeria implements a subset of BigWorld's backup system, focused specifically on preserving entity state during **space transitions** (when an entity teleports from one space/zone to another).

### Message Protocol

From `base_cell_messages.hpp`:

```cpp
/*
 * 0x0A Backup Entity
 *
 * uint32    Space ID    - Space ID of the entity
 * uint32    Entity ID   - Entity to back up
 * uint32    Object Size
 * uint8[]   Object
 * uint32    Python Object Size
 * uint8[]   Python Object
 */
CELL_BASE_BACKUP_ENTITY = 0x0A,
```

### CellApp Side: Sending Backup

The `sendEntityBackup()` method in `base_client.cpp` serializes the entity's cell state and sends it to the BaseApp:

```cpp
void BaseAppClient::sendEntityBackup(CellEntity * entity)
{
    SGW_ASSERT(isReady());
    Writer request(*this);
    request.beginMessage(Mercury::CELL_BASE_BACKUP_ENTITY);
    request << entity->space()->spaceId() << entity->entityId();
    entity->backup(request);
    request.endMessage();
}
```

The `CellEntity::backup()` method serializes all cell state into the stream:

```cpp
template <typename _T>
void backup(_T & stream)
{
    SGW_ASSERT(classDef() && "Cannot backup server-only entities");
    bool player;
    stream << (uint32_t)43;  // Backup format version/size marker
    stream << position_.x << position_.y << position_.z;
    stream << rotation_.x << rotation_.y << rotation_.z;
    stream << velocity_.x << velocity_.y << velocity_.z;
    stream << maxSpeed_ << movementType_ << player << (client_ != nullptr);

    PyGilGuard guard;
    auto backup = Entity::callMethod("backup");
    if (backup)
    {
        PyTypeDb::instance().findType("PYTHON")->pack(stream, backup);
    }
    else
    {
        // Fallback: pack empty Python object
        PyTypeDb::instance().findType("PYTHON")->pack(stream, bp::object());
    }
}
```

The backup contains:
- Position (3 floats)
- Rotation (3 floats)
- Velocity (3 floats)
- Movement metadata (maxSpeed, movementType, player flag, client flag)
- Python entity state (serialized via entity's `backup()` Python method)

### Trigger: Space Transitions Only

Backup is triggered in `CellEntity::moveTo()` when an entity changes spaces:

```cpp
void CellEntity::moveTo(uint32_t spaceId, std::string const & worldName,
                         Vec3 const & position, Vec3 const & rotation)
{
    BaseAppClient * client = CellService::cell_instance().baseClient();
    client->sendEntityBackup(this);
    client->sendSwitchSpace(entityId(), spaceId, worldName, position, rotation);
}
```

The backup is sent immediately before the space switch request, ensuring the BaseApp has a snapshot of the entity's cell state before the transition occurs.

### BaseApp Side: Receiving Backup

The `CellAppConnection::onEntityBackup()` handler in `cell_handler.cpp` unpacks the backup into the entity:

```cpp
void CellAppConnection::onEntityBackup(Reader & msg)
{
    uint32_t spaceId, entityId;
    msg >> spaceId >> entityId;

    BaseEntity::Ptr entity = BaseEntityManager<BaseEntity>::instance().get(entityId);
    if (!entity)
    {
        WARNC(CATEGORY_MESSAGES, "Cannot backup entity %d without a base part", entityId);
        return;
    }
    entity->unpackBackup(msg);
}
```

### BaseEntity Backup Storage

From `base_entity.hpp`, the entity stores two byte arrays -- one for C++ cell state and one for Python state:

```cpp
class BaseEntity : public Entity
{
public:
    template <typename _T>
    void unpackBackup(_T & stream)
    {
        delete [] backup_;
        delete [] pythonBackup_;

        stream >> backupSize_;
        backup_ = new uint8_t[backupSize_];
        stream.read(backup_, backupSize_);

        stream >> pythonBackupSize_;
        pythonBackup_ = new uint8_t[pythonBackupSize_];
        stream.read(pythonBackup_, pythonBackupSize_);
    }

    template <typename _T>
    void packBackup(_T & stream)
    {
        SGW_ASSERT(hasBackup() && "Cell part of the entity is not backed up");
        stream.write(backup_, backupSize_);
        stream << pythonBackupSize_;
        stream.write(pythonBackup_, pythonBackupSize_);
    }

    bool hasBackup();

private:
    uint8_t * backup_;           // Cell C++ state blob
    uint32_t backupSize_;
    uint8_t * pythonBackup_;     // Cell Python state blob
    uint32_t pythonBackupSize_;
};
```

The backup is later sent to the target CellApp via `packBackup()` when the entity is restored in the new space (using the `CELL_BASE_RESTORE_ENTITY_ACK` flow).

---

## Gap Analysis

### What Cimmeria Has

| Feature | Status | Details |
|---|---|---|
| Entity backup serialization | Implemented | `CellEntity::backup()` / `BaseEntity::unpackBackup()` |
| Backup message protocol | Implemented | `CELL_BASE_BACKUP_ENTITY = 0x0A` |
| Space transition backup | Implemented | `moveTo()` triggers backup before space switch |
| Backup restore flow | Implemented | `onRestoreEntityRequest()` / `packBackup()` / `RESTORE_ENTITY_ACK` |
| Backup storage on BaseEntity | Implemented | `backup_` / `pythonBackup_` byte arrays |

### What Cimmeria Is Missing

| Feature | Status | Impact |
|---|---|---|
| `BackupHash` entity-to-BaseApp routing | Not implemented | No deterministic backup distribution |
| `BackupHashChain` cascading failure resolution | Not implemented | No multi-failure recovery |
| `BackupSender` periodic incremental backup | Not implemented | No continuous backup; only on space transitions |
| `AutoBackupAndArchive` policy system | Not implemented | No script-controlled backup triggers |
| `ReviverSubject` health monitoring | Not implemented | No automatic failure detection |
| Reviver process | Not implemented | No automated crash recovery |
| Cross-BaseApp backup storage | Not implemented | Backups stored on same BaseApp, not distributed |
| Database archiving integration | Not implemented | No periodic database saves |
| Hash transition during topology changes | Not implemented | Single BaseApp assumed |

### Summary

Cimmeria's backup system is **functionally sufficient for its current single-BaseApp architecture**. The backup/restore mechanism correctly handles the primary use case: preserving entity state when entities move between spaces. The entity's C++ state (position, rotation, velocity, movement metadata) and Python state (via `Entity.backup()`) are serialized, transmitted, and restored correctly.

The missing distributed components (`BackupHash`, `BackupSender`, `ReviverSubject`) are only needed in a multi-BaseApp deployment where crash recovery is critical. Since Cimmeria currently runs a single BaseApp, there is no second BaseApp to back up to. If a multi-BaseApp deployment becomes a goal, these components would need to be implemented.

### Implementation Priority Recommendations

| Priority | Component | Rationale |
|---|---|---|
| **Low** | `BackupHash` / `BackupSender` | Only needed for multi-BaseApp. Single BaseApp has no backup target. |
| **Low** | `BackupHashChain` | Only relevant after BackupHash is implemented. |
| **Low** | `ReviverSubject` | Only useful in multi-process deployments with automated recovery. |
| **Medium** | `AutoBackupAndArchive` | Would enable periodic database saves, useful even for single-BaseApp. |
| **None** | Existing backup/restore | Already working correctly for space transitions. |

The most practical near-term addition would be `AutoBackupAndArchive` to enable periodic database persistence of entity state, even in a single-BaseApp configuration. The full distributed backup system can be deferred until multi-BaseApp support is a concrete goal.

---

## Related Documents

- [BigWorld Architecture](bigworld-architecture.md) -- distributed entity model and BaseApp/CellApp roles
- [Space Management](space-management.md) -- space transitions that trigger backup
- [Entity Property Synchronization Protocol](../protocol/entity-property-sync.md) -- property encoding used in backup streams
- [Entity Type Catalog](entity-type-catalog.md) -- entity types and their `.def` definitions
