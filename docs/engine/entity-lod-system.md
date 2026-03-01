# Entity LOD (Level of Detail) System

> **Last updated**: 2026-03-01
> **Phase**: 5 -- BigWorld Engine Subsystems
> **RE Status**: Documented from BigWorld 2.0.1 reference source; NOT used by Stargate Worlds
> **Sources**: BW `lib/entitydef/data_lod_level.hpp`, BW `lib/entitydef/data_lod_level.cpp`, BW `lib/entitydef/volatile_info.hpp`, BW `lib/entitydef/volatile_info.cpp`, BW `lib/entitydef/volatile_info.ipp`, BW `lib/entitydef/property_change.hpp`, BW `lib/entitydef/property_change.cpp`, BW `lib/entitydef/property_event_stamps.hpp`, BW `lib/entitydef/property_event_stamps.ipp`, BW `lib/entitydef/data_description.hpp`, BW `lib/network/basictypes.hpp`, SGW `entities/defs/*.def`

---

## Overview

BigWorld's entity property LOD system is a bandwidth optimization feature that reduces the amount of property data sent to clients observing distant entities. Properties are assigned to LOD levels, and as an entity moves farther from an observer, higher-detail-level properties are dropped from updates. This allows MMOs with large player counts to scale efficiently -- nearby entities send full property data while distant ones send only essential information.

**Stargate Worlds does not use this system.** All `<LoDLevels>` tags in SGW `.def` files are empty, confirmed across every entity definition. The client binary contains no `DataLoDLevel`, `VolatileInfo`, or `EventStamp` functions -- only UE3/CME rendering LOD for draw distance. Property change encoding (`MAX_SIMPLE_PROPERTY_CHANGE_ID = 60`) IS used by Cimmeria for entity property synchronization, documented separately in the entity-property-sync findings.

---

## DataLoDLevel

Each LOD level defines a distance threshold at which properties assigned to that level stop being sent to an observer. Transitions use hysteresis to prevent flickering when an entity is near the boundary.

### Class Definition

From `data_lod_level.hpp`:

```cpp
class DataLoDLevel
{
public:
    DataLoDLevel();

    float low() const;     // Squared distance: go to MORE detailed level if below
    float high() const;    // Squared distance: go to LESS detailed level if above
    float start() const;   // Raw distance in metres where this level begins
    float hyst() const;    // Hysteresis buffer in metres

    void set( const std::string & label, float start, float hyst );
    const std::string & label() const;
    void finalise( DataLoDLevel * pPrev, bool isLast );

    int index() const;
    void index( int i );

    enum
    {
        OUTER_LEVEL = -2,   // Property has no LOD level (always sent)
        NO_LEVEL = -1       // Property not assigned to any level
    };

private:
    float low_;             // Squared distance threshold (more detail)
    float high_;            // Squared distance threshold (less detail)
    float start_;           // Start distance in metres
    float hyst_;            // Hysteresis in metres
    std::string label_;     // Human-readable label ("near", "far", etc.)
    int index_;             // Index for remapping after sort
};
```

### MAX_DATA_LOD_LEVELS

From `basictypes.hpp`:

```cpp
const int MAX_DATA_LOD_LEVELS = 6;
```

A maximum of 6 distinct LOD levels are supported per entity type. The `DataLoDLevels` container stores `MAX_DATA_LOD_LEVELS + 1` entries (7 total), where the extra entry is the "always send" level for properties not assigned to any specific LOD tier.

### Distance Calculation

LOD thresholds use **squared distances** for performance (avoids sqrt). The `finalise()` method computes low/high thresholds from raw distances:

```cpp
void DataLoDLevel::finalise( DataLoDLevel * pPrev, bool isLast )
{
    if (pPrev)
    {
        float v = pPrev->start_;
        low_ = v * v;           // Squared distance from previous level's start
    }
    else
    {
        low_ = -1;              // First level: always active when close
    }

    if (!isLast)
    {
        float v = start_ + hyst_;
        high_ = v * v;          // Squared distance = (start + hysteresis)^2
    }
    // Last level: high_ stays FLT_MAX (never transitions to less detail)
}
```

