#include <stdafx.hpp>
#include <common/service.hpp>
#include <mercury/unified_connection.hpp>
#include <boost/python/errors.hpp>
#include <entity/pyutil.hpp>

namespace Mercury
{

UnifiedConnection::UnifiedConnection(uint32_t connectionId)
	: socket_(Service::instance().ioService()), connectionId_(connectionId), registered_(false),
	  connected_(false), writingMessage_(false), headerOffset_(0), writableBytes_(0),
	  received_(0x100000), sending_(0x100000)
{
}

UnifiedConnection::~UnifiedConnection()
{
	socket_.close();
	// TRACE("Connection closed");
}

void UnifiedConnection::connect(const std::string & address, uint16_t port)
{
	SGW_ASSERT(!connected_);

	boost::asio::ip::tcp::resolver resolver(Service::instance().ioService());
	auto iter = resolver.resolve(boost::asio::ip::tcp::resolver::query(boost::asio::ip::tcp::v4(), address, "1"));
	boost::asio::ip::tcp::endpoint ep = *iter;
	ep.port(port);
	auto self = shared_from_this();
	socket_.async_connect(ep, [self](const boost::system::error_code & err) {
		self->connectionCallback(err);
	});
}

void UnifiedConnection::connected()
{
	// TRACE("Connected");
	connected_ = true;
	registered_ = true;
	onConnected(boost::system::error_code(boost::system::errc::success, boost::system::generic_category()));
	receive();
}

void UnifiedConnection::connectionCallback(const boost::system::error_code & errcode)
{
	connected_ = (errcode == boost::system::errc::success);
	// TRACE("Connected");
	onConnected(errcode);
	if (errcode == boost::system::errc::success)
		receive();
}

boost::asio::ip::tcp::socket & UnifiedConnection::socket()
{
	return socket_;
}

uint32_t UnifiedConnection::connectionId() const
{
	return connectionId_;
}

void UnifiedConnection::close(const boost::system::error_code & errcode)
{
	if (connected_)
		onDisconnected(errcode);
	connected_ = false;

	if (socket_.is_open())
		socket_.close();

	if (registered_)
	{
		registered_ = false;
		unregisterConnection();
	}

	// Remove leftovers from the send and receive buffers
	received_.clear();
	sending_.clear();
	writableBytes_ = 0;
	writingMessage_ = false;
	headerOffset_ = 0;
}

void UnifiedConnection::close()
{
	close(boost::system::error_code());
}

bool UnifiedConnection::isConnected() const
{
	return socket_.is_open();
}

/**
 * Begin writing a message to the internal send buffer.
 * 
 * @param messageId Message ID (or command code)
 */
void UnifiedConnection::beginMessage(uint8_t messageId)
{
	SGW_ASSERT(!writingMessage_);
	if (!connected_)
	{
		throw std::runtime_error("Socket is not connected");
	}

	writingMessage_ = true;

	headerOffset_ = sending_.size();
	BufferWriter<Buffer> writer(sending_);
	MessageHeader header;
	header.length = 0;
	header.messageId = messageId;
	writer << header;
}

/**
 * Finish writing a message to the internal send buffer.
 * If no message is currently being transmitted, a send operation is started on the socket.
 */
void UnifiedConnection::endMessage()
{
	SGW_ASSERT(writingMessage_);
	bool writing = (writableBytes_ > 0);
	BufferWriter<Buffer> writer(sending_);
	writer.writeAt(headerOffset_, (uint32_t)(sending_.size() - headerOffset_));
	writableBytes_ = sending_.size();
	writingMessage_ = false;
	if (!writing)
	{
		send();
	}
}

/*
 * Rolls back the message being written (it won't be sent to the peer).
 * Can only be called for messages where endMessage() wasn't called yet!
 */
void UnifiedConnection::rollbackMessage()
{
	SGW_ASSERT(writingMessage_);
	sending_.eraseEnd(sending_.size() - writableBytes_);
	writingMessage_ = false;
}

/*
 * Called when the asynchronous socket receive call has completed and we have received some data.
 *
 * @param errcode Status code; receive is successful if == errc::success
 * @param receivedLength Amount of bytes received on socket
 */
void UnifiedConnection::onDataReceived(const boost::system::error_code & errcode, size_t receivedLength)
{
	if (errcode != boost::system::errc::success)
	{
		// WARN("Socket receive failed: %s", errcode.message().c_str());
		close(errcode);
		return;
	}

	received_.insertEnd(receivedLength);
	while (received_.size() > sizeof(MessageHeader))
	{
		MessageHeader header;
		BufferReader<Buffer> reader(received_);
		std::size_t receivedSize = received_.size();
		reader >> header;
		if (header.length < sizeof(MessageHeader))
		{
			WARN("Message length too short (got %d bytes), closing connection", header.length);
			close();
			return;
		}

		if (header.length > MaxMessageLength)
		{
			WARN("Message too long (got %d bytes, max is %d), closing connection", header.length, MaxMessageLength);
			close();
			return;
		}

		if (header.length > receivedSize)
		{
			received_.insertBegin(sizeof(header));
			break;
		}

		std::size_t endSize = receivedSize - header.length;
		Reader msgReader(*this, header.messageId, header.length - sizeof(MessageHeader));
		try
		{
			onMessageReceived(msgReader);
		} 
		catch (std::exception & e)
		{
			FAULT("Unhandled exception while processing message 0x%02x: %s", header.messageId, e.what());
		}
		// HACK: We need to catch this separately as error_already_set is not a child class of std::exception
		catch (bp::error_already_set &)
		{
			FAULT("Unhandled Python exception while processing message 0x%02x:", header.messageId);
			PyUtil_ShowErr();
		}

		// Stop processing messages if we were disconnected while processing the previous one
		if (!connected_)
		{
			SGW_ASSERT(received_.size() == 0);
			break;
		}

		SGW_ASSERT(received_.size() >= endSize);
		if (received_.size() > endSize)
		{
			std::size_t remaining = received_.size() - endSize;
			WARN("Handler for message 0x%02x did not consume all data (%d bytes left)", header.messageId, remaining);
			received_.eraseBegin(remaining);
		}
	}

	receive();
}

/*
 * Called when the asynchronous socket send call has completed.
 *
 * @param errcode Status code; sending is successful if == errc::success
 * @param receivedLength Amount of bytes sent on socket
 */
void UnifiedConnection::onDataSent(const boost::system::error_code & errcode, size_t sentLength)
{
	if (errcode != boost::system::errc::success)
	{
		WARN("Socket send failed: %s, closing connection", errcode.message().c_str());
		close(errcode);
		return;
	}

	// Don't do anything if we got disconnected while sending the message
	if (!connected_)
	{
		SGW_ASSERT(writableBytes_ == 0);
		return;
	}

	SGW_ASSERT(writableBytes_ >= sentLength);
	writableBytes_ -= sentLength;
	sending_.eraseBegin(sentLength);
	if (writableBytes_ > 0)
	{
		send();
	}
}

/*
 * Starts an asynchronous receive operation.
 * This is automatically called on connection and by onDataReceived(), don't call it manually!
 */
void UnifiedConnection::receive()
{
	if (received_.free() <= 1)
	{
		WARN("Ran out of buffer space while receiving message (buffered %d bytes), closing connection", received_.size());
		close();
		return;
	}

	auto self = shared_from_this();
	socket_.async_receive(
		boost::asio::buffer(received_.end(), received_.contiguousEndItems()),
		[self](const boost::system::error_code & err, size_t len) {
			self->onDataReceived(err, len);
		});
}

/*
 * Starts an asynchronous send operation.
 * This is automatically called by onDataSent() and endMessage(), don't call it manually!
 */
void UnifiedConnection::send()
{
	SGW_ASSERT(writableBytes_ > 0 && writableBytes_ <= sending_.size());
	std::size_t writeLength = (writableBytes_ < sending_.contiguousBeginItems()) ? writableBytes_ : sending_.contiguousBeginItems();

	auto self = shared_from_this();
	socket_.async_send(
		boost::asio::buffer(sending_.begin(), writeLength),
		[self](const boost::system::error_code & err, size_t len) {
			self->onDataSent(err, len);
		});
}

}
