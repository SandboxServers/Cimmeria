#pragma once

#include <mercury/message_handler.hpp>
#include <mercury/message.hpp>

namespace SGW
{

class EntityMessageHandler : public Mercury::MessageHandler
{
	public:
		EntityMessageHandler();
		virtual ~EntityMessageHandler();

		virtual void onMessage(Mercury::Message::Ptr msg) override final;
		
		// Handle normal messages (x < 0x80)
		virtual void onBaseMessage(Mercury::Message::Ptr msg) = 0;
		// Handle entity messages with entityId (0x80 <= x < 0xC0)
		virtual void onEntityMessage(uint8_t rpcMessageId, uint32_t entityId, Mercury::Message::Ptr msg) = 0;
		// Handle entity messages with entityId (0xC0 <= x)
		virtual void onBaseEntityMessage(uint8_t rpcMessageId, Mercury::Message::Ptr msg) = 0;
};

}