Default values in the constructor:

| Field | Default |
|-------|---------|
| `low_` | `FLT_MAX` |
| `high_` | `FLT_MAX` |
| `start_` | `FLT_MAX` |
| `hyst_` | `0.0f` |

### DataLoDLevels Container

The container manages multiple levels, sorted by start distance:

```cpp
class DataLoDLevels
{
public:
    DataLoDLevels();
    bool addLevels( DataSectionPtr pSection );  // Parse from <LoDLevels> XML

    int size() const;
    const DataLoDLevel & getLevel( int i ) const;
    DataLoDLevel * find( const std::string & label );
    bool findLevel( int & level, DataSectionPtr pSection ) const;

    bool needsMoreDetail( int level, float priority ) const;
    bool needsLessDetail( int level, float priority ) const;

private:
    DataLoDLevel level_[ MAX_DATA_LOD_LEVELS + 1 ];  // 7 slots
    int size_;  // Defaults to 1 (one catch-all level)
};
```

Levels are parsed from `<LoDLevels>` sections in `.def` files. After parsing, they are sorted by ascending `start()` distance and finalised with hysteresis thresholds. Default hysteresis is 10 metres if not specified in XML:

```cpp
float hyst = (*iter)->readFloat( "hyst", 10.f );
```

### XML Configuration Example (BigWorld)

A typical BigWorld `.def` file with LOD levels:

```xml
<LoDLevels>
    <level> 50
        <hyst> 10 </hyst>
        <label> NEAR </label>
    </level>
    <level> 150
        <hyst> 20 </hyst>
        <label> MEDIUM </label>
    </level>
    <level> 500
        <hyst> 50 </hyst>
        <label> FAR </label>
    </level>
</LoDLevels>
```

---

## Property Assignment

Each property is assigned to a LOD level via the `DataDescription.detailLevel_` member.

From `data_description.hpp`:

```cpp
class DataDescription
{
public:
    int detailLevel() const;
    void detailLevel( int level );

private:
    int detailLevel_;   // Initialised to DataLoDLevel::NO_LEVEL (-1)
    // ...
};
```

The detail level is initialised to `DataLoDLevel::NO_LEVEL` (-1) in the constructor. Properties in a `.def` file can reference a LOD level by label:

```xml
<Properties>
    <health>
        <Type> INT32 </Type>
        <Flags> OTHER_CLIENTS </Flags>
        <DetailLevel> NEAR </DetailLevel>    <!-- Only sent within 50m -->
    </health>
    <guildName>
        <Type> STRING </Type>
        <Flags> OTHER_CLIENTS </Flags>
        <DetailLevel> MEDIUM </DetailLevel>  <!-- Only sent within 150m -->
    </health>
</Properties>
```

Properties without a `<DetailLevel>` tag get `OUTER_LEVEL` (-2), meaning they are always sent regardless of distance. The `DataLoDLevels::findLevel()` method resolves label strings to indices:

```cpp
bool DataLoDLevels::findLevel( int & level, DataSectionPtr pSection ) const
{
    if (pSection)
    {
        const std::string label = pSection->asString();
        for (int i = 0; i < size_-1; ++i)
        {
            if (label == level_[ i ].label())
            {
                level = level_[ i ].index();
                return true;
            }
        }
        level = 0;
        ERROR_MSG( "DataLoDLevels:findLevel: Did not find '%s'\n", label.c_str() );
    }
    else
    {
        level = DataLoDLevel::OUTER_LEVEL;  // No section = always send
        return true;
    }
    return false;
}
```

---

## VolatileInfo

