#include <stdafx.hpp>
#include <mercury/channel.hpp>
#include <mercury/bundle.hpp>
#include <mercury/message_handler.hpp>
#include <mercury/encryption_filter.hpp>
#include <mercury/nub.hpp>
#include <common/service.hpp>

#define CHANNEL_DEBUG
#define CHANNEL_IGNORE_COMMON_ERRORS
// #define CHANNEL_VERBOSE_DEBUG
// #define CHANNEL_CONTENTS_DEBUG

// TODO LIST: 
//  - Recalculate ACK RTT dynamically & adjust retransmit / ack timeouts per client
//  - Add "channel clear" callback for resource sender (avoid exhausting txqueue / memory usage bursts)
//  - Add tx rate throttling
//  - Add rx rate throttling (by dropping packets) -- needed to avoid flooding server messages to others
//  - Add statistics collection (ackMin, Avg, Max, ...)
//  - Add cleanup handler
//  - Better handling of out-of-order unreliable packets (don't discard them)

namespace Mercury {

uint32_t BaseChannel::InactivityTimeout = 10000;

void BaseChannel::initialize()
{
	InactivityTimeout = (uint32_t)atol(Service::instance().getConfigParameter("client_inactivity_timeout").c_str());
}

BaseChannel::BaseChannel(Nub & nub, MessageFilter * filter, MessageHandler * handler,
			Message::Table const & sendMessages, Message::Table const & receiveMessages)
	: references_(0), 
	bound_(false), 
	disconnected_(false), 
	condemned_(false), 
	nub_(nub), 
	filter_(filter),
	handler_(handler), 
	lastActivity_(0), 
	lastPeerActivity_(0), 
	nextSendSequenceId_(0), 
	nextReceiveSequenceId_(0),
	nextUnreliableSendSequenceId_(0), 
	nextUnreliableReceiveSequenceId_(0),
	receiveWindowStart_(0),
	receiveWindow_(RxWindowMaxSize),
	sendWindow_(TxWindowMaxSize),
	sendMessages_(sendMessages),
	receiveMessages_(receiveMessages)
{
}

BaseChannel::~BaseChannel()
{
	delete filter_;
}

/**
 * Checks if there are any reliable packets in the send window/queue that weren't ACKed by the peer.
 */
bool BaseChannel::hasUnackedPackets() const
{
	return !sendWindow_.empty() || !sendQueue_.empty();
}

MessageFilter * BaseChannel::filter() const
{
	return filter_;
}

unsigned int BaseChannel::sendQueueSize() const
{
	return sendQueue_.size();
}

/*
 * Returns the active reliable bundle
 * (creates one if none is open)
 */
Bundle::Ptr BaseChannel::bundle()
{
	if (!bundle_)
		bundle_.reset(new Bundle(sendMessages_, true));

	return bundle_;
}

/*
 * Returns the active unreliable bundle
 * (creates one if none is open)
 */
Bundle::Ptr BaseChannel::unreliableBundle()
{
	if (!unreliableBundle_)
		unreliableBundle_.reset(new Bundle(sendMessages_, false));

	return unreliableBundle_;
}

/*
 * Finalze reliable and unreliable bundles and
 * flush their contents to the send window
 */
void BaseChannel::flushBundle(bool all)
{
	if (bundle_ && !bundle_->isEmpty())
		sendBundle(bundle_, all);
	if (unreliableBundle_ && !unreliableBundle_->isEmpty())
		sendBundle(unreliableBundle_, all);
}

/*
 * Sends a bundle on this channel
 */
void BaseChannel::sendBundle(Bundle::Ptr bundle, bool all)
{
	if (condemned() && bundle->hasMessages())
	{
		#ifdef CHANNEL_DEBUG
		WARN("Channel::sendBundle(): Sending a message on a condemned channel!");
		#endif
	}

	bundle->finalize(all);
	bundle->flush(this, all);
	SGW_ASSERT(!all || bundle->isFlushed());
	if (bundle->isFlushed())
	{
		if (bundle == bundle_)
			bundle_.reset();
		if (bundle == unreliableBundle_)
			unreliableBundle_.reset();
	}
}

/*
 * Closes and unregisters the channel immediately.
 */
void BaseChannel::close()
{
	if (!disconnected_)
	{
		// TODO: Clean up rx & tx
		handler_->onDisconnected();
		disconnected_ = true;
	}
}

/*
 * Add the channel to the condemned channels list.
 * The channel will be automatically closed when the last reliable message is ACKed by the peer.
 */
void BaseChannel::condemn()
{
	if (condemned_ || disconnected_)
	{
		WARN("BaseChannel::condemn(): Already condemned!");
		return;
	}

	// Flush all pending messages, as we won't be able to do so later
	flushBundle(true);

	condemned_ = true;
	nub_.condemn(shared_from_this());
}

Nub::Address const & BaseChannel::address() const
{
	return address_;
}

void BaseChannel::setLastActivity(uint64_t time)
{
	lastActivity_ = time;
}

/*
 * Handles a packet received from a client on this channel.
 */
void BaseChannel::handlePacket(boost::asio::ip::udp::socket & /*socket*/, ReceivedPacket::Ptr & packet, uint64_t timestamp)
{
	#ifdef CHANNEL_VERBOSE_DEBUG
	TRACEC(CATEGORY_MERCURY, "Received packet: flags 0x%02x, seq #%d", packet->flags(), packet->sequenceId());
	#endif
	
	#ifdef CHANNEL_CONTENTS_DEBUG
	packet->debugDump();
	#endif

	ReceivedPacket & p = *packet;

	if (disconnected_)
	{
		TRACEC(CATEGORY_MERCURY, "Channel disconnected, dropping frame");
		return;
	}

	lastPeerActivity_ = timestamp;

	/*
	 * Process ACKs even if the packet has an old sequence id.
	 * Usually packets do not get duplicated, but they might be received out of order, 
	 * causing the older packet (and important acknowledgements) to get discarded
	 */
	if (p.flags() & Packet::FLAG_HAS_ACKS)
	{
		const std::vector<uint32_t> & acks = p.acks();
		for (uint32_t i = 0; i < acks.size(); i++)
			processAck(acks[i]);
	}
	
	/*
	 * If the channel was condemned then we're only interested in the ACKs and not 
	 * the messages that were sent.
	 */
	if (condemned())
	{
		if (p.hasMessages())
		{
			#ifdef CHANNEL_DEBUG
			TRACEC(CATEGORY_MERCURY, "Channel is condemned, not processing packet at seq #%d", p.sequenceId());
			#endif
		}

		return;
	}

	/*
	 * Reliable messages are always processed in order, all of them must
	 * be processed, acked and they can be bundled unlike unreliable ones that
	 * can be discarded, processed out of order and should not be acked.
	 */
	if (p.flags() & Packet::FLAG_RELIABLE)
	{
		// Acknowledgement should be sent even if it's an already acked packet, as the previous ack might have got lost
		unreliableBundle()->addAck(p.sequenceId());

		if (p.sequenceId() < receiveWindowStart_)
		{
			#if defined(CHANNEL_DEBUG) && !defined(CHANNEL_IGNORE_COMMON_ERRORS)
			TRACEC(CATEGORY_MERCURY, "Discarding already seen reliable frame #%d below window #%d", p.sequenceId(), receiveWindowStart_);
			#endif
			return;
		}

		if (p.sequenceId() >= receiveWindowStart_ + receiveWindow_.capacity())
		{
			#ifdef CHANNEL_DEBUG
			TRACEC(CATEGORY_MERCURY, "Received seq #%d is not in window #%d -> #%d", p.sequenceId(), receiveWindowStart_,
				receiveWindowStart_ + receiveWindow_.capacity() - 1);
			#endif
			return;
		}

		uint32_t rwinIndex = p.sequenceId() - receiveWindowStart_;
		if (receiveWindow_.size() > rwinIndex && receiveWindow_[rwinIndex].get() != nullptr)
		{
			#ifdef CHANNEL_DEBUG
			TRACEC(CATEGORY_MERCURY, "Discarding already buffered packet #%d", p.sequenceId());
			#endif
			return;
		}
		
		// Fill the missing sequence numbers with empty references
		while (receiveWindow_.size() <= rwinIndex)
			receiveWindow_.push_back(ReceivedPacket::Ptr());
		
		// Buffer received frame
		receiveWindow_[rwinIndex] = packet;
		
		#ifdef CHANNEL_VERBOSE_DEBUG
		if (rwinIndex != 0)
			TRACEC(CATEGORY_MERCURY, "Buffered packet #%d over rwin first #%d", p.sequenceId(), receiveWindowStart_);
		#endif
		
		/*
		 * Fragmented packets need an unpacker to add
		 * multiple fragments to the same bundle
		 */
		bool canUnpack = false;
		if (p.flags() & Packet::FLAG_FRAGMENTED)
		{
			BundleUnpacker * unpacker = nullptr;

			// Check if there is an unpacker created for this bundle
			for (unsigned int i = 0; i < unpackers_.size(); i++)
			{
				if (unpackers_[i]->firstFragmentId() == p.firstFragmentId())
				{
					unpacker = unpackers_[i];
					break;
				}
			}

			#ifdef CHANNEL_VERBOSE_DEBUG
			TRACEC(CATEGORY_MERCURY, "Adding seq #%d to unpacker", p.sequenceId());
			#endif

			if (!unpacker)
			{
				/*
				 * No unpacker, create a new one.
				 *
				 * There is a corner case when the client sends a 
				 * non-fragmented frame and later a fragmented frame covering
				 * the range that the non-fragmented frame seq is in.
				 * This is not an issue, as the bundle won't be completed
				 * (the unfragmented packet won't get to the unpacker), so
				 * the connection will get closed due to rx stall
				 */
				unpacker = new BundleUnpacker(receiveMessages_, packet);
				unpackers_.push_back(unpacker);

				#ifdef CHANNEL_VERBOSE_DEBUG
				TRACEC(CATEGORY_MERCURY, "Creating new unpacker for #%d -> #%d", p.firstFragmentId(), p.lastFragmentId());
				#endif
			}
			// There is an unpacker created for this bundle, add the packet to it
			else if (!unpacker->addPacket(packet))
				return;

			// Check if the current bundle can be processed
			if (unpacker->isComplete() && receiveWindowStart_ == unpacker->firstFragmentId())
			{
				#ifdef CHANNEL_VERBOSE_DEBUG
				TRACEC(CATEGORY_MERCURY, "Unpacker has all frames, can unpack");
				#endif
				canUnpack = true;
			}
		}

		// Try to do a queue run
		if (rwinIndex == 0 || canUnpack)
			processBufferedMessages();
	}
	else
	{
		// Fragmented unreliable frames are not supported
		if (p.flags() & Packet::FLAG_FRAGMENTED)
		{
			#ifdef CHANNEL_DEBUG
			TRACEC(CATEGORY_MERCURY, "Discarding fragmented unreliable frame #%d", 
				p.sequenceId());
			#endif
			return;
		}

		if (p.sequenceId() < nextUnreliableReceiveSequenceId_)
		{
			#ifdef CHANNEL_DEBUG
			TRACEC(CATEGORY_MERCURY, "Discarding already-seen unreliable frame #%d below last frame #%d", 
				p.sequenceId(), nextUnreliableReceiveSequenceId_);
			#endif
			return;
		}

		nextUnreliableReceiveSequenceId_ = p.sequenceId();

		/*
		 * Unfragmented packets are technically one-fragment bundles
		 * (the internal stream is the same), so we'll process them
		 * with the same unpacker
		 */
		BundleUnpacker unpacker(receiveMessages_, packet);
		processBundle(unpacker);
	}
}

/*
 * Processes all messages currently held in the reliable receive window.
 * Processing will stop when a gap is reached in the window (a frame is missing),
 * or the frame has an unpacker that is incomplete.
 */
void BaseChannel::processBufferedMessages()
{
	#ifdef CHANNEL_VERBOSE_DEBUG
	TRACEC(CATEGORY_MERCURY, "Processing channel backlog");
	#endif
	while (!receiveWindow_.empty() && receiveWindow_[0] != nullptr)
	{
		ReceivedPacket::Ptr pkt = receiveWindow_[0];
		#ifdef CHANNEL_VERBOSE_DEBUG
		TRACEC(CATEGORY_MERCURY, "Processing bundled seq #%d", pkt->sequenceId());
		#endif
		if (pkt->flags() & Packet::FLAG_FRAGMENTED)
		{
			// Find an unpacker for the packet
			BundleUnpacker * unpacker = nullptr;
			unsigned int unpackerIdx;
			for (unsigned int i = 0; i < unpackers_.size(); i++)
			{
				if (unpackers_[i]->firstFragmentId() == pkt->firstFragmentId())
				{
					#ifdef CHANNEL_VERBOSE_DEBUG
					TRACEC(CATEGORY_MERCURY, "My unpacker is for seqs #%d -> #%d", pkt->firstFragmentId(), pkt->lastFragmentId());
					#endif
					unpackerIdx = i;
					unpacker = unpackers_[i];
					break;
				}
			}

			/*
			 * This should find an unpacker for this fragment
			 * as it gets created when a fragmented packet hits handlePacket()
			 */
			SGW_ASSERT(unpacker != nullptr);

			if (unpacker->isComplete())
			{
				// Bundle is complete, let's break it up into individual messages
				processBundle(*unpacker);

				// Clean up receive window & delete unpacker
				receiveWindowStart_ += unpacker->packets();
				#ifdef CHANNEL_VERBOSE_DEBUG
				TRACEC(CATEGORY_MERCURY, "Removing %d sequences from rwin", unpacker->packets());
				#endif
				for (unsigned int i = 0; i < unpacker->packets(); i++)
					receiveWindow_.pop_front();
				delete unpacker;
				if (unpackerIdx < unpackers_.size())
					unpackers_.erase(unpackers_.begin() + unpackerIdx);
			}
			else
			{
				#ifdef CHANNEL_VERBOSE_DEBUG
				TRACEC(CATEGORY_MERCURY, "Bundle is not complete, deferring bundle processing");
				#endif
				break;
			}
		}
		else
		{
			#ifdef CHANNEL_VERBOSE_DEBUG
			TRACEC(CATEGORY_MERCURY, "Processing standalone seq #%d", pkt->sequenceId());
			#endif
			receiveWindowStart_++;
			receiveWindow_.pop_front();
			BundleUnpacker unpacker(receiveMessages_, pkt);
			processBundle(unpacker);
		}
	}
}

bool BaseChannel::tick(uint64_t time)
{
	/*
	 * Disconnected channels don't send or receive messages
	 */
	if (disconnected_)
		return false;

	if (handler_)
		handler_->tick(time);

	/*
	 * Send a keepalive message if no messages were sent in the 
	 * last "InactivityKeepaliveInterval" msecs
	 */
	if (!condemned_ && lastActivity_ != 0 && lastActivity_ + InactivityKeepaliveInterval < time)
		bundle()->touch();

	// Flush all pending packets
	flushBundle(true);

	/*
	 * lastPeerActivity_ is 0 when the channel was opened, but
	 * no packet was received yet (except the connect request,
	 * which was handled by the server message handler, because
	 * it was not on a channel)
	 */
	if (lastPeerActivity_ == 0)
		lastPeerActivity_ = time;

	if (nub_.lastTick() - lastPeerActivity_ > InactivityTimeout)
	{
		DEBUG1C(CATEGORY_MERCURY, "Inactivity timeout triggered on channel %s, closing it", address_.address().to_string().c_str());
		close();
	}

	return true;
}

/*
 * Binds the channel to the specified client address.
 */
void BaseChannel::bind(Nub::Address const & address)
{
	SGW_ASSERT(!bound_);
	address_ = address;
	bound_ = true;
}

/*
 * Process an acknowledgement sent by the client
 */
void BaseChannel::processAck(Packet::SequenceId seq)
{
	// Sequence is for an old frame that is outside our send window, ignore
	if (seq < (nextSendSequenceId_ - sendWindow_.size()))
	{
		#if defined(CHANNEL_DEBUG) && !defined(CHANNEL_IGNORE_COMMON_ERRORS)
		TRACEC(CATEGORY_MERCURY, "Received acknowledgement for out-of-window packet #%d", seq);
		#endif
		return;
	}

	/*
	 * Sequence is for a frame that wasn't sent yet
	 * (we issue a warning because this is an abnormal condition)
	 */
	if (seq >= nextSendSequenceId_)
	{
		#ifdef CHANNEL_DEBUG
		WARNC(CATEGORY_MERCURY, "Received acknowledgement for not yet sent packet #%d", seq);
		#endif
		return;
	}

	uint32_t swndIndex = seq - (nextSendSequenceId_ - (uint32_t)sendWindow_.size());

	// Packet was already acked, ignore
	if (!sendWindow_[swndIndex].packet.get())
	{
		TRACEC(CATEGORY_MERCURY, "Received ack for already acked packet #%d", seq);
		return;
	}
	
	#ifdef CHANNEL_VERBOSE_DEBUG
	TRACEC(CATEGORY_MERCURY, "Packet #%d acked", seq);
	#endif

	// Remove the packet from the send window & cancel resend timer
	sendWindow_[swndIndex].packet.reset();
	deleteResendTimer(sendWindow_[swndIndex].timerId);

	// If the acked packet was the head of the send window, remove all empty slots following it
	if (swndIndex == 0)
	{
		while (!sendWindow_.empty() && (sendWindow_[0].packet.get() == nullptr))
			sendWindow_.pop_front();
		#ifdef CHANNEL_VERBOSE_DEBUG
		TRACEC(CATEGORY_MERCURY, "New tx queue size: %d", sendWindow_.size());
		#endif

		uint32_t numAdded = 0;
		while (!sendQueue_.empty() && !sendWindow_.full())
		{
			addOutgoingPacketToSendWindow(*sendQueue_.begin());
			sendQueue_.pop_front();
			numAdded++;
		}

		#ifdef CHANNEL_DEBUG
		if (numAdded)
			TRACEC(CATEGORY_MERCURY, "%d packets admitted into send window (%d remaining)", numAdded, sendQueue_.size());
		#endif
	}
}

/*
 * Unpack and process all messages in the bundle
 */
void BaseChannel::processBundle(BundleUnpacker & unpacker)
{
	#ifdef CHANNEL_VERBOSE_DEBUG	
	TRACEC(CATEGORY_MERCURY, "Processing bundle at %d", unpacker.firstFragmentId());
	#endif
	SGW_ASSERT(unpacker.isComplete());
	unpacker.beginUnpack();
	Message::Ptr msg;
	try
	{
		while ((msg = unpacker.next()))
			processMessage(msg);
	}
	catch (std::exception & e)
	{
		WARNC(CATEGORY_MERCURY, "Failed to unpack bundle: %s", e.what());
	}
}

/*
 * Dispatch a message to the corresponding message handler
 */
void BaseChannel::processMessage(Message::Ptr message)
{
	#ifdef CHANNEL_VERBOSE_DEBUG	
	TRACEC(CATEGORY_MERCURY, "Processing message: 0x%02x", message->messageId());
	#endif
	if (handler_)
		handler_->onMessage(message);
}

/*
 * Add a reliable packet to the send window
 */
void BaseChannel::addOutgoingPacketToSendWindow(Packet::Ptr packet)
{
	SGW_ASSERT(packet->flags() & Packet::FLAG_RELIABLE);

	if (sendWindow_.full())
	{
		WARN("Channel::addOutgoingPacketToSendWindow(%s): Reliable send window is full!",
			address_.address().to_string().c_str());
		return;
	}

	if (condemned_)
	{
		#ifdef CHANNEL_DEBUG
		WARN("Channel::addOutgoingPacketToSendWindow(%s): Tried to send a reliable packet on a condemned channel!",
			address_.address().to_string().c_str());
		#endif
		return;
	}

	Packet::SequenceId seq = nextSendSequenceId_++;
	OutgoingPacket pkt;
	pkt.packet = packet;
	pkt.timerId = addResendTimer(seq);
	pkt.retries = 0;
	sendWindow_.push_back(pkt);
	nub_.queuePacket(shared_from_this(), packet, seq);

#if defined(CHANNEL_VERBOSE_DEBUG)
	TRACEC(CATEGORY_MERCURY, "Added reliable seq #%d to outgoing", seq);
#endif
}

/*
 * Queue an outgoing packet for transmission
 * Unrealiable packets are added to the Nub send queue,
 * reliable packets are added to the channel send window/queue.
 */
bool BaseChannel::addOutgoingPacket(Packet::Ptr packet)
{
	if (disconnected_)
		return false;

	if (packet->flags() & Packet::FLAG_RELIABLE)
	{
		if (sendQueue_.empty() && !sendWindow_.full())
			addOutgoingPacketToSendWindow(packet);
		else
		{
			if (sendQueue_.size() < TxQueueMaxSize)
				sendQueue_.push_back(packet);
			else
			{
				WARNC(CATEGORY_MERCURY, "Tx queue size (%d) exceeded", TxQueueMaxSize);
				return false;
			}
		}
	}
	else
	{
		// Unreliable packets need no queuing as losing them
		// is not a problem
		Packet::SequenceId seq = ++nextUnreliableSendSequenceId_;
		nub_.queuePacket(shared_from_this(), packet, seq);
	}

	return true;
}

/*
 * Adds a resend timer for a reliable frame
 */
BaseChannel::Timer::TimerId BaseChannel::addResendTimer(Packet::SequenceId seq)
{
	// Check if the resend timer is for a valid packet
	SGW_ASSERT(seq < nextSendSequenceId_ && seq + 1 >= nextSendSequenceId_ - sendWindow_.size());

	auto self = shared_from_this();
	return nub_.timers().addTimer(
		[self, seq](auto &, auto, auto, auto *) { self->expireTimer(seq); },
		nub_.lastTick() + AckTimeout);
}

/*
 * Deletes the specified frame resend timer
 */
void BaseChannel::deleteResendTimer(Timer::TimerId timerId)
{
	nub_.timers().cancelTimer(timerId);
}

/*
 * Called when a packet resend timer expires
 * 
 * The "self" parameter is needed to keep the channel reference alive if
 * the channel dies sooner than the timer, do not remove it!
 */
void BaseChannel::expireTimer(Packet::SequenceId seq)
{
	if (disconnected_)
		return;

	/*
	 * Sanity check for send sequence
	 * When a seq is acked, its timer is cancelled, so we should only
	 * get unacked packets here
	 */
	SGW_ASSERT(seq < nextSendSequenceId_ && seq >= nextSendSequenceId_ - sendWindow_.size());
	OutgoingPacket & packet = sendWindow_[seq + sendWindow_.size() - nextSendSequenceId_];
	SGW_ASSERT(packet.packet);
	
#if defined(CHANNEL_VERBOSE_DEBUG)
	TRACEC(CATEGORY_MERCURY, "Resend timer expired for seq #%d", seq);
#endif

	if (packet.retries < TxMaxRetries)
	{
		packet.retries++;
		nub_.queuePacket(shared_from_this(), packet.packet, seq);
		packet.timerId = addResendTimer(seq);
	}
	else
	{
		WARNC(CATEGORY_MERCURY, "Resend retries exceeded for seq #%d", seq);
		close();
	}
}

}
