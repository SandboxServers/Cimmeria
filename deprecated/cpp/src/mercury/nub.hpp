#pragma once

#include <mercury/packet.hpp>
#include <mercury/channel.hpp>
#include <mercury/timer.hpp>
#include <boost/asio/deadline_timer.hpp>

namespace Mercury
{

class ClientlessPacketHandler
{
public:
	virtual void handlePacket(class Nub & nub, ReceivedPacket::Ptr packet, boost::asio::ip::udp::endpoint & endpoint) = 0;
};

class Nub
{
	public:
		static const uint32_t TxSendQueueMaxSize = 1024;
		static const uint32_t MaxPacketLength = 1472;
		typedef boost::asio::ip::udp::endpoint Address;
		typedef TimerMgr<uint64_t> Timer;

		Nub(boost::asio::io_service & service, unsigned short port, ClientlessPacketHandler * clientlessHandler);
		~Nub();

		Timer & timers();
		uint64_t lastTick() const;
		void setTickInterval(uint32_t interval);

		void connectChannel(BaseChannel::Ptr channel, Address const & ep);
		void unregisterChannel(class BaseChannel * channel, Address const & ep);
		void condemn(BaseChannel::Ptr channel);
		BaseChannel::Ptr findChannel(Address const & endpoint);

		void queuePacket(BaseChannel::Ptr channel, Packet::Ptr packet, Packet::SequenceId sequenceId);

	private:
		boost::asio::ip::udp::socket socket_;
		// Packet being received
		ReceivedPacket::Ptr receivedPacket_;
		// Address we received the packet from
		Address receivedEndpoint_;
		// List of channels that we manage
		std::map<Address, BaseChannel::Ptr> channels_;
		// Channel deleter
		class CondemnedChannels * condemned_;

		// Nub resend timers
		Timer timers_;

		// How often should we tick registered channels
		uint32_t tickInterval_;

		// System time when the last tick() call was made 
		uint64_t lastTick_;

		boost::asio::deadline_timer m_timer;
		ClientlessPacketHandler * clientlessHandler_;

		struct PendingPacket
		{
			Packet::Ptr packet;
			Packet::SequenceId sequenceId;
			BaseChannel::Ptr channel;
		};

		// Packets awaiting send
		boost::circular_buffer<PendingPacket> sendQueue_;

		// Are we sending a packet?
		bool isSending_;

		// Temp send buffer for current packet
		Mercury::MemoryStream sendBuffer_;
		
		void sendPacket(BaseChannel::Ptr channel, Packet::Ptr packet, Packet::SequenceId sequenceId);
		void onPacketSent(BaseChannel::Ptr channel, Packet::Ptr packet, const boost::system::error_code & errcode);

		void frameHandler(const boost::system::error_code & error, size_t length, Address & endpoint, ReceivedPacket::Ptr receive_buf);
		void receiveFrames();
		void tick(const boost::system::error_code & error);
};

}

