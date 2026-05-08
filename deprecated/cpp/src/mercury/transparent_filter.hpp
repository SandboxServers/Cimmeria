#pragma once

#include <mercury/packet.hpp>
#include <mercury/channel.hpp>

namespace Mercury {

/*
 * Transparent filter: does not change the data when sending/receiving.
 */
class TransparentFilter : public MessageFilter
{
public:
	TransparentFilter();
	~TransparentFilter();

	virtual bool sendMessage(Packet & packet, Mercury::MemoryStream & buffer);
	virtual bool receiveMessage(ReceivedPacket & packet);
	virtual uint32_t addedSize();
};

}
