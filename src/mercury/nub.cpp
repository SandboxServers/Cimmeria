#include <stdafx.hpp>
#include <mercury/nub.hpp>
#include <mercury/packet.hpp>
#include <mercury/channel.hpp>
#include <mercury/condemned_channels.hpp>
#include <common/service.hpp>
#include <boost/asio/post.hpp>
#ifndef _WIN32
#include <cerrno>
#endif

namespace Mercury
{

Nub::Nub(boost::asio::io_context & service, unsigned short port, ClientlessPacketHandler * clientlessHandler)
	: socket_(service, Address(boost::asio::ip::udp::v4(), port)),
	  tickInterval_(50),
	  lastTick_(0),
	  m_timer(service),
	  clientlessHandler_(clientlessHandler),
	  sendQueue_(TxSendQueueMaxSize),
	  isSending_(false),
	  sendBuffer_(MaxPacketLength)
{
	condemned_ = new CondemnedChannels(*this);
	unsigned int rxBufSize = atol(Service::instance().getConfigParameter("nub_receive_buffer").c_str()) * 1024;
	unsigned int txBufSize = atol(Service::instance().getConfigParameter("nub_send_buffer").c_str()) * 1024;
	boost::asio::ip::udp::socket::receive_buffer_size rxBufCmd(rxBufSize);
	boost::system::error_code ec;
	if (socket_.set_option(rxBufCmd, ec) != boost::system::errc::success)
		WARNC(CATEGORY_MERCURY, "Nub: Failed to resize receive buffer: %s", ec.message().c_str());
	boost::asio::ip::udp::socket::send_buffer_size txBufCmd(txBufSize);
	if (socket_.set_option(txBufCmd, ec) != boost::system::errc::success)
		WARNC(CATEGORY_MERCURY, "Nub: Failed to resize send buffer: %s", ec.message().c_str());

	receiveFrames();
	m_timer.expires_after(std::chrono::milliseconds(tickInterval_));
	m_timer.async_wait([this](const boost::system::error_code & err) { tick(err); });
}

Nub::~Nub()
{
	m_timer.cancel();
	delete condemned_;
}

Nub::Timer & Nub::timers()
{
	return timers_;
}

uint64_t Nub::lastTick() const
{
	return lastTick_;
}

void Nub::setTickInterval(uint32_t interval)
{
	tickInterval_ = interval;
}

void Nub::connectChannel(BaseChannel::Ptr channel, Address const & ep)
{
	channels_.insert(std::make_pair(ep, channel));
	channel->bind(ep);
}

void Nub::unregisterChannel(BaseChannel * /*channel*/, Address const & ep)
{
	auto iter = channels_.find(ep);
	if (iter != channels_.end())
	{
		DEBUG1C(CATEGORY_MERCURY, "Unregistering channel from address %s", ep.address().to_string().c_str());
		channels_.erase(iter);
		return;
	}

	FAULTC(CATEGORY_MERCURY, "Failed to unregister channel from nonexistent address %s", ep.address().to_string().c_str());
}

void Nub::condemn(BaseChannel::Ptr channel)
{
	condemned_->addChannel(channel);
}

BaseChannel::Ptr Nub::findChannel(Address const & address)
{
	auto iter = channels_.find(address);
	if (iter == channels_.end())
		return BaseChannel::Ptr();
	else
		return iter->second;
}

void Nub::frameHandler(const boost::system::error_code & error, size_t length, Address & address, ReceivedPacket::Ptr packet)
{
	if (error == boost::system::errc::operation_canceled)
		return;

	if (error == boost::system::errc::success)
	{
		SGW_ASSERT(receivedPacket_ == packet);
		receivedPacket_.reset();
		packet->resize(length);

		try
		{
			auto iter = channels_.find(address);
			if (iter != channels_.end())
			{
				if (iter->second->filter()->receiveMessage(*packet) && packet->unserialize(true))
					iter->second->handlePacket(socket_, packet, lastTick_);
			}
			else
			{
				// Clientless frame
				if (packet->unserialize(false, false))
					clientlessHandler_->handlePacket(*this, packet, address);
			}

			// Only we have references to the packet --> keep it for next receive operations
			if (packet->references() == 1)
				receivedPacket_ = packet;
		}
		catch (std::exception & e)
		{
			CRITICALC(CATEGORY_MERCURY, "Nub: Exception in frame handler: %s",  e.what());
			abort();
		}
	}
	#ifdef _WIN32
	else if (error.value() != WSAECONNREFUSED)
	#else
	else if (error.value() != ECONNREFUSED)
	#endif
		WARNC(CATEGORY_MERCURY, "Nub: Packet receive failed: %s", error.message().c_str());

	receiveFrames();
}

void Nub::receiveFrames()
{
	if (!receivedPacket_)
		receivedPacket_.reset(new ReceivedPacket());

	auto pkt = receivedPacket_;
	socket_.async_receive_from(
		boost::asio::buffer(receivedPacket_->buffer(), receivedPacket_->size()), receivedEndpoint_,
		[this, pkt](const boost::system::error_code & err, size_t len) {
			frameHandler(err, len, receivedEndpoint_, pkt);
		});
}

void Nub::tick(const boost::system::error_code & error)
{
	// Make sure that we don't tick if tick() is called after
	// canceling a timer (with "operation canceled" error code)
	if (error == boost::system::errc::operation_canceled)
		return;
	SGW_ASSERT(error == boost::system::errc::success);

	m_timer.expires_after(std::chrono::milliseconds(tickInterval_));
	m_timer.async_wait([this](const boost::system::error_code & err) { tick(err); });

	lastTick_ = Service::instance().microTime();

	timers_.tick(lastTick_);

	for (auto iter = channels_.begin(); iter != channels_.end();)
	{
		try
		{
			bool connected = iter->second->tick(lastTick_);
			if (!connected)
			{
				auto del = iter++;
				unregisterChannel(del->second.get(), del->first);
			}
			else
				++iter;
		}
		catch (std::exception & e)
		{
			WARNC(CATEGORY_MERCURY, "Nub::tick() exception: %s", e.what());
			++iter;
		}
	}
}

void Nub::queuePacket(BaseChannel::Ptr channel, Packet::Ptr packet, Packet::SequenceId sequenceId)
{
	if (!isSending_)
	{
		SGW_ASSERT(sendQueue_.empty());
		sendPacket(channel, packet, sequenceId);
	}
	else
	{
		if (!sendQueue_.full())
		{
			PendingPacket p;
			p.packet = packet;
			p.channel = channel;
			p.sequenceId = sequenceId;
			sendQueue_.push_back(p);
		}
		else
			WARNC(CATEGORY_MERCURY, "Couldn't queue outgoing packet: TxSendQueueMaxSize exceeded");
	}
}

void Nub::sendPacket(BaseChannel::Ptr channel, Packet::Ptr packet, Packet::SequenceId sequenceId)
{
	SGW_ASSERT(!isSending_);
	packet->send(sequenceId);

	if (!channel->filter()->sendMessage(*packet, sendBuffer_))
	{
		WARNC(CATEGORY_MERCURY, "Failed to filter outgoing packet");
		return;
	}

	#ifdef CHANNEL_DEBUG_MESSAGES
	if (packet->flags() != 0x4c)
	{
		TRACEC(CATEGORY_MERCURY, "Sending packet: flags 0x%02x, seq #%d, len %d, enclen %d, addr %s", 
			packet->flags(), sequenceId, packet->offset(), sendBuffer_.offset(),
			channel->address().address().to_string().c_str());
		packet->debugDump(packet->offset());
	}
	#endif
	channel->setLastActivity(lastTick_);
	isSending_ = true;

	socket_.async_send_to(
		boost::asio::buffer(sendBuffer_.buffer(), sendBuffer_.offset()),
		channel->address(),
		[this, channel, packet](const boost::system::error_code & err, std::size_t /*bytes*/) {
			onPacketSent(channel, packet, err);
		});
}

void Nub::onPacketSent(BaseChannel::Ptr /*channel*/, Packet::Ptr /*packet*/, const boost::system::error_code & errcode)
{
	SGW_ASSERT(isSending_);
	isSending_ = false;

	// TODO: Handle send failures (re-queue or wait for a tick)
	if (errcode != boost::system::errc::success)
		WARNC(CATEGORY_MERCURY, "Packet send failed: %s", errcode.message().c_str());

	if (!sendQueue_.empty())
	{
		PendingPacket & p = *sendQueue_.begin();
		sendPacket(p.channel, p.packet, p.sequenceId);
		sendQueue_.pop_front();
	}
}

}