VolatileInfo controls how frequently an entity's position and orientation are sent to observers. It is separate from property LOD but works alongside it -- volatile info determines spatial update frequency while LOD determines which property values are included.

### Class Definition

From `volatile_info.hpp`:

```cpp
class VolatileInfo
{
public:
    VolatileInfo( float positionPriority = -1.f,
                  float yawPriority = -1.f,
                  float pitchPriority = -1.f,
                  float rollPriority = -1.f );

    bool parse( DataSectionPtr pSection );

    bool shouldSendPosition() const { return positionPriority_ > 0.f; }
    int dirType( float priority ) const;

    bool isLessVolatileThan( const VolatileInfo & info ) const;
    bool isValid() const;
    bool hasVolatile( float priority ) const;

    static const float ALWAYS;  // = FLT_MAX

    float positionPriority() const;
    float yawPriority() const;
    float pitchPriority() const;
    float rollPriority() const;

private:
    float asPriority( DataSectionPtr pSection ) const;

    float positionPriority_;    // Squared distance threshold for position updates
    float yawPriority_;         // Squared distance threshold for yaw
    float pitchPriority_;       // Squared distance threshold for pitch
    float rollPriority_;        // Squared distance threshold for roll
};
```

### Priority Semantics

Priority values are **squared distances** stored internally. A value of `-1.0f` means "never send". The special constant `ALWAYS` (`FLT_MAX`) means "always send regardless of distance". When parsing from XML, the raw distance is squared:

```cpp
float VolatileInfo::asPriority( DataSectionPtr pSection ) const
{
    if (pSection)
    {
        float value = pSection->asFloat( -1.f );
        return (value == -1.f) ? ALWAYS : value * value;
    }
    return -1.f;  // No section = never send
}
```

### Direction Type Resolution

The `dirType()` method determines which orientation components to send based on squared distance:

```cpp
int VolatileInfo::dirType( float priority ) const
{
    return (priority > yawPriority_) +
           (priority > pitchPriority_) +
           (priority > rollPriority_);
}
```

| Return Value | Meaning |
|:---:|---|
| 0 | Send yaw, pitch, and roll |
| 1 | Send yaw and pitch only |
| 2 | Send yaw only |
| 3 | Send no direction data |

Direction priorities must be in descending order (`yaw >= pitch >= roll`). The `isValid()` method enforces this constraint.

### Volatile Check

```cpp
bool VolatileInfo::hasVolatile( float priority ) const
{
    return (priority < positionPriority_) || (priority < yawPriority_);
}
```

An entity has volatile data to send if the observer's distance is within either the position or yaw threshold.

---

## PropertyEventStamps

The `PropertyEventStamps` class tracks the last event number when each property was modified. This is used for change detection -- when an observer transitions to a more detailed LOD level, the system checks stamps to determine which properties have changed since the observer last received data at that detail level.

### Class Definition

From `property_event_stamps.hpp`:

```cpp
class PropertyEventStamps
{
public:
    void init( const EntityDescription & entityDescription );
    void init( const EntityDescription & entityDescription,
               EventNumber lastEventNumber );

    void set( const DataDescription & dataDescription,
              EventNumber eventNumber );

    EventNumber get( const DataDescription & dataDescription ) const;

    void addToStream( BinaryOStream & stream ) const;
    void removeFromStream( BinaryIStream & stream );

private:
    typedef std::vector< EventNumber > Stamps;
    Stamps eventStamps_;
};
```

### Initialization

The stamps vector is sized to match the entity's stamped property count. All stamps default to `1`:

```cpp
void PropertyEventStamps::init(
        const EntityDescription & entityDescription )
{
    eventStamps_.resize( entityDescription.numEventStampedProperties(), 1 );
}
```

An overload allows initializing all stamps to a specific event number for newly-entering observers:

