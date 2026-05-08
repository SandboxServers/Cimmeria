#pragma once

#include <mercury/message.hpp>

namespace Mercury
{

// Protocol version sent by the BaseApp/CellApp during handshake
// Increase when messages are changed
const uint32_t BaseCellProtocolVersion = 391;

// Magic value sent by the BaseApp in the handshake message
// (CRC32 of "Atrea.BaseApp")
const uint32_t BaseAppHandshakeHash = 0xD293BF1E;

// Magic value sent by the CellApp in the handshake message
// (CRC32 of "Atrea.CellApp")
const uint32_t CellAppHandshakeHash = 0x70CDB965;

// Max number of cacheable property sets stored on the Base
// *ONLY* change this value if really necessary, as this increases
// the resource consumption of cache stamps and visibility lists.
const uint32_t MaxPropertySets = 2;

// First entity ID assigned to CellApps
// Each CellApp has a unique range from which it can assign approx. ~2 million IDs
// Currently this is 0x10000000 + (CellApp ID << 21)
// IDs above 0x40000000 are allocated by the client
const uint32_t MinLocalEntityId = 0x10000000;

// Returns the first entity ID assigned to the specified CellApp
inline uint32_t MinCellEntityId(uint16_t cellId)
{
	return MinLocalEntityId + (cellId << 21);
}

// Returns the last entity ID assigned to the specified CellApp
inline uint32_t MaxCellEntityId(uint16_t cellId)
{
	return MinLocalEntityId + ((cellId + 1) << 21) - 1;
}

// Checks if the entity ID belongs to the BaseApp
inline bool IsBaseEntity(uint32_t entityId)
{
	return entityId < MinLocalEntityId;
}

// Checks if the entity ID belongs to the specified CellApp
inline bool IsCellEntity(uint32_t entityId, uint16_t cellId)
{
	return entityId >= MinCellEntityId(cellId) && entityId <= MaxCellEntityId(cellId);
}

enum BaseCellMessageId
{
	/*
	 * 0x00 Cell -> Base Login
	 * Sent by the Cell when opening a channel to the Base
	 *
	 * uint32    Handshake Hash
	 * uint32    Protocol Version
	 * uint32    Cell Instance ID
	 */
	CELL_BASE_AUTHENTICATE = 0x00,
	/*
	 * 0x01 Entity Create ACK
	 * Sent by the Cell to acknowledge an entity creation request
	 *
	 * uint32    Entity ID
	 * uint32    Space ID   - (0xffffffff = creation failed)
	 */
	CELL_BASE_ENTITY_CREATE_ACK = 0x01,
	/*
	 * 0x02 Entity Delete ACK
	 * Sent by the Cell to acknowledge an entity deletion request
	 *
	 * uint32    Entity ID
	 */
	CELL_BASE_ENTITY_DELETE_ACK = 0x02,
	/*
	 * 0x03 Space Data
	 * Propagates space availability information for a specific space.
	 *
	 * uint32    Space ID
	 * uint32    Cell ID   (0xffffffff = space is deleted)
	 * string    World Name
	 */
	CELL_BASE_SPACE_DATA = 0x03,
	/*
	 * 0x06 Create Cell Player
	 * Requests the BaseApp to create a cell player for the client
	 *
	 * uint32    Entity ID
	 * uint32    Space ID
	 * float     LocationX  - Player's location
	 * float     LocationY
	 * float     LocationZ
	 * float     Yaw
	 * float     Pitch
	 * float     Roll
	 */
	CELL_BASE_CREATE_CELL_PLAYER = 0x04,
	/*
	 * 0x05 Entity Disconnect ACK
	 *
	 * uint32    Entity ID
	 */
	CELL_BASE_DISCONNECT_ENTITY_ACK = 0x05,
	/*
	 * 0x06 Tick
	 * Informs the BaseApp of the current time on the Cell
	 *
	 * uint64    Time
	 */
	CELL_BASE_TICK = 0x06,
	/*
	 * 0x07 Enter AoI
	 * 
	 * uint32    Witness ID  - Entity witnessing the event (0xffffffff = all)
	 * uint32    Space ID    - Space the event occurred in
	 * uint32    Entity ID   - Entity that entered the AoI
	 * uint8     Class ID    - Class of entity
	 * Vec3      Location
	 * Vec3      Rotation
	 */
	CELL_BASE_ENTER_AOI = 0x07,
	/*
	 * 0x08 Leave AoI
	 *
	 * uint32    Space ID    - Space the event occurred in
	 * uint32    Entity ID   - Entity that left the AoI
	 * uint8     Delete      - Delete entity or only hide it on the client?
	 */
	CELL_BASE_LEAVE_AOI = 0x08,
	/*
	 * 0x09 Move
	 *
	 * uint32    Space ID    - Space the event occurred in
	 * uint32    Entity ID   - Entity that moved
	 * Vec3      Location
	 * Vec3      Rotation
	 * Vec3      Velocity
	 * uint8     Flags
	 * uint8     ServerFlags
	 */
	CELL_BASE_MOVE = 0x09,
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
	/*
	 * 0x0B Restore Entity ACK
	 *
	 * uint32    Entity ID   - Entity to restore
	 */
	CELL_BASE_RESTORE_ENTITY_ACK = 0x0B,
	/*
	 * 0x0C Switch Space
	 *
	 * uint32    Entity ID   - Entity to move
	 * uint32    Space ID    - Destination space (Instanced = 0xffffffff)
	 * string    World Name  - Destination world name (used for instanced spaces)
	 * Vec3      Position
	 * Vec3      Rotation
	 */
	CELL_BASE_SWITCH_SPACE = 0x0C,
	/*
	 * 0x0D BaseApp Message
	 *
	 * uint32    Entity ID   - Recipient entity
	 * uint8     Message ID  - Type ID of RPC message to the BaseApp
	 * uint8[]   Message
	 */
	CELL_BASE_BASEAPP_MESSAGE = 0x0D,
	/*
	 * 0x0E Client Message
	 *
	 * uint32    Entity ID   - Recipient entity
	 * uint8     Message ID  - Type ID of exposed RPC message to the Client
	 * uint8[]   Message
	 */
	CELL_BASE_CLIENT_MESSAGE = 0x0E,
	/*
	 * 0x0F Create Cell Entity
	 *
	 * uint32    Entity ID   - Created entity
	 * uint32    Space  ID   - Which space is the entity on
	 * uint8     Class  ID   - Entity type
	 * uint8     Flags       - Cell entity flags
	 * Vec3      Position
	 */
	CELL_BASE_NOTIFY_CREATE_CELL_ENTITY = 0x0F,
	/*
	 * 0x10 Delete Cell Entity
	 *
	 * uint32    Entity ID   - Deleted entity
	 */
	CELL_BASE_NOTIFY_DELETE_CELL_ENTITY = 0x10,
	/*
	 * 0x11 Update Cache Stamp
	 *
	 * uint32    Entity ID   - Entity to update
	 * uint32    PropSet ID  - Property set to update
	 * uint8     Invalidate  - Invalidate current cache for this property set?
	 * Message[] Messages    - Entity messages in this cache stamp
	 *     uint8  Message ID - RPC message ID
	 *     uint8  Flags      - Message flags (INCREMENTAL_UPDATABLE)
	 *     uint16 Length     - Length of message
	 *     byte[] Args       - Contents of message
	 */
	CELL_BASE_UPDATE_CACHE_STAMP = 0x11,
	


