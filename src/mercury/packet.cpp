#include <stdafx.hpp>
#include <mercury/packet.hpp>
#include <mercury/message.hpp>
#include <iostream>
#include <iomanip>

namespace Mercury
{

Packet::Packet(uint32_t initialSize)
	: Mercury::MemoryStream(initialSize), references_(0), flags_(FLAG_ON_CHANNEL), 
	  touched_(false), footerOffset_(0)
{
}

void Packet::finalize()
{
	SGW_ASSERT(!footerOffset_);
	SGW_ASSERT(offset() <= MAX_BODY_LENGTH + sizeof(Flags));

	Mercury::MemoryStream & buf = *this;

	*(Flags *)&buffer_[0] = flags_;
	
	/*
	 * Write dummy values for fragment and sequence indices,
	 * as those will be updated when binding the packet to a channel
	 */
	footerOffset_ = offset();
	if (flags_ & FLAG_FRAGMENTED)
		buf << (uint32_t)0 << (uint32_t)0;

	if (flags_ & FLAG_HAS_SEQUENCE)
		buf << (uint32_t)0;
			
	if (flags_ & FLAG_HAS_ACKS)
	{
		for (unsigned int i = 0; i < acks_.size(); i++)
			buf << acks_[i];

		uint8_t acks = (uint8_t)acks_.size();
		buf << acks;
	}

	size_ = offset_;

	// Make sure that the packet won't get truncated
	SGW_ASSERT(buf.offset() <= MAX_LENGTH);
}

void Packet::touch()
{
	touched_ = true;
}

void Packet::send(SequenceId sequenceId)
{
	SGW_ASSERT(footerOffset_);

	uint32_t off = footerOffset_;

	if (flags_ & FLAG_FRAGMENTED)
	{
		*(SequenceId *)&buffer_[off] = sequenceId - fragmentIndex_;
		*(SequenceId *)&buffer_[off + sizeof(SequenceId)] = sequenceId - fragmentIndex_ + fragments_;
		off += 2 * sizeof(SequenceId);
	}

	if (flags_ & FLAG_HAS_SEQUENCE)
		*(SequenceId *)&buffer_[off] = sequenceId;
}

void Packet::reset(uint8_t initialFlags)
{
	seek(1);
	flags_ = initialFlags;
	acks_.clear();
}

void Packet::setFragments(uint32_t myFragmentIndex, uint32_t numFragments)
{
	flags_ |= FLAG_FRAGMENTED;
	fragmentIndex_ = myFragmentIndex;
	fragments_ = numFragments;
}

void Packet::addAcknowledgement(uint32_t sequenceId)
{
	flags_ |= FLAG_HAS_ACKS;
	acks_.push_back(sequenceId);
	// The ACK counter is 8 bits, and it would overflow!
	// FIXME: Re-add this as BUG()
	SGW_ASSERT(acks_.size() <= MaxAcks);
}

void Packet::chopFromTail(uint32_t size)
{
	SGW_ASSERT(size <= size_);
	size_ -= size;
}

bool Packet::hasEnoughSpaceFor(uint32_t bytes) const
{
	// This assumes that piggybacking is not used and only flags are written
	// (which is true for BaseApp, as those connections don't use piggybacking)
	return getFreeSpace() >= bytes;
}

uint32_t Packet::spaceForAcks() const
{
	// Return the amount of ACKs that can be written to this packet
	uint32_t space = getFreeSpace();
	if (!(flags_ & FLAG_HAS_ACKS))
	{
		// Packet has no acks -> we need to remove one byte for the ack counter
		space--;
	}
	uint32_t acks = space / sizeof(SequenceId);
	if (acks > MaxAcks - acks_.size())
		acks = MaxAcks - (uint32_t)acks_.size();
	return acks;
}

uint32_t Packet::getFreeSpace() const
{
	SGW_ASSERT(MAX_BODY_LENGTH + sizeof(Flags) >= offset());
	return MAX_BODY_LENGTH + sizeof(Flags) - offset();
}

ReceivedPacket::ReceivedPacket()
	: Packet(MAX_RECEIVE_LENGTH), sequenceId_(NullSequence)
{
}

/*
	Packets are bidirectional (that is, they must be read from both the beginning and the end)

	Rough format:
		uint8 flags	[this has to be read first, it determines whether other members are present]
		[message part] (see later)

		uint16_t first_req_offset (when FLAG_HAS_REQUESTS is set)
		uint32 first_fragment_id  (when FLAG_FRAGMENTED is set)
		uint32 last_fragment_id   (when FLAG_FRAGMENTED is set)
		(Identifies the first and last sequence id of the bundle; the *message part* of all packets must
		be put together and parsed as one big data chunk)

		uint32 sequence_id        (when FLAG_HAS_SEQUENCE is set)
		(the exact order of messages cannot be determined when this field
			field is omitted, so we require HAS_SEQUENCE to be set in all packets)

		uint32 ack[ack_count] (when FLAG_HAS_ACKS is set, has to be read ack_count times)
		uint8 ack_count       (when FLAG_HAS_ACKS is set)

	Optional part (only when authenticated with the BaseApp):
		PKCS #7 Padding:
			The packet is padded to a multiple of the AES-256 block size (16 bytes).
			Every pad byte contain the amount of padding bytes appended.
			If the packet size is already a multiple of the block size, 16 bytes of padding
			is appended.
			
		After this step, the packet is encrypted with cipher block chained AES-256.
			
		HMAC-MD5: A 16-byte MD5 MAC is calculated from the encrypted packet, using the AES key as
			the shared secret.

	The message part consists of an arbitrary amount of messages with the following format:
		uint8 messageId
		XXXXX length
		uint32_t requestId (if the previous nextRequestOffset points to the beginning of this message)
		uint16_t nextRequestOffset (if the previous nextRequestOffset points to the beginning of this message)
		uint8 contents[length]

	How to determine message length:
		If the message id is greater than or equal to 0x80 (Client/Base/Cell message id), the length
		field is always a 16-bit integer.

		Otherwise the client or server-side message table contains the length type. It can be:
			- a fixed static length read from the table (no length field in the packet)
			- a 16-bit integer length field
			- a 32-bit integer length field (rare)

	Sequence numbers:
		Only sequence numbers below 0x0FFFFFFF are valid.
		0x10000000 is a "null" sequence.

	Piggybacked message format (FLAG_PIGGYBACK is set):
		uint8_t flags

		uint8_t dataN[packetN_len]
		int16_t packetN_len

		...

		uint8_t data2[packet2_len]
		int16_t packet2_len

		uint8_t data1[packet1_len]
		int16_t packet1_len

	Note #1: Piggybacked messages cannot contain acknowledgements, sequence id or fragment id-s
	Note #2: The packets are read in reverse order, so the last packet is actually the first
	Note #3: The last (N-th) packet should have a signed negative length (e.g. -40 instead of 40), this indicates the end of the chain.

*/
bool ReceivedPacket::unserialize(bool isOnChannel, bool warn)
{
	#define WARN_BAD_PACKET(msg, ...) { if (warn) WARNC(CATEGORY_MERCURY, msg, ##__VA_ARGS__); }

	// Simplify read operation syntax
	Mercury::MemoryStream & buf = *this;

	buf >> flags_;
	if (flags_ == 0)
	{
		WARN_BAD_PACKET("Received packet with bad flags: %02x!", flags_);
		return false;
	}

	if (flags_ & FLAG_PIGGYBACK)
	{
		WARN_BAD_PACKET("Piggybacked packets are not supported");
		return false;
	}

	if (flags_ & FLAG_HAS_ACKS)
	{
		uint8_t ack_count;
		buf.pop(ack_count);

		if (ack_count == 0)
		{
			WARN_BAD_PACKET("Received packet with FLAG_HAS_ACKS and 0 acks");
			return false;
		}

		if (buf.remaining() < (uint32_t)ack_count * 4)
		{
			WARN_BAD_PACKET("Not enough data for acks in footer (got %d, %d needed)", buf.remaining(), ack_count * 4);
			return false;
		}

		acks_.resize(ack_count);
		if (ack_count)
			buf.pop(&acks_[0], sizeof(SequenceId) * ack_count);
	}

	if (isOnChannel && (!(flags_ & FLAG_ON_CHANNEL)))
	{
		WARN_BAD_PACKET("Packet received without FLAG_ON_CHANNEL set: 0x%02x", flags_);
		return false;
	}

	if (!isOnChannel && (flags_ & FLAG_ON_CHANNEL))
	{
		WARN_BAD_PACKET("Packet received with FLAG_ON_CHANNEL set: 0x%02x", flags_);
		return false;
	}

	if (flags_ & FLAG_HAS_SEQUENCE)
	{
		buf.pop(sequenceId_);
		if (sequenceId_ == NullSequence)
		{
			WARN_BAD_PACKET("Null sequence number in packet");
			return false;
		}

		if (sequenceId_ > NullSequence)
		{
			WARN_BAD_PACKET("Seq %08x outside valid range", sequenceId_);
			return false;
		}
	}
	else
	{
		WARN_BAD_PACKET("Packet received without FLAG_HAS_SEQUENCE: %02x", flags_);
		return false;
	}
	
	if (flags_ & FLAG_FRAGMENTED)
	{
		buf.pop(lastFragmentId_).pop(firstFragmentId_);

		if (firstFragmentId_ >= lastFragmentId_)
		{
			WARN_BAD_PACKET("Bogus fragment count in packet: firstFragment >= lastFragment (%d >= %d)", firstFragmentId_, lastFragmentId_);
			return false;
		}

		if (lastFragmentId_ - firstFragmentId_ + 1 > MaxFragmentsPerBundle)
		{
			WARN_BAD_PACKET("Bundle size (%d) exceeds fragment limit (%d)", lastFragmentId_ - firstFragmentId_ + 1, MaxFragmentsPerBundle);
			return false;
		}

		if (sequenceId_ < firstFragmentId_ || sequenceId_ > lastFragmentId_)
		{
			WARN_BAD_PACKET("Packet seq (%d) outside fragment (%d, %d)", sequenceId_, firstFragmentId_, lastFragmentId_);
			return false;
		}
	}

	if (flags_ & FLAG_INDEXED)
	{
		WARN_BAD_PACKET("Indexed channel packets are not supported");
		return false;
	}
	
	if (flags_ & FLAG_HAS_REQUESTS)
	{
		buf.pop(firstRequestOffset_);
		if (firstRequestOffset_ < 1 || firstRequestOffset_ > buf.size())
		{
			WARN_BAD_PACKET("Packet has illegal first request offset (%d); stream length: %d", firstRequestOffset_, buf.size());
			return false;
		}
	}

	// The buffer object now holds the message list only
	// (to be more precise, the offset and length fields are the boundaries of the message list)
	return true;
	#undef WARN_BAD_PACKET
}


void Packet::debugDump(uint32_t length)
{
	std::cout << "\tflags: ";
	if (flags_ & FLAG_HAS_REQUESTS)
		std::cout << "REQS ";
	if (flags_ & FLAG_PIGGYBACK)
		std::cout << "PIGGYBACK ";
	if (flags_ & FLAG_HAS_ACKS)
		std::cout << "ACKS ";
	if (flags_ & FLAG_ON_CHANNEL)
		std::cout << "ON_CHANNEL ";
	if (flags_ & FLAG_RELIABLE)
		std::cout << "RELIABLE ";
	if (flags_ & FLAG_FRAGMENTED)
		std::cout << "FRAGMENTED ";
	if (flags_ & FLAG_HAS_SEQUENCE)
		std::cout << "SEQ ";
	if (flags_ & FLAG_INDEXED)
		std::cout << "INDEXED ";
	std::cout << std::endl;

	if (flags_ & FLAG_HAS_ACKS)
	{
		std::cout << "\tacks: ";
		for (unsigned int i = 0; i < acks_.size(); i++)
			std::cout << acks_[i] << " ";
		std::cout << std::endl;
	}
	
	std::cout << "\tlength: " << offset() << "\tfooter: " << footerOffset_ << std::endl;
	if (flags_ & FLAG_FRAGMENTED)
		std::cout << "\tfrags: " << fragments_ << "\tfragIdx: " << fragmentIndex_ << std::endl;
	if (flags_ & FLAG_HAS_REQUESTS)
		std::cout << "\trequest offset: " << firstRequestOffset_ << std::endl;
	std::cout << "\tbody: ";
	SGW_ASSERT(length <= size());
	for (unsigned int i = 0; i < length; i++)
	{
		if ((i % 16) == 0)
			std::cout << std::endl << "\t";
		std::cout << std::hex << std::setw(2) << std::setfill('0') << (unsigned int)((uint8_t)buffer_[i]) << " ";
	}

	std::cout << std::dec << std::endl;
}

void ReceivedPacket::debugDump()
{
	std::cout << "\tseq: " << sequenceId_ << std::endl;

	if (flags_ & FLAG_FRAGMENTED)
		std::cout << "\tfrags: " << firstFragmentId_ << " - " << lastFragmentId_ << std::endl;

	Packet::debugDump(size());
}

}