```cpp
void PropertyEventStamps::init(
        const EntityDescription & entityDescription, EventNumber number )
{
    this->init( entityDescription );
    // Set all stamps to the given number
    for (auto iter = eventStamps_.begin(); iter != eventStamps_.end(); ++iter)
    {
        (*iter) = number;
    }
}
```

### Per-Property Access

Each `DataDescription` has an `eventStampIndex_` that indexes into the stamps vector:

```cpp
void PropertyEventStamps::set(
        const DataDescription & dataDescription, EventNumber eventNumber )
{
    const int index = dataDescription.eventStampIndex();
    eventStamps_[ index ] = eventNumber;
}
```

---

## PropertyChange Encoding

The property change encoding system serializes individual property modifications for transmission over the network. This encoding IS used by Cimmeria for property synchronization, unlike the LOD system described above.

### Constants

From `property_change.hpp`:

```cpp
typedef uint8 PropertyChangeType;

const PropertyChangeType PROPERTY_CHANGE_TYPE_SINGLE = 0;
const PropertyChangeType PROPERTY_CHANGE_TYPE_SLICE  = 1;

const int MAX_SIMPLE_PROPERTY_CHANGE_ID = 60;
const int PROPERTY_CHANGE_ID_SINGLE     = 61;
const int PROPERTY_CHANGE_ID_SLICE      = 62;
```

### Encoding Tiers

The system uses a tiered encoding scheme based on the property's client-server index:

| Property Index Range | Message ID | Encoding |
|---|---|---|
| 0--60 (top-level, no nesting) | Index value (1 byte) | No additional path data needed |
| > 60 or nested property | 61 (`PROPERTY_CHANGE_ID_SINGLE`) | Bit-packed path encoding |
| Array slice change | 62 (`PROPERTY_CHANGE_ID_SLICE`) | Bit-packed path + start/end indices |

The first 61 top-level properties (indices 0--60) use a **single-byte** message ID that doubles as the property identifier. This is the fast path for the most common case.

### Path Encoding for Nested Properties

For properties beyond index 60 or for changes to nested structures (arrays, dicts), a bit-packed path is encoded:

```cpp
uint8 PropertyChange::addPathToStream( BinaryOStream & stream,
        const PropertyOwnerBase * pOwner, int clientServerID ) const
{
    if (clientServerID == -1)
    {
        // Internal server streaming: simple path
        this->writePathSimple( stream );
        this->addExtraBits( stream );
    }
    else if ((clientServerID <= MAX_SIMPLE_PROPERTY_CHANGE_ID) && path_.empty())
    {
        // Fast path: return index as message ID, no extra encoding
        ret = uint8(clientServerID);
    }
    else
    {
        // Complex path: bit-packed encoding
        ret = PROPERTY_CHANGE_ID_SINGLE;
        BitWriter bits;

        // Walk path from root to leaf, encoding each index
        for (int i = path_.size()-1; i >= 0; --i)
        {
            bits.add( 1, 1 );  // "not done yet" flag
            bits.add( bitsRequired( numProperties ), streamedIndex );
            // ...
        }
        bits.add( 1, 0 );  // "stop" flag
        // ...
    }
}
```

The `bitsRequired()` function computes `ceil(log2(numValues))` using the x86 BSR (Bit Scan Reverse) instruction for efficiency:

```cpp
int bitsRequired( int numValues )
{
    if (numValues <= 1) return 0;
    numValues--;
    register int nbits;
    _asm bsr eax, numValues
    _asm mov nbits, eax
    return nbits+1;
}
```

### ChangePath

The `ChangePath` is a vector of `int32` indices ordered **leaf-to-root**:

```cpp
// A sequence of child indexes ordered from the leaf to the root (i.e. entity).
// For example, 3,4,6 would be the 6th property of the entity, the 4th "child"
// of that property and then the 3rd "child".
// E.g. If the 6th property is a list of lists called myList, this refers
// to entity.myList[4][3]
typedef std::vector< int32 > ChangePath;
```

### SinglePropertyChange

Represents a single value change:

