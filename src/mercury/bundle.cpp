#include <stdafx.hpp>
#include <mercury/bundle.hpp>
#include <mercury/channel.hpp>
#include <iostream>
#include <iomanip>

namespace Mercury
{
	
Bundle::Bundle(Message::Table const & table, bool reliable)
	: references_(0),
	table_(table),
	onChannel_(true),
	hasRequests_(false),
	reliable_(reliable), 
	finalized_(false), 
	finalizedToDriver_(false),
	driver_(-1), 
	writingMessage_(false), 
	messageLengthPtr_(nullptr), 
	messageFormat_(nullptr),
	numMessages_(0),
	flushedTo_(0)
{
}

Bundle::~Bundle()
{
}

void Bundle::beginMessage(uint8_t messageId, Message::Format const & format, uint32_t flags)
{
	SGW_ASSERT(!finalized_);
	endMessage();

	if (!currentPacket_)
		currentPacket_ = createPacket();
	
	// This needs to be set at the beginning so we know which packet type to use later
	messageFlags_ = flags;

	/*
	 * The header must be appended in one piece as BW has trouble unpacking
	 * bundles where header fields are in different packets.
	 */
	uint32_t headerLen = sizeof(messageId) + format.lengthType;
	void * header = expandAtomic(headerLen);
	*(uint8_t *)header = messageId;
	writingMessage_ = true;
	if (format.lengthType != Message::CONSTANT_LENGTH)
		messageLengthPtr_ = &((uint8_t *)header)[1];
	messageFormat_ = &format;
	messageFixedLength_ = format.length;
	messageBodyLength_ = 0;
	Packet::Ptr current = currentPacket_;
	messageStartOffset_ = current->offset();
}

/*
 * Begin writing a client message with headers
 */
void Bundle::beginMessage(uint8_t messageId, uint32_t flags)
{
	if (messageId >= 0x80)
		beginMessage(messageId, table_.entityMessage, flags);
	else
	{
		SGW_ASSERT(messageId < table_.size);
		beginMessage(messageId, table_.messages[messageId], flags);
	}
}

/*
 * Begin writing a type 1 (Base) entity RPC message with headers
 */
void Bundle::beginEntityMessage(uint16_t methodId, uint32_t entityId, uint32_t flags)
{
	SGW_ASSERT(methodId < 0x13D);
	if (methodId >= 0x3D)
	{
		beginMessage(0xBD, flags);
		*this << entityId << (uint8_t)(methodId - 0x3D);
	}
	else
	{
		beginMessage((uint8_t)methodId + 0x80, flags);
		*this << entityId;
	}
}

/*
 * Begin writing a type 2 (Cell) entity RPC message with headers
 */
void Bundle::beginCellEntityMessage(uint16_t methodId, uint32_t entityId, uint32_t flags)
{
	SGW_ASSERT(methodId < 0x13D);
	if (methodId >= 0x3D)
	{
		beginMessage(0xFD, flags);
		*this << entityId << (uint8_t)(methodId - 0x3D);
	}
	else
	{
		beginMessage((uint8_t)methodId + 0xC0, flags);
		*this << entityId;
	}
}

/*
 * Finalizes the current message (updates length)
 */
void Bundle::endMessage()
{
	if (writingMessage_)
	{
		messageBodyLength_ += (currentPacket_->offset() - messageStartOffset_);

		if (messageLengthPtr_)
		{
			if (messageFormat_->lengthType == Message::WORD_LENGTH)
				*(uint16_t *)messageLengthPtr_ = messageBodyLength_;
			else
			{
				SGW_ASSERT(messageFormat_->lengthType == Message::DWORD_LENGTH);
				*(uint32_t *)messageLengthPtr_ = messageBodyLength_;
			}
		}
		else
			SGW_ASSERT(messageBodyLength_ == messageFixedLength_);

		if (messageFlags_ & FLUSH)
		{
			driver_ = packets_.size();
			finalizedToDriver_ = false;
		}

		messageLengthPtr_ = nullptr;
		messageFormat_ = nullptr;
		messageBodyLength_ = 0;
		writingMessage_ = false;
		numMessages_++;
	}
}

/*
 * Closes all packets and updates relative fragment indices
 */
void Bundle::finalize(bool all)
{
	// Don't do anything if we already finalized the bundle
	// up to the reliable driver
	if (!all && finalizedToDriver_)
		return;
	if (finalized_)
		return;

	// No driver packet, don't finalize
	// This shortcut is needed to avoid closing the last packet
	if (!all && driver_ == -1)
	{
		finalizedToDriver_ = true;
		return;
	}

	endMessage();

	// Flush all ACKs to the next packet(s)
	if (!acks_.empty())
	{
		if (!currentPacket_)
			currentPacket_ = createPacket();

		uint32_t ack = 0;
		while (ack < acks_.size())
		{
			uint32_t maxAcks = currentPacket_->spaceForAcks();
			if (maxAcks == 0)
			{
				closeCurrentPacket();
				currentPacket_ = createPacket();
			}
			uint32_t writeAcks = ((uint32_t)acks_.size() - ack) < maxAcks ? ((uint32_t)acks_.size() - ack) : maxAcks;
			for (uint32_t i = 0; i < writeAcks; i++)
			{
				currentPacket_->addAcknowledgement(acks_[ack + i]);
			}
			ack += writeAcks;
		}
		SGW_ASSERT(ack == acks_.size());
	}
	
	// Close last packet
	if (currentPacket_)
		closeCurrentPacket();

	uint32_t startingIndex = flushedTo_;
	uint32_t endIndex = (uint32_t)(all ? packets_.size() : driver_ + 1);
	SGW_ASSERT(endIndex >= startingIndex);

	// Update fragment indices up to the last packet (if there is one)
	if (endIndex - startingIndex > 1)
	{
		for (unsigned int i = startingIndex; i < endIndex; i++)
		{
			packets_[i]->setFragments(i - startingIndex, endIndex - startingIndex - 1);
			packets_[i]->finalize();
		}
	}
	else if (endIndex - startingIndex == 1)
		packets_[endIndex - 1]->finalize();
	
	finalizedToDriver_ = true;
	if (all)
		finalized_ = true;
}

/*
 * Add acknowledgement for a reliable packet
 */
void Bundle::addAck(Packet::SequenceId ack)
{
	SGW_ASSERT(!finalized_);
	acks_.push_back(ack);
}

/*
 * Check if there are any messages, acks or other interesting data written to the bundle
 */
bool Bundle::isEmpty() const
{
	return acks_.empty() && packets_.empty() && 
		(!currentPacket_ || currentPacket_->isEmpty());
}

/*
 * Check if there are any messages written to the bundle
 */
bool Bundle::hasMessages() const
{
	return numMessages_ > 0;
}

/*
 * Returns the amount of packets (closed or pending) in this bundle
 */
std::size_t Bundle::numPackets() const
{
	return packets_.size() + (currentPacket_ ? 1 : 0);
}

/*
 * Increases bundle size by (bytes). The current packet is expanded if it has
 * any free space; a new packet is opened and expanded otherwise.
 */
void * Bundle::expand(uint32_t bytes, uint32_t & expanded)
{
	SGW_ASSERT(!finalized_);
	uint32_t free = currentPacket_->getFreeSpace();
	if (free)
	{
		void * buf = currentPacket_->expand(std::min(free, bytes), expanded);
		SGW_ASSERT(currentPacket_->getFreeSpace() < 0x1000000);
		return buf;
	}
	else
	{
		if (writingMessage_)
			messageBodyLength_ += (currentPacket_->offset() - messageStartOffset_);
		// There isn't enough space in the packet, we need to start a new one
		closeCurrentPacket();
		currentPacket_ = createPacket();
		if (writingMessage_)
			messageStartOffset_ = currentPacket_->offset();
		free = currentPacket_->getFreeSpace();
		return currentPacket_->expand(std::min(free, bytes), expanded);
	}
}

/*
 * Increases bundle size by (bytes). The current packet is expanded if it has
 * enough space; a new packet is opened and expanded otherwise.
 * Unlike expand(), expandAtomic() is guaranteed to return a contiguous chunk of
 * memory (not fragmented to multiple packets)
 */
void * Bundle::expandAtomic(uint32_t bytes)
{
	SGW_ASSERT(!finalized_);
	void * buf = nullptr;
	if (currentPacket_->hasEnoughSpaceFor(bytes))
	{
		buf = currentPacket_->expandAtomic(bytes);
	}
	else
	{
		if (writingMessage_)
			messageBodyLength_ += (currentPacket_->offset() - messageStartOffset_);
		// There isn't enough space in the packet, we need to start a new one
		closeCurrentPacket();
		currentPacket_ = createPacket();
		if (writingMessage_)
			messageStartOffset_ = currentPacket_->offset();
		buf = currentPacket_->expandAtomic(bytes);
	}
	return buf;
}

/*
 * Flushes pending packets in the bundle to the specified channel and
 * sets the last flushed index to avoid flushing them later.
 */
void Bundle::flush(BaseChannel * channel, bool all)
{
	SGW_ASSERT(finalizedToDriver_);
	SGW_ASSERT(!all || finalized_);

	uint32_t last = (uint32_t)(all ? packets_.size() : driver_ + 1);
	for (uint32_t i = flushedTo_; i < last; i++)
		channel->addOutgoingPacket(packets_[i]);
	flushedTo_ = last;
}

/*
 * Returns true if all packets in the bundle were flushed
 */
bool Bundle::isFlushed() const
{
	return (flushedTo_ == packets_.size());
}

/*
 * Sets if packets in the bundle are sent on a channel or
 * are standalone
 */
void Bundle::setOnChannel(bool onChannel)
{
	SGW_ASSERT(packets_.empty() && !currentPacket_);
	onChannel_ = onChannel;
}

/*
 * Sets if the bundle contains a numbered request
 */
void Bundle::setHasRequests(bool hasRequests)
{
	SGW_ASSERT(packets_.empty() && !currentPacket_);
	hasRequests_ = hasRequests;
}

/*
 * Forces the bundle to open a new packet even if it contains
 * no messages or acknowledgements
 */
void Bundle::touch()
{
	if (!currentPacket_)
		currentPacket_ = createPacket();

	currentPacket_->touch();
}

/*
 * Appends an SGW string to the bundle
 */
void Bundle::writeString(std::string const & s)
{
	*this << (uint8_t)s.length();
	write(s.data(), (unsigned)s.length());
}

/*
 * Appends an SGW UTF-16 string to the bundle
 */
void Bundle::writeString(std::wstring const & s)
{
	*this << (uint32_t)s.length();
	write(s.data(), (unsigned)s.length() * 2);
}

/*
 * Finalizes the last written packet if it wasn't finalized already
 */
void Bundle::closeCurrentPacket()
{
	if (currentPacket_)
	{
		if (!currentPacket_->isEmpty())
			packets_.push_back(currentPacket_);
		currentPacket_.reset();
	}
}

/*
 * Creates a new packet for the bundle
 */
Packet::Ptr Bundle::createPacket()
{
	Packet::Ptr packet(new Packet());
	uint8_t flags = Packet::FLAG_HAS_SEQUENCE;
	if (hasRequests_)
		flags |= Packet::FLAG_HAS_REQUESTS;
	if (onChannel_)
		flags |= Packet::FLAG_ON_CHANNEL;
	if (reliable_)
		flags |= Packet::FLAG_RELIABLE;
	packet->reset(flags);
	return packet;
}

/*
 * Creates an unpacker and adds the first packet to the fragment list.
 */
BundleUnpacker::BundleUnpacker(Message::Table const & table, ReceivedPacket::Ptr packet)
	: table_(table), nextRequestOffset_(0)
{
	if (packet->flags() & Packet::FLAG_FRAGMENTED)
	{
		firstFragment_ = packet->firstFragmentId();
		lastFragment_ = packet->lastFragmentId();
		fragments_.resize(lastFragment_ - firstFragment_ + 1);
		fragments_[packet->sequenceId() - firstFragment_] = packet;
	}
	else
	{
		firstFragment_ = lastFragment_ = packet->sequenceId();
		fragments_.push_back(packet);
	}

	receivedFragments_ = 1;
}

BundleUnpacker::~BundleUnpacker()
{
}

/*
 * Adds the packet to the fragment list.
 */
bool BundleUnpacker::addPacket(ReceivedPacket::Ptr packet)
{
	SGW_ASSERT(firstFragment_ != lastFragment_);
	if (firstFragment_ != packet->firstFragmentId() ||
		lastFragment_ != packet->lastFragmentId())
	{
		WARNC(CATEGORY_MERCURY, "Tried to add invalid fragment (%d, %d) to bundle (%d, %d)",
			packet->firstFragmentId(), packet->lastFragmentId(),
			firstFragment_, lastFragment_);
		return false;
	}

	uint32_t fragmentIdx = packet->sequenceId() - firstFragment_;
	SGW_ASSERT(fragments_[fragmentIdx] == nullptr);
	fragments_[fragmentIdx] = packet;
	receivedFragments_++;
	return true;
}

void BundleUnpacker::beginUnpack()
{
	SGW_ASSERT(fragments_.size() == receivedFragments_);
	unpackPacket_ = 0;
	auto first = *fragments_.begin();
	if (first->flags() & Packet::FLAG_HAS_REQUESTS)
	{
		nextRequestOffset_ = first->firstRequestOffset();
	}
}

/*
 * Extracts the next packet from the unpacker.
 * If no more messages can be read or the stream is corrupted,
 * the function returns nullptr.
 */
Message::Ptr BundleUnpacker::next()
{
	SGW_ASSERT(fragments_[unpackPacket_] != nullptr);

	if (fragments_[unpackPacket_]->remaining() == 0)
	{
		// Reached end of last packet in bundle
		if (unpackPacket_ == fragments_.size() - 1)
			return Message::Ptr();

		// This wasn't the last packet, jump to the next
		unpackPacket_++;
	}
	
	ReceivedPacket & pkt = *fragments_[unpackPacket_];
	uint32_t messageStart = pkt.offset();

	// Read message header (message id and length)
	uint8_t messageId;
	pkt >> messageId;

	/*
	 * Determine message length
	 *  < 0x80: Message should be in the msg descriptor table
	 * >= 0x80: Message has a 16-bit length field
	 */
	Message::Length type;
	uint16_t length;
	if (messageId >= 0x80)
		type = Message::WORD_LENGTH;
	else if (messageId < table_.size)
	{
		type = table_.messages[messageId].lengthType;
		length = table_.messages[messageId].length;
	}
	else
	{
		WARNC(CATEGORY_MERCURY, "Encountered unhandled message (%02x) in bundle!", messageId);
		return Message::Ptr();
	}
	
	if (type == Message::WORD_LENGTH)
		pkt >> length;
	else if (type == Message::DWORD_LENGTH)
	{
		uint32_t len;
		pkt >> len;
		if (len > 0xFFFF)
		{
			WARNC(CATEGORY_MERCURY, "Encountered oversized message (%d) in bundle!", len);
			return Message::Ptr();
		}
		length = (uint16_t)len;
	}
	
	uint32_t requestId = 0;
	if (nextRequestOffset_ == messageStart)
	{
		if (pkt.remaining() < sizeof(uint32_t) + sizeof(uint16_t))
		{
			WARNC(CATEGORY_MERCURY, "Not enough data on stream for request ID and offset");
			return Message::Ptr();
		}

		pkt >> requestId >> nextRequestOffset_;
	}
	
	if (pkt.remaining() >= length)
	{
		/*
		 * The whole message body is in the current packet
		 * Copy it to a message buffer and return
		 */
		Message::Ptr msg(new Message((uint8_t *)&pkt.buffer()[pkt.offset()], length));
		pkt.seek(pkt.offset() + length);
		msg->setMessageId(messageId);
		msg->setRequestId(requestId);
		return msg;
	}

	if (pkt.remaining() == 0 && unpackPacket_ < fragments_.size() - 1 &&
		fragments_[unpackPacket_ + 1]->remaining() >= length)
	{
		/*
		 * The whole message body is at the beginning of the next packet
		 * and the current packet is empty
		 */
		unpackPacket_++;
		ReceivedPacket & nextPkt = *fragments_[unpackPacket_];
		Message::Ptr msg(new Message((uint8_t *)&nextPkt.buffer()[nextPkt.offset()], length));
		nextPkt.seek(nextPkt.offset() + length);
		msg->setMessageId(messageId);
		msg->setRequestId(requestId);
		return msg;
	}
	
	/*
	 * The message spans multiple packets; iterate over packets
	 * and read buffers from them until we reach the end of packet
	 * (or end of stream)
	 */
	Message::Ptr msg(new Message(length));
	msg->setMessageId(messageId);
	msg->setRequestId(requestId);
	uint32_t msgOffset = 0;
	ReceivedPacket::Ptr currPkt = fragments_[unpackPacket_];
	do
	{
		if (currPkt->remaining() >= length - msgOffset)
		{
			currPkt->fetch(&msg->buffer()[msgOffset], length - msgOffset);
			msgOffset = length;
		}
		else
		{
			uint32_t read = currPkt->remaining();
			currPkt->fetch(&msg->buffer()[msgOffset], read);
			msgOffset += read;

			if (++unpackPacket_ == fragments_.size())
			{
				WARNC(CATEGORY_MERCURY, "Not enough data on stream for message (needed %d bytes)", length - msgOffset);
				return Message::Ptr();
			}

			currPkt = fragments_[unpackPacket_];
		}
	}
	while (msgOffset < length);

	return msg;
}

Packet::SequenceId BundleUnpacker::firstFragmentId() const
{
	return firstFragment_;
}

uint32_t BundleUnpacker::packets() const
{
	return (uint32_t)fragments_.size();
}

bool BundleUnpacker::isComplete() const
{
	return (fragments_.size() == receivedFragments_);
}

}
