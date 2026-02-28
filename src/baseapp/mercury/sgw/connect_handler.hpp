#pragma once
#include <mercury/nub.hpp>

namespace Mercury
{
namespace SGW
{

class ConnectHandler : public ClientlessPacketHandler
{
	public:
		virtual void handlePacket(class Nub & nub, ReceivedPacket::Ptr packet, boost::asio::ip::udp::endpoint & endpoint);
};

}
}
