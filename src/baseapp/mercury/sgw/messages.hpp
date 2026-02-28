#pragma once

#include <mercury/message.hpp>

namespace Mercury
{
namespace SGW
{

enum MessageId
{
	BASEMSG_AUTHENTICATE = 0x00,
	BASEMSG_BANDWIDTH_NOTIFICATION = 0x01,
	BASEMSG_UPDATE_FREQUENCY_NOTIFICATION = 0x02,
	BASEMSG_SET_GAME_TIME = 0x03,
	BASEMSG_RESET_ENTITIES = 0x04,
	BASEMSG_CREATE_BASE_PLAYER = 0x05,
	BASEMSG_CREATE_CELL_PLAYER = 0x06,
	BASEMSG_SPACE_DATA = 0x07,
	BASEMSG_SPACE_VIEWPORT_INFO = 0x08,
	BASEMSG_CREATE_ENTITY = 0x09,
	BASEMSG_UPDATE_ENTITY = 0x0A,
	BASEMSG_ENTITY_INVISIBLE = 0x0B,
	BASEMSG_LEAVE_AOI = 0x0C,
	BASEMSG_TICK_SYNC = 0x0D,
	BASEMSG_SET_SPACE_VIEWPORT = 0x0E,
	BASEMSG_SET_VEHICLE = 0x0F,
	BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YAW_PITCH_ROLL = 0x10,
	BASEMSG_DETAILED_POSITION = 0x30,
	BASEMSG_FORCED_POSITION = 0x31,
	BASEMSG_CONTROL_ENTITY = 0x32,
	BASEMSG_VOICE_DATA = 0x33,
	BASEMSG_RESTORE_CLIENT = 0x34,
	BASEMSG_RESTORE_BASEAPP = 0x35,
	BASEMSG_RESOURCE_FRAGMENT = 0x36,
	BASEMSG_LOGGED_OFF = 0x37,
	BASEMSG_ENTITY_MESSAGE = 0x38,
	// This is a special reply message to BaseApp connection requests
	BASEMSG_REPLY_MESSAGE = 0xFF,

	CLIENTMSG_BASEAPP_LOGIN = 0x00,
	CLIENTMSG_AUTHENTICATE = 0x01,
	CLIENTMSG_AVATAR_UPDATE_IMPLICIT = 0x02,
	CLIENTMSG_AVATAR_UPDATE_EXPLICIT = 0x03,
	CLIENTMSG_AVATAR_UPDATE_WARD_IMPLICIT = 0x04,
	CLIENTMSG_AVATAR_UPDATE_WARD_EXPLICIT = 0x05,
	CLIENTMSG_SWITCH_INTERFACE = 0x06,
	CLIENTMSG_REQUEST_ENTITY_UPDATE = 0x07,
	CLIENTMSG_ENABLE_ENTITIES = 0x08,
	CLIENTMSG_VIEWPORT_ACK = 0x09,
	CLIENTMSG_VEHICLE_ACK = 0x0A,
	CLIENTMSG_RESTORE_CLIENT_ACK = 0x0B,
	CLIENTMSG_DISCONNECT = 0x0C,
	CLIENTMSG_ENTITY_MESSAGE = 0x0D
};

/*
 * Table for storing length data of client messages in the range 0x00-0x7F
 */
const unsigned int ClientMessageTableSize = 0x0D;
extern const Message::Format ClientMessageList[ClientMessageTableSize];

extern const Message::Table ClientMessageTable;


/*
 * Table for storing length data of server messages in the range 0x00-0x7F
 */
const unsigned int ServerMessageTableSize = 0x39;
extern const Message::Format ServerMessageList[ServerMessageTableSize];

extern const Message::Table ServerMessageTable;

/*******************************************************************
  Resource fragment flags
*******************************************************************/
const unsigned int RESOURCE_BASE_FLAG = 0x40;
const unsigned int RESOURCE_INITIAL_FRAGMENT = 0x01;
const unsigned int RESOURCE_FINAL_FRAGMENT = 0x02;

}
}
