#pragma once

#include <mercury/message.hpp>
#include <mercury/packet.hpp>
#include <mercury/stream.hpp>

namespace Mercury
{

/*
 * A collection of messages.
 * Its main goal is to improve network efficiency of the BW
 * protocol by bundling packets together.
 */
class Bundle : public Mercury::OutputStream {
public:
	enum MessageFlag
	{
		FLUSH = 0x01
	};

	// All pointers to this type should be intrusive references
	typedef boost::shared_ptr<Bundle> Ptr;

	Bundle(Message::Table const & table, bool reliable);
	~Bundle();
	
	void beginMessage(uint8_t messageId, Message::Format const & format, uint32_t flags = 0);

	/*
	 * Begin writing a message with headers
	 */
	void beginMessage(uint8_t messageId, uint32_t flags = 0);

	/*
	 * Begin writing a type 1 (Base) entity RPC message with headers
	 */
	void beginEntityMessage(uint16_t methodId, uint32_t entityId, uint32_t flags = 0);

	/*
	 * Begin writing a type 2 (Cell) entity RPC message with headers
	 */
	void beginCellEntityMessage(uint16_t methodId, uint32_t entityId, uint32_t flags = 0);

	/*
	 * Finalizes the current message (updates length)
	 */
	void endMessage();

	/*
	 * Closes all packets and updates relative fragment indices
	 */
	void finalize(bool all = true);

	/*
	 * Add acknowledgement for a reliable packet
	 */
	void addAck(Packet::SequenceId ack);

	/*
	 * Returns if any messages or acks, or other interesting data were written to the bundle
	 */
	bool isEmpty() const;

	/*
	 * Returns if any messages were written to the bundle
	 */
	bool hasMessages() const;

	/*
	 * Returns the amount of packets (closed or pending) in this bundle
	 */
	std::size_t numPackets() const;

	virtual void * expand(uint32_t bytes, uint32_t & expanded);
	void * expandAtomic(uint32_t bytes);
	
	void writeString(std::string const & s);
	void writeString(std::wstring const & s);

	/*
	 * Flush packets up to the reliable driver, or
	 * all packets (if all = true)
	 */
	void flush(class BaseChannel * channel, bool all);

	/*
	 * Returns if all packets in the bundle were flushed
	 */
	bool isFlushed() const;

	/*
	 * Sets if packets in the bundle are sent on a channel or
	 * are standalone
	 */
	void setOnChannel(bool onChannel);

	/*
	 * Sets if the bundle contains a numbered request
	 */
	void setHasRequests(bool hasRequests);

	/*
	 * Forces the bundle to open a new packet even if it contains
	 * no messages or acknowledgements
	 */
	void touch();

protected:
	uint32_t references_;

private:
	const static uint32_t MaxSafeFragments = Packet::MaxFragmentsPerBundle - 5;

	// Message format table for packing bundles
	Message::Table const & table_;

	// Is the bundle on a chanel?
	bool onChannel_;
	// Does the bundle contain request messages?
	bool hasRequests_;
	// Is the bundle reliable?
	bool reliable_;
	// Is the bundle finalized?
	bool finalized_;
	bool finalizedToDriver_;

	// List of unflushed packets
	std::vector<Packet::Ptr> packets_;
	// Current packet being written
	Packet::Ptr currentPacket_;
	// Last packet that must be flushed after writing
	size_t driver_;
	// Next packet that will be flushed
	uint32_t flushedTo_;

	// Acknowledgements to send when the bundle is flushed
	std::vector<Packet::SequenceId> acks_;

	// Are we writing a message?
	bool writingMessage_;
	// Pointer to length field of current message
	void * messageLengthPtr_;
	// Length type of current message
	Message::Format const * messageFormat_;
	// Submission flags of current message
	uint32_t messageFlags_;
	// Offset of message body in current packet
	uint16_t messageStartOffset_;
	// Length of message (if length type is fixed)
	uint16_t messageFixedLength_;
	// Length of written message body
	uint16_t messageBodyLength_;
	// How many messages have we written so far
	uint32_t numMessages_;

	void closeCurrentPacket();
	Packet::Ptr createPacket();
};

/*
 * Helper class for unpacking messages from bundles
 */
class BundleUnpacker
{
public:
	BundleUnpacker(Message::Table const & table, ReceivedPacket::Ptr packet);
	~BundleUnpacker();

	bool addPacket(ReceivedPacket::Ptr packet);

	void beginUnpack();
	Message::Ptr next();

	Packet::SequenceId firstFragmentId() const;
	uint32_t packets() const;
	bool isComplete() const;

private:
	// Message format table for unpacking bundles
	Message::Table const & table_;
	// List of bundle fragments received so far
	// Must be complete to begin unpacking
	std::vector<ReceivedPacket::Ptr> fragments_;
	// Seq number of first and last fragment of the unpacker
	Packet::SequenceId firstFragment_, lastFragment_;
	// Number of fragments received so far
	uint32_t receivedFragments_;
	// Index of packet being unpacked
	uint32_t unpackPacket_;
	// Offset of the next message with a request ID
	uint16_t nextRequestOffset_;
};

}