```cpp
class SinglePropertyChange : public PropertyChange
{
public:
    SinglePropertyChange( int leafIndex, const DataType & type );

    // Adds value + path to stream, returns message ID to use
    virtual uint8 addToStream( BinaryOStream & stream,
            const PropertyOwnerBase * pOwner, int messageID ) const;

private:
    int leafIndex_;       // Index of the changing property
    PyObjectPtr pValue_;  // New value
};
```

### SlicePropertyChange

Represents a slice replacement in an array property:

```cpp
class SlicePropertyChange : public PropertyChange
{
public:
    SlicePropertyChange( Py_ssize_t startIndex, Py_ssize_t endIndex,
            const std::vector< PyObjectPtr > & newValues,
            const DataType & type );

private:
    int32 startIndex_;    // First index to replace
    int32 endIndex_;      // One past last index to replace
    const std::vector< PyObjectPtr > & newValues_;
};
```

Slice changes always use message ID `PROPERTY_CHANGE_ID_SLICE` (62). The start and end indices are bit-packed relative to the original array size.

---

## SGW Status: Not Used

### Evidence

1. **All `<LoDLevels>` tags are empty.** Every SGW `.def` file that declares LOD levels provides an empty block:

```xml
<!-- entities/defs/SGWPlayer.def -->
<LoDLevels>
</LoDLevels>

<!-- entities/defs/SGWEntity.def -->
<LoDLevels>
</LoDLevels>

<!-- entities/defs/SGWSpawnSet.def -->
<LoDLevels>
</LoDLevels>

<!-- entities/defs/SGWSpawnableEntity.def -->
<LoDLevels>
</LoDLevels>
```

With no levels defined, `DataLoDLevels::size_` remains at 1 (the default catch-all level). All properties have `detailLevel_` set to `DataLoDLevel::OUTER_LEVEL` (-2), meaning they are always sent at full detail regardless of observer distance.

2. **Client binary has no LOD entity functions.** The SGW client executable (`SGW.exe`) contains no references to `DataLoDLevel`, `VolatileInfo`, or `EventStamp` functions. The client only implements UE3/CME rendering LOD for mesh and texture detail based on camera distance.

3. **Draw distance slider is UE3 rendering.** The client's draw distance slider controls the Unreal Engine 3 rendering pipeline, not BigWorld's entity property LOD system. This is a CME/UE3 feature that determines when 3D models are culled from rendering, not when property data stops being sent from the server.

### Implications

- All properties flagged `OTHER_CLIENTS` are sent to every observer in AoI at full detail, regardless of distance.
- There is no server-side bandwidth reduction based on observer proximity for entity properties.
- Position and orientation updates are sent based on the default VolatileInfo (no configured thresholds), meaning all entities in AoI receive full volatile updates.
- This was likely acceptable for SGW's expected concurrent player counts. The game uses relatively small instanced/persistent spaces (16 spaces at startup) with moderate player density, making entity property LOD unnecessary.

### What IS Used

The `PropertyChange` encoding system (Section "PropertyChange Encoding" above) IS used by Cimmeria for all entity property synchronization. The `MAX_SIMPLE_PROPERTY_CHANGE_ID = 60` constant and the 1-byte / 2-byte encoding tiers are active in the Cimmeria protocol. See the entity-property-sync findings for protocol-level details.

---

## Related Documents

- [Entity Property Synchronization Protocol](../protocol/entity-property-sync.md) -- property change encoding in the Cimmeria protocol
- [Entity Property Sync Findings](../reverse-engineering/findings/entity-property-sync.md) -- RE findings on property sync
- [BigWorld Architecture](bigworld-architecture.md) -- distributed entity model overview
- [Entity Type Catalog](entity-type-catalog.md) -- complete list of SGW entity types and their `.def` files
- [Space Management](space-management.md) -- AoI and spatial partitioning (context for when LOD would apply)
