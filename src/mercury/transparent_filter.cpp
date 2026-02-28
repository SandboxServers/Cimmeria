#include <stdafx.hpp>
#include <mercury/transparent_filter.hpp>

namespace Mercury {
	

TransparentFilter::TransparentFilter()
{
}

TransparentFilter::~TransparentFilter()
{
}

bool TransparentFilter::sendMessage(Packet & packet, Mercury::MemoryStream & buffer)
{
	packet.seek(0);
	buffer.seek(0);
	buffer.resize(packet.size());
	packet.copyTo(buffer, packet.size());
	return true;
}

bool TransparentFilter::receiveMessage(ReceivedPacket & packet)
{
	return true;
}

uint32_t TransparentFilter::addedSize()
{
	// Filter does not add any data
	return 0;
}


}
