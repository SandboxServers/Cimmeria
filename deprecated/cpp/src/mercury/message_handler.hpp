#pragma once

#include <mercury/message.hpp>
#include <mercury/channel.hpp>

namespace Mercury
{

class MessageHandler
{
	public:
		MessageHandler();
		virtual ~MessageHandler();

		void update_channel(BaseChannel::Ptr channel);

		inline BaseChannel::Ptr channel()
		{
			return channel_;
		}

		virtual void onMessage(Message::Ptr msg) = 0;
		virtual void onConnected() = 0;
		virtual void onDisconnected() = 0;
		virtual void tick(uint64_t time) = 0;

		virtual Message::Table const & sendMessages() = 0;
		virtual Message::Table const & receiveMessages() = 0;

	protected:
		BaseChannel::Ptr channel_;
};

}
