#pragma once

#include <mercury/memory_stream.hpp>

namespace Mercury
{

/*
 * A BigWorld UDP network packet.
 * 
 * We don't store exact sequence ID-s (only relative indices) on purpose:
 * packets need to be sent to multiple destinations (as they're sent as part of a bundle),
 * and hardcoding the sequence ID would bind them (and the bundle) to a channel instance.
 * Making them channel-independent conserves memory on broadcast messages.
 * (but adds a retransmission overhead, which is negligible since retransmits don't happen often)
 */
class Packet : public Mercury::MemoryStream
{
	public:
		typedef uint32_t SequenceId;
		typedef uint8_t AckCount;
		typedef uint8_t Flags;
		typedef std::vector<SequenceId> AckList;

		const static SequenceId NullSequence = 0x10000000;

		const static uint8_t MaxAcks = 0xff;
		// TODO: Make this a per-channel setting!
		const static uint32_t MaxFragmentsPerBundle = 64;

		/*
		 * Core MTU = 1500 (1492 for ADSL)
		 * IP header: 20 bytes (average)
		 * UDP header: 8 bytes
		 */
		const static uint32_t MAX_RECEIVE_LENGTH = 1456;

		/*
		 * HMAC: 16 bytes
		 * Space left for UDP packet: 1448 bytes
		 * AES encrypts in multiples of 16 bytes, so truncating this to a multiple of 16 bytes we get 1440 bytes.
		 * PKCS #7 padding is at least 1 byte (otherwise it adds 16 bytes of padding to pad it to 16's multiple again),
		 * so the maximal length of the actual contents are 1424 bytes.
		 */
		const static uint32_t MAX_LENGTH = (MAX_RECEIVE_LENGTH & ~0x0f) - 32;

		/*
		 * Additional message fields: header (1 byte), sequence ID (4 byte), fragments (2*4 byte)
		 */
		const static uint32_t MAX_BODY_LENGTH = MAX_LENGTH - sizeof(SequenceId) * 3 - sizeof(Flags);
	
		typedef std::shared_ptr<Packet> Ptr;

		enum
		{
			FLAG_HAS_REQUESTS = 0x01, // Packet has message tagged with a RequestID
			FLAG_PIGGYBACK = 0x02,    // Packets are piggybacked to the end of this packet
			FLAG_HAS_ACKS = 0x04,     // Acknowledgement list is found at the end of the packet
			FLAG_ON_CHANNEL = 0x08,   // Message is sent on a channel
			FLAG_RELIABLE = 0x10,     // Packet sent on a reliable channel and must be ACKed
			FLAG_FRAGMENTED = 0x20,   // Packet is part of a fragmented bundle
			FLAG_HAS_SEQUENCE = 0x40, // Packet has a sequence id
			FLAG_INDEXED = 0x80       // SGW has no indexed channels, we don't use this flag
		};

		Packet(uint32_t initialSize = MAX_LENGTH);

		inline uint8_t flags() const
		{
			return flags_;
		}

		inline AckList const & acks() const
		{
			return acks_;
		}

		inline uint32_t references() const
		{
			return references_;
		}

		inline bool isEmpty() const
		{
			return offset() == sizeof(Flags) && acks_.empty() && !touched_;
		}

		inline uint16_t firstRequestOffset() const
		{
			return firstRequestOffset_;
		}
		
		void reset(uint8_t initialFlags = FLAG_ON_CHANNEL);
		void finalize();
		void touch();
		void send(SequenceId sequenceId);

		void setFragments(uint32_t myFragmentIndex, uint32_t numFragments);
		void addAcknowledgement(SequenceId sequenceId);
		
		void chopFromTail(uint32_t size);

		bool hasEnoughSpaceFor(uint32_t bytes) const;
		uint32_t spaceForAcks() const;
		uint32_t getFreeSpace() const;
		void debugDump(uint32_t length);

	protected:
		uint32_t references_;
		uint8_t flags_;
		uint16_t firstRequestOffset_;
		uint32_t fragmentIndex_, fragments_;
		std::vector<SequenceId> acks_;
		bool touched_;

		// Used to update sequence numbers for each resend 
		// on different channels
		uint32_t footerOffset_;
};

/*
 * A packet that was received from a client
 * Unlike Packet, ReceivedPacket has fixed sequence and fragment ID-s.
 */
class ReceivedPacket : public Packet
{
	public:
		typedef std::shared_ptr<ReceivedPacket> Ptr;

		ReceivedPacket();
		
		inline bool hasMessages() const
		{
			return size() - sizeof(Flags) > 0;
		}

		inline SequenceId sequenceId() const
		{
			return sequenceId_;
		}

		inline SequenceId firstFragmentId() const
		{
			return firstFragmentId_;
		}

		inline SequenceId lastFragmentId() const
		{
			return lastFragmentId_;
		}

		bool unserialize(bool isOnChannel, bool warn = true);
		void debugDump();

	private:
		uint32_t sequenceId_, firstFragmentId_, lastFragmentId_;
};

}
