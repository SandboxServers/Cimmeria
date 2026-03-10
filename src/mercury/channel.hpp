#pragma once

#include <mercury/packet.hpp>
#include <mercury/message.hpp>
#include <mercury/bundle.hpp>
#include <mercury/timer.hpp>
#include <boost/circular_buffer.hpp>
#include <set>

namespace Mercury
{

#undef CHANNEL_DEBUG_MESSAGES

class Bundle;

/*
 * Applies transformations incoming and outgoing messages
 * (encryption, compression, ...)
 */
class MessageFilter
{
public:
	/*
	 * Transforms the packet in "packet" and writes the result to "buffer".
	 * Returns true if filtering was successful, false on failure.
	 */
	virtual bool sendMessage(Packet & packet, Mercury::MemoryStream & buffer) = 0;
	/*
	 * Transforms the packet in-place (writes the result to "packet").
	 * Returns true if filtering was successful, false on failure.
	 */
	virtual bool receiveMessage(ReceivedPacket & packet) = 0;
	/*
	 * Returns the maximal amount of bytes this transformation will add
	 * to the total packet size when sending packets.
	 */
	virtual uint32_t addedSize() = 0;
};

class BaseChannel : public std::enable_shared_from_this<BaseChannel>
{
	public:
		typedef std::shared_ptr<BaseChannel> Ptr;

		/*
		 * Maximal number of sequences the receive window will hold
		 */
		static const uint32_t RxWindowMaxSize = 64;
		/*
		 * Maximal size of send window
		 */
		static const uint32_t TxWindowMaxSize = 45;
		/*
		 * Maximal amount of packets queued for sending on this channel
		 */
		static const uint32_t TxQueueMaxSize = 1024;
		/*
		 * Amount of retransmissions before aborting packet and
		 * killing the connection
		 */
		static const uint32_t TxMaxRetries = 20;
		/*
		 * Send keepalive messages at this rate (msec) if server has nothing else to send
		 */
		static const uint32_t InactivityKeepaliveInterval = 1000;
		/*
		 * Wait at least this amount of time (msecs) between send bursts
		 */
		static const uint32_t TxBurstInterval = 200;
		/*
		 * Time to wait before attempting retransmission of a reliable packet
		 */
		static const uint32_t AckTimeout = 700;
		/*
		 * Time to wait before disconnecting client due to inactivity
		 * (no packets received)
		 */
		static uint32_t InactivityTimeout;
		typedef TimerMgr<uint64_t> Timer;

		static void initialize();

		BaseChannel(class Nub & nub, MessageFilter * filter, class MessageHandler * handler,
			Message::Table const & sendMessages, Message::Table const & receiveMessages);
		~BaseChannel();

		inline bool closed() const
		{
			return disconnected_;
		}

		inline bool condemned() const
		{
			return condemned_;
		}

		inline uint64_t lastActivity() const
		{
			return lastActivity_;
		}

		inline uint64_t lastPeerActivity() const
		{
			return lastPeerActivity_;
		}

		bool hasUnackedPackets() const;
		MessageFilter * filter() const;
		unsigned int sendQueueSize() const;
		Bundle::Ptr bundle();
		Bundle::Ptr unreliableBundle();
		void flushBundle(bool all);
		void sendBundle(Bundle::Ptr bundle, bool all = true);
		void close();
		void condemn();

		boost::asio::ip::udp::endpoint const & address() const;
		void setLastActivity(uint64_t time);

		void handlePacket(boost::asio::ip::udp::socket & socket, ReceivedPacket::Ptr & packet, uint64_t timestamp);
		bool addOutgoingPacket(Packet::Ptr packet);

		bool tick(uint64_t time);
		void bind(boost::asio::ip::udp::endpoint const & address);

	private:
		uint32_t references_;

		// Is the channel bound to a client?
		bool     bound_;
		// Is the channel disconnected?
		bool     disconnected_;
		// Is the channel condemned? (being closed)
		bool     condemned_;

		class Nub & nub_;
		// Address of client
		boost::asio::ip::udp::endpoint address_;
		// Message filter for outgoing packets
		MessageFilter * filter_;
		// Message handler for incoming packets
		MessageHandler * handler_;

		// Bundle of unsent reliable messages
		Bundle::Ptr bundle_;
		// Bundle of unsent unreliable messages
		Bundle::Ptr unreliableBundle_;
		// Timestamp of last server and client activity
		uint64_t lastActivity_, lastPeerActivity_;
		// Sequence number of next reliable send and receive frame
		Packet::SequenceId nextSendSequenceId_, nextReceiveSequenceId_;
		// Sequence number of next unreliable send and receive frame
		Packet::SequenceId nextUnreliableSendSequenceId_, nextUnreliableReceiveSequenceId_;

		// Reliable receive window
		Packet::SequenceId receiveWindowStart_;
		boost::circular_buffer<ReceivedPacket::Ptr> receiveWindow_;

		struct OutgoingPacket
		{
			// Packet to (re)send
			Packet::Ptr packet;
			// Retransmission retries so far
			uint32_t retries;
			// Timer handling retransmission for this packet
			Timer::TimerId timerId;
		};

		// Reliable send window
		boost::circular_buffer<OutgoingPacket> sendWindow_;
		// Packets waiting for getting admitted into the send window
		std::deque<Packet::Ptr> sendQueue_;

		// Pending receive bundles
		std::vector<class BundleUnpacker *> unpackers_;
		
		// Message format table for packing bundles
		Message::Table const & sendMessages_;
		// Message format table for unpacking bundles
		Message::Table const & receiveMessages_;
		
		void processBufferedMessages();
		void processAck(Packet::SequenceId seq);
		void processBundle(BundleUnpacker & unpacker);
		void processMessage(Message::Ptr message);

		void addOutgoingPacketToSendWindow(Packet::Ptr packet);
		Timer::TimerId addResendTimer(Packet::SequenceId seq);
		void deleteResendTimer(Timer::TimerId timerId);
		void expireTimer(Packet::SequenceId seq);
};

}

