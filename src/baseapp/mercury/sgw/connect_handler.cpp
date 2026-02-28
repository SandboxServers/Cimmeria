#include <stdafx.hpp>	
#include <baseapp/mercury/sgw/connect_handler.hpp>
#include <baseapp/mercury/sgw/client_handler.hpp>
#include <baseapp/mercury/sgw/messages.hpp>
#include <mercury/channel.hpp>
#include <mercury/nub.hpp>
#include <mercury/bundle.hpp>
#include <mercury/encryption_filter.hpp>
#include <authentication/shard_client.hpp>

namespace Mercury
{
namespace SGW
{

uint8_t hextobin(char c)
{
	if (c >= '0' && c <= '9')
		return c - '0';
	else if (c >= 'a' && c <= 'f')
		return c - 'a' + 0x0A;
	else if (c >= 'A' && c <= 'F')
		return c - 'A' + 0x0A;
	else
		throw Mercury::packet_processing_exception("Invalid hexadecimal character found in string!");
}

void ConnectHandler::handlePacket(Nub & nub, ReceivedPacket::Ptr packet, boost::asio::ip::udp::endpoint & endpoint)
{
	if (packet->flags() != (Packet::FLAG_HAS_REQUESTS | Packet::FLAG_HAS_SEQUENCE))
	{
		WARNC(CATEGORY_MESSAGES, "Illegal flags on connect message: %02x", packet->flags());
		return;
	}

	BundleUnpacker unpacker(ClientMessageTable, packet);
	SGW_ASSERT(unpacker.isComplete());
	unpacker.beginUnpack();
	Message::Ptr msg = unpacker.next();
	if (!msg)
		return;

	if (unpacker.next() != nullptr)
	{
		WARNC(CATEGORY_MESSAGES, "Connect message has multiple requests");
		return;
	}

	if (msg->messageId() != 0x00)
	{
		WARNC(CATEGORY_MESSAGES, "Illegal message id in connect message: %02x", msg->messageId());
		return;
	}

	uint32_t accountId, requestId;
	uint8_t ticketLength;

	requestId = msg->requestId();
	*msg >> accountId >> ticketLength;

	if (ticketLength != 20)
	{
		WARNC(CATEGORY_MESSAGES, "Invalid ticket length in connect message: %d", ticketLength);
		return;
	}

	char ticket[21];
	ticket[20] = 0;
	msg->read(ticket, ticketLength);

	TRACEC(CATEGORY_MESSAGES, "Request id: %08x, account id: %08x", requestId, accountId);
	TRACEC(CATEGORY_MESSAGES, "Ticket: %s", ticket);
	
	uint32_t access_level;
	std::string account_name, key_string;
	if (authentication::ShardLogonQueue::instance().openClientSession(accountId, account_name, access_level, ticket, key_string))
	{
		DEBUG1C(CATEGORY_MESSAGES, "Authenticated client from %s", endpoint.address().to_string().c_str());

		uint8_t key[32];
		for (uint32_t i = 0; i < 32; i++)
			key[i] = (hextobin(key_string[i << 1]) << 4) | hextobin(key_string[(i << 1) + 1]);

		EncryptionFilter * filter = new EncryptionFilter(key);
		ClientHandler * handler = new ClientHandler(accountId, account_name, access_level);
		BaseChannel::Ptr channel(new BaseChannel(nub, filter, handler, 
			ServerMessageTable, ClientMessageTable));
		handler->update_channel(channel);
		nub.connectChannel(channel, endpoint);
		
		// Send BaseApp connection ack message
		Bundle & bundle = *channel->bundle();
		bundle.beginMessage(BASEMSG_REPLY_MESSAGE, ServerMessageList[BASEMSG_AUTHENTICATE], Bundle::FLUSH);
		bundle << requestId << (uint8_t)0x20;
		bundle.write(key, sizeof(key));
		bundle.endMessage();
		// The auth message needs to be flushed in a separate packet
		// (at least that's how the SGW server does it, maybe it's not necessary)
		channel->flushBundle(false);

		handler->onConnected();
		TRACEC(CATEGORY_MESSAGES, "Connected to client at %s", endpoint.address().to_string().c_str());
	}
	else
		// TODO: Tickets should be able to expire (in ~60 sec?)
		WARNC(CATEGORY_MESSAGES, "Unable to authenticate ticket %s (maybe it expired?)", ticket);
}

}
}
