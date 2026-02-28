#include <stdafx.hpp>
#include <baseapp/mercury/sgw/messages.hpp>

namespace Mercury
{
namespace SGW
{

const Message::Table ClientMessageTable = {
	ClientMessageTableSize, ClientMessageList,
	{Message::WORD_LENGTH,		0,	"CLIENT_MESSAGE", true}
};

/*
 * Table for storing length data of client messages in the range 0x00-0x7F
 */
const Message::Format ClientMessageList[ClientMessageTableSize] = 
{
	/*
	 * 0x00 BaseApp Login
	 * Send by the client when opening a channel to the BaseApp
	 *
	 * uint32    Account ID
	 * byte[32]  AES key
	 */
	{Message::WORD_LENGTH,			0,	"BASEAPP_LOGIN", false},
	/*
	 * 0x01 Authenticate
	 * Sent by the client in each frame to authenticate itself to the proxy
	 *
	 * byte[20]  Ticket ID
	 */
	{Message::WORD_LENGTH,			0,	"AUTHENTICATE", false},
	/*
	 * 0x02 Avatar Update Implicit
	 * Not used by SGW
	 */
	{Message::CONSTANT_LENGTH,		36,	"AVATAR_UPD_IMPLICIT", true},
	/*
	 * 0x03 Avatar Update Explicit
	 * Client controlled entity location update - this is the one used by SGW
	 * 
	 * uint32    Space ID
	 * uint32    Vehicle ID
	 * Vec3      Position
	 * Vec3      Velocity
	 * Byte3     Direction
	 * uint8     Flags (0x01 = ?, 0x02 = in air?)
	 * uint8     X Cell
	 * uint8     Y Cell
	 * uint8     Z Cell
	 */
	{Message::CONSTANT_LENGTH,		40,	"AVATAR_UPD_EXPLICIT", true},
	/*
	 * 0x04 Avatar Update Ward Implicit
	 * Not used by SGW
	 */
	{Message::CONSTANT_LENGTH,		36,	"AVATAR_UPDW_IMPLICIT", true},
	/*
	 * 0x05 Avatar Update Ward Explicit
	 * Not used by SGW
	 */
	{Message::CONSTANT_LENGTH,		40,	"AVATAR_UPDW_EXPLICIT", true},
	/*
	 * 0x06 Switch Interface
	 * Deprecated in BW, Not used by SGW
	 */
	{Message::CONSTANT_LENGTH,		0,	"SWITCH_INTERFACE", false},
	/*
	 * 0x07 Request Entity Update
	 * Client requests an update for an entity that entered its AoI
	 * (Server should ignore this after CreateEntity)
	 *
	 * uint32    Entity ID
	 */
	{Message::WORD_LENGTH,			0,	"REQ_ENTITY_UPDATE", true},
	/*
	 * 0x08 Enable Entities
	 * Notifies the server that the client entity system is reset
	 *
	 * uint64    Dummy
	 */
	{Message::CONSTANT_LENGTH,		8,	"ENABLE_ENTITIES", true},
	/*
	 * 0x09 Viewport ACK
	 * Acknowledgement for server viewport update
	 *
	 * ???
	 */
	{Message::CONSTANT_LENGTH,		8,	"VIEWPORT_ACK", false},
	/*
	 * 0x0A Vehicle ACK
	 * Acknowledgement for vehicle update
	 *
	 * ???
	 */
	{Message::CONSTANT_LENGTH,		8,	"VEHICLE_ACK", false},
	/*
	 * 0x0B Restore Client ACK
	 * Acknowledgement for restore client request
	 *
	 * uint32    ID
	 */
	{Message::WORD_LENGTH,			0,	"RESTORE_CLIENT_ACK", false},
	/*
	 * 0x0C Disconnect
	 * Client requests a disconnection from server
	 */
	{Message::CONSTANT_LENGTH,		1,	"DISCONNECT", false}
};

const Message::Table ServerMessageTable = {
	ServerMessageTableSize, ServerMessageList,
	{Message::WORD_LENGTH,		0,	"SERVER_MESSAGE", true}
};

/*
 * Table for storing length data of server messages in the range 0x00-0x7F
 */
const Message::Format ServerMessageList[ServerMessageTableSize] = 
{
	/*
	 * 0x00 Authenticate
	 *
	 * Never seen this frame
	 */
	{Message::DWORD_LENGTH,			0,	"AUTHENTICATE", false},
	/*
	 * 0x01 Bandwidth Notification
	 * Updates the max bandwidth allowed; not used by SGW since it doesn't have a bandwidth mutator.
	 * 
	 * uint32 Bandwidth in bps (bits per sec or bytes per sec?)
	 */
	{Message::CONSTANT_LENGTH,		4,	"BANDWIDTH_NOTIFICATION", false},
	/*
	 * 0x02 Update Frequency Notification
	 * Set resolution of the server's internal clock (ticks per second)
	 * Usually set to 10.
	 * 
	 * uint8  Resolution
	 */
	{Message::CONSTANT_LENGTH,		1,	"UPD_FREQ_NOTICIATION", false},
	/*
	 * 0x03 Set Game Time
	 * Set ingame time; sent during init
	 * Value is game time in the resolution specified by UPD_FREQ_NOTICIATION
	 * 
	 * uint32 Game time
	 */
	{Message::CONSTANT_LENGTH,		4,	"SET_GAME_TIME", false},
	/*
	 * 0x04 Reset Entities
	 * Sent when tearing down the entity system
	 * 
	 * bool   Keep base player entities or not (should be 0)
	 */
	{Message::CONSTANT_LENGTH,		1,	"RESET_ENTITIES", false},
	/*
	 * 0x05 Create Base Player
	 * Notifies the client that the base part of the player entity was created
	 *
	 * uint32 EntityID    - ID of the entity to create
	 * uint8  ClassID     - Index into class table
	 * uint8  Properties  - Amount of python properties (zero)
	 */
	{Message::WORD_LENGTH,			0,	"CREATE_BASE_PLAYER", false},
	/*
	 * 0x06 Create Cell Player
	 * Creates the player entity on the CellApp
	 *
	 * uint32 InstanceID - World instance ID
	 * uint32 VehicleID  - Zero
	 * float  LocationX  - Player's location
	 * float  LocationY
	 * float  LocationZ
	 * float  Yaw
	 * float  Pitch
	 * float  Roll
	 */
	{Message::WORD_LENGTH,			0,	"CREATE_CELL_PLAYER", false},
	/*
	 * 0x07 Space Data
	 * UNUSED - Only used in older builds, newer versions use SGWPlayer.onClientMapLoad
	 *
	 * uint32    SpaceID
	 * uint32    SpaceEntryID
	 * uint16    Key
	 * char[]    Value
	 */
	{Message::WORD_LENGTH,			0,	"SPACE_DATA", false},
	/*
	 * 0x08 Space Viewport Info
	 * Send information about controlled entity & space
	 * 
	 * uint32 EntityID   - The client's controlling entity ID, aka the player
	 * uint32 EntityID2  - Same as the previous one; maybe Viewport Entity ID?
	 * uint32 SpaceID    - Identifier of this world space; not the same as World ID!
	 * uint8  ViewportID - Index of the current viewport (0)
	 *
	 * Open/override viewport: Set all values to nonzero
	 * Close viewport: Set EntityID2 and SpaceID to zero.
	 */
	{Message::CONSTANT_LENGTH,		13,	"VIEWPORT_INFO", false},
	/*
	 * 0x09 Create Entity
	 *
	 * Creates a base and cell entity
	 * uint32 EntityID
	 * uint8  0xFF     - ID alias
	 * uint8  ClassID  - Index into class table
	 * uint8  Unknown  - Zero
	 * uint8  Unknown2 - Zero
	 */
	{Message::WORD_LENGTH,			0,	"CREATE_ENTITY", false},
	/*
	 * 0x0A Update Entity
	 * Updates the Python attribute dictionary of the entity.
	 * Since SGW doesn't use Python on the client, this is useless
	 */
	{Message::WORD_LENGTH,			0,	"UPDATE_ENTITY", false},
	/*
	 * 0x0B Entity Invisible
	 * Sent before an entity is destroyed (from the client's point of view)
	 *
	 * uint32 EntityID
	 * uint8  ID alias  - 0xFF means no alias
	 */
	{Message::CONSTANT_LENGTH,		5,	"ENTITY_INVISIBLE", false},
	/*
	 * 0x0C Leave AoI
	 * Entity leaves client AoI (either got too far or was destroyed)
	 *
	 * uint32   EntityID
	 * uint32[] CacheStamps  - Only stamp 0 is sent
	*/
	{Message::WORD_LENGTH,			0,	"LEAVE_AOI", false},
	/*
	 * 0x0D Tick Sync
	 * Synchronize ingame time; should be sent with a 10 Hz rate
	 * 
	 * uint32 Time     - Value is game time in the resolution specified by UPD_FREQ_NOTICIATION
	 * uint32 Duration - (THIS IS ONLY A GUESS) Duration of a game tick in milliseconds (= 100 for a 10 Hz update rate)
	 */
	{Message::CONSTANT_LENGTH,		8,	"TICK_SYNC", false},
	/*
	 * 0x0E Set Space Viewport
	 * Switch active viewport
	 *
	 * uint8  ViewportID - Viewport previously created with VIEWPORT_INFO
	 */
	{Message::CONSTANT_LENGTH,		1,	"SET_SPACE_VIEWPORT", false},
	/*
	 * 0x0F Set Vehicle
	 * Doesn't seem to be supported by SGW - UNTESTED!
	 * 
	 * uint32 EntityID   - Entity that enters/exits the vehicle
	 * uint32 VehicleID  - Vehicle the entity is in
	 */
	{Message::CONSTANT_LENGTH,		4,	"SET_VEHICLE", false},
	/*
	 * 0x10 Update Avatar No Alias Full Pos Yaw Pitch Roll
	 * Movement update (this is the only one sent by the server; others are unused)
	 * It doesn't work on client entities!
	 * 
	 * uint32  EntityID  - The entity whose location is updated
	 * float   LocationX - Location of the entity
	 * float   LocationY
	 * float   LocationZ
	 * byte[6] VelocityAndFlags - Compressed vector, see ClientHandler::packXYZ() for exact format
	 * uint8   Yaw
	 * uint8   Pitch
	 * uint8   Roll
	 */
	{Message::CONSTANT_LENGTH,		25,	"UPDATE_AVATAR", false},			// 0x10 Update Avatar No Alias Full Pos Yaw Pitch Roll
	/*
	 * Additional position update messages; each one does the same, as 0x10, their purpose is to conserve network bandwidth.
	 * (We currently don't use any of these, neither does the official server)
	 */
	{Message::CONSTANT_LENGTH,		24,	"UPDATE_AVATAR", false},			// 0x11 Update Avatar No Alias Full Pos Yaw Pitch
	{Message::CONSTANT_LENGTH,		23,	"UPDATE_AVATAR", false},			// 0x12 Update Avatar No Alias Full Pos Yaw
	{Message::CONSTANT_LENGTH,		22,	"UPDATE_AVATAR", false},			// 0x13 Update Avatar No Alias Full Pos No Dir
	{Message::CONSTANT_LENGTH,		25,	"UPDATE_AVATAR", false},			// 0x14 Update Avatar No Alias On Chunk Pos Yaw Pitch Roll
	{Message::CONSTANT_LENGTH,		24,	"UPDATE_AVATAR", false},			// 0x15 Update Avatar No Alias On Chunk Pos Yaw Pitch
	{Message::CONSTANT_LENGTH,		23,	"UPDATE_AVATAR", false},			// 0x16 Update Avatar No Alias On Chunk Pos Yaw
	{Message::CONSTANT_LENGTH,		22,	"UPDATE_AVATAR", false},			// 0x17 Update Avatar No Alias On Chunk No Dir
	{Message::CONSTANT_LENGTH,		25,	"UPDATE_AVATAR", false},			// 0x18 Update Avatar No Alias Ground Pos Yaw Pitch Roll
	{Message::CONSTANT_LENGTH,		24,	"UPDATE_AVATAR", false},			// 0x19 Update Avatar No Alias Ground Pos Yaw Pitch
	{Message::CONSTANT_LENGTH,		23,	"UPDATE_AVATAR", false},			// 0x1A Update Avatar No Alias Ground Pos Yaw
	{Message::CONSTANT_LENGTH,		22,	"UPDATE_AVATAR", false},			// 0x1B Update Avatar No Alias Ground No Dir
	{Message::CONSTANT_LENGTH,		13,	"UPDATE_AVATAR", false},			// 0x1C Update Avatar No Alias No Pos Yaw Pitch Roll
	{Message::CONSTANT_LENGTH,		12,	"UPDATE_AVATAR", false},			// 0x1D Update Avatar No Alias No Pos Yaw Pitch
	{Message::CONSTANT_LENGTH,		11,	"UPDATE_AVATAR", false},			// 0x1E Update Avatar No Alias No Pos Yaw
	{Message::CONSTANT_LENGTH,		10,	"UPDATE_AVATAR", false}, 			// 0x1F Update Avatar No Alias No Pos No Dir
	{Message::CONSTANT_LENGTH,		22,	"UPDATE_AVATAR", false},			// 0x20 Update Avatar Alias Full Pos Yaw Pitch Roll
	{Message::CONSTANT_LENGTH,		21,	"UPDATE_AVATAR", false},			// 0x21 Update Avatar Alias Full Pos Yaw
	{Message::CONSTANT_LENGTH,		20,	"UPDATE_AVATAR", false},			// 0x22 Update Avatar Alias Full Pos Yaw Pitch
	{Message::CONSTANT_LENGTH,		19,	"UPDATE_AVATAR", false},			// 0x23 Update Avatar Alias Full Pos No Dir
	{Message::CONSTANT_LENGTH,		22,	"UPDATE_AVATAR", false},			// 0x24 Update Avatar Alias On Chunk Pos Yaw Pitch Roll
	{Message::CONSTANT_LENGTH,		21,	"UPDATE_AVATAR", false},			// 0x25 Update Avatar Alias On Chunk Pos Yaw Pitch
	{Message::CONSTANT_LENGTH,		20,	"UPDATE_AVATAR", false},			// 0x26 Update Avatar Alias On Chunk Pos Yaw
	{Message::CONSTANT_LENGTH,		19,	"UPDATE_AVATAR", false},			// 0x27 Update Avatar Alias On Chunk No Dir
	{Message::CONSTANT_LENGTH,		22,	"UPDATE_AVATAR", false},			// 0x28 Update Avatar Alias Ground Pos Yaw Pitch Roll
	{Message::CONSTANT_LENGTH,		21,	"UPDATE_AVATAR", false},			// 0x29 Update Avatar Alias Ground Pos Yaw Pitch
	{Message::CONSTANT_LENGTH,		20,	"UPDATE_AVATAR", false},			// 0x2A Update Avatar Alias Ground Pos Yaw
	{Message::CONSTANT_LENGTH,		19,	"UPDATE_AVATAR", false},			// 0x2B Update Avatar Alias Ground No Dir
	{Message::CONSTANT_LENGTH,		10,	"UPDATE_AVATAR", false},			// 0x2C Update Avatar Alias No Pos Yaw Pitch Roll
	{Message::CONSTANT_LENGTH,		9,	"UPDATE_AVATAR", false},			// 0x2D Update Avatar Alias No Pos Yaw Pitch
	{Message::CONSTANT_LENGTH,		8,	"UPDATE_AVATAR", false},			// 0x2E Update Avatar Alias No Pos Yaw
	{Message::CONSTANT_LENGTH,		7,	"UPDATE_AVATAR", false},			// 0x2F Update Avatar Alias No Pos No Dir
	/*
	 * 0x30 Detailed Position
	 * Detailed position, velocity and direction update for an entity
	 * It doesn't work on client entities!
	 * 
	 * uint32  EntityID
	 * Vec3    Location
	 * Vec3    Velocity
	 * Vec3    Rotation
	 * uint8   Flags      - 0x01
	 */
	{Message::CONSTANT_LENGTH,		41,	"DETAILED_POSITION", false},
	/*
	 * 0x31 Forced Position
	 * ForcedPosition can update the location of client controlled entities (including the player).
	 * It shouldn't (and cannot) be used to update location for non-client entities!
	 * 
	 * uint32  EntityID
	 * uint32  SpaceID
	 * uint32  VehicleID
	 * Vec3    Location
	 * Vec3    Velocity
	 * Vec3    Rotation
	 * uint8   Flags      - 0x01
	 */
	{Message::CONSTANT_LENGTH,		49,	"FORCED_POSITION", false},
	/*
	 * 0x32 Control Entity
	 * Tells the client if an entity is controlled locally - UNTESTED!
	 * 
	 * uint32  EntityID
	 * bool    ControlledLocally
	 */
	{Message::CONSTANT_LENGTH,		5,	"CONTROL_ENTITY", false},
	/*
	 * 0x33 Voice Data
	 * Unused, no handler registered for SGW
	 */
	{Message::WORD_LENGTH,			0,	"VOICE_DATA", false},
	/*
	 * 0x34 Restore Client
	 * Called when the server wants to restore the client to a previous state - UNTESTED!
	 *
	 * uint32  EntityID
	 * uint32  SpaceID
	 * uint32  VehicleID
	 * Vec3    Location
	 * Vec3    Velocity
	 * Vec3    Direction
	 */
	{Message::WORD_LENGTH,			0,	"RESTORE_CLIENT", false},
	/*
	 * 0x35 Restore Base App
	 * Notification about BaseApp errors - UNTESTED!
	 */
	{Message::WORD_LENGTH,			0,	"RESTORE_BASEAPP", false},
	/*
	 * 0x36 Resource Fragment
	 *
	 * uint16  TransactionID   - Incremented for each resource
	 * uint8   FragmentID      - Index of frament in resource
	 * uint8   Flags           - 0x01 = First fragment, 0x02 = Final fragment
	 */
	{Message::WORD_LENGTH,			0,	"RESOURCE_FRAGMENT", false},
	/*
	 * 0x37 Logged Off
	 * Request client logoff
	 *
	 * uint8    Reason
	 */
	{Message::CONSTANT_LENGTH,		1,	"LOGGED_OFF", false},
	/*
	 * Client entity messages
	 */
	{Message::WORD_LENGTH,		0,	"CLIENT_MESSAGE", true}
};

}
}