	/*
	 * 0x00 Authenticate ACK
	 *
	 * uint32    Handshake Hash
	 * uint32    Protocol Version
	 * uint32    Result Code (0 = success)
	 * uint32    Current BaseApp Ticks
	 * uint32    Tickrate
	 */
	BASE_CELL_AUTHENTICATE_ACK = 0x00,
	/*
	 * 0x01 Create Entity
	 * 
	 * uint32    Space ID  (0xffffffff = create new space)
	 * uint32    Entity ID
	 * uint32    DBID      (0xffffffff = don't load)
	 * float     LocationX
	 * float     LocationY
	 * float     LocationZ
	 * float     Yaw
	 * float     Pitch
	 * float     Roll
	 * string    ClassName
	 * string    WorldName
	 * uint32    Property count
	 *    string Name
	 *    python Value
	 */
	BASE_CELL_CREATE_ENTITY = 0x01,
	/*
	 * 0x02 Destroy Entity
	 * Asks the Cell to destroy the specified entity on the cell
	 * 
	 * uint32    Entity ID
	 */
	BASE_CELL_DESTROY_ENTITY = 0x02,
	/*
	 * 0x03 Entity Connect
	 * 
	 * uint32    Entity ID
	 */
	BASE_CELL_CONNECT_ENTITY = 0x03,
	/*
	 * 0x04 Entity Disconnect
	 * 
	 * uint32    Entity ID
	 */
	BASE_CELL_DISCONNECT_ENTITY = 0x04,
	/*
	 * 0x05 Move
	 *
	 * uint32    Entity ID   - Entity that moved
	 * Vec3      Location
	 * Vec3      Rotation
	 * Vec3      Velocity
	 * uint8     Flags
	 */
	BASE_CELL_MOVE = 0x05,
	/*
	 * 0x06 Restore Entity
	 *
	 * uint32    Space ID    - Space ID of the entity (0xffffffff = create new space)
	 * uint32    Entity ID   - Entity to back up
	 * uint8     Class ID
	 * uint8     Recovery    - 1, if the entity is restored because of a CellApp crash
	 *                         0, if the entity is moved here from a different space
	 * string    World Name
	 * Vec3      Position
	 * Vec3      Rotation
	 * uint32    Object Size
	 * uint8[]   Object
	 * uint32    Python Object Size
	 * uint8[]   Python Object
	 */
	BASE_CELL_RESTORE_ENTITY = 0x06,
	/*
	 * 0x07 BaseApp Message (internal Cell RPC)
	 *
	 * uint32    Entity ID   - Recipient entity
	 * uint8[]   Message
	 */
	BASE_CELL_BASEAPP_MESSAGE = 0x07,
	/*
	 * 0x08 Client Message (exposed Cell RPC)
	 *
	 * uint32    Entity ID   - Recipient entity
	 * uint8[]   Message
	 */
	BASE_CELL_CLIENT_MESSAGE = 0x08,
	/*
	 * 0x09 Request Entity Update
	 *
	 * uint32    Witness ID  - Witness seeing the entity
	 * uint32    Entity ID   - Entity needing an update
	 * uint8     Flags       - (STATIC, UNCACHED, DYNAMIC)
	 */
	BASE_CELL_REQUEST_ENTITY_UPDATE = 0x09
};

enum DistributionFlags
{
	DISTRIBUTE_CLIENT = 0x01,
	DISTRIBUTE_LOD = 0x02,
	DISTRIBUTE_SPACE = 0x04,
	DISTRIBUTE_RECIPIENT = 0x08
};

enum RegistrationResponseCode
{
	REGISTRATION_OK = 0x00,
	REGISTRATION_FAILED = 0x01,
	REGISTRATION_HASH_MISMATCH = 0x02,
	REGISTRATION_PROTOCOL_MISMATCH = 0x03
};

enum EntityUpdateFlags
{
	UPDATE_STATIC = 0x01,
	UPDATE_DYNAMIC = 0x02,
	UPDATE_UNCACHED = 0x04
};

enum CellEntityFlags
{
	ENTITY_NOT_CACHED = 0x01,
	ENTITY_HAS_DYNAMIC = 0x02
};

}
