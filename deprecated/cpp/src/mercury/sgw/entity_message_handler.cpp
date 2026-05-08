#include <stdafx.hpp>
#include <mercury/sgw/entity_message_handler.hpp>

namespace SGW
{
	EntityMessageHandler::EntityMessageHandler() {}
	EntityMessageHandler::~EntityMessageHandler() {}

	void EntityMessageHandler::onMessage(Mercury::Message::Ptr msg)
	{
		// TODO: uint8_t!
		uint16_t messageId = msg->messageId();
		if (!(messageId & 0x80))
		{
			onBaseMessage(msg);
			return;
		}

		uint32_t entityId;
		if (!(messageId & 0x40))
			*msg >> entityId;

		uint8_t rpcMessageId = messageId & 0x3f;
		if (rpcMessageId == 0x3d)
		{
			uint8_t subMsgId;
			*msg >> subMsgId;
			rpcMessageId += subMsgId;
		}

		if (messageId & 0x40)
			onBaseEntityMessage(rpcMessageId, msg);
		else
			onEntityMessage(rpcMessageId, entityId, msg);
	}


}
