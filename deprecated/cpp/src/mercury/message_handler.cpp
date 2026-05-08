#include <stdafx.hpp>
#include <mercury/message_handler.hpp>

namespace Mercury
{

MessageHandler::MessageHandler()
{
}

MessageHandler::~MessageHandler()
{
}

void MessageHandler::update_channel(BaseChannel::Ptr channel)
{
	channel_ = channel;
}

}
