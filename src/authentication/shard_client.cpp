#include <stdafx.hpp>
#include <authentication/shard_client.hpp>
#include <mercury/unified_protocol.hpp>
#include <common/service.hpp>
#include <baseapp/cell_manager.hpp>
#include <authentication/logon_queue.hpp>
#include <boost/lexical_cast.hpp>

namespace authentication
{

ShardLogonQueue::ShardLogonQueue()
{
}

ShardLogonQueue::~ShardLogonQueue()
{
}

void ShardLogonQueue::logonClient(uint32_t accountId, std::string const & accountName, uint32_t accessLevel, std::string const & ticketId, std::string const & key)
{
	checkExpiration();

	QueueEntry & ent = pendingLogins_[accountId];
	ent.accountId = accountId;
	ent.accountName = accountName;
	ent.accessLevel = accessLevel;
	ent.ticketId = ticketId;
	ent.key = key;
	ent.expiration = Service::instance().microTime();
}

bool ShardLogonQueue::openClientSession(uint32_t accountId, std::string & accountName, uint32_t & accessLevel, std::string const & ticketId, std::string & key)
{
	checkExpiration();

	auto iter = pendingLogins_.find(accountId);
	if (iter != pendingLogins_.end() && iter->second.ticketId == ticketId)
	{
		accountName = iter->second.accountName;
		accessLevel = iter->second.accessLevel;
		key = iter->second.key;
		pendingLogins_.erase(iter);
		return true;
	}
	else
		return false;
}

/*
 * Checks the expiration of pending logon requests for the BaseApp.
 * If any of them is expired, it will be removed.
 */
void ShardLogonQueue::checkExpiration()
{
	uint64_t now = Service::instance().microTime();
	for (auto it = pendingLogins_.begin(); it != pendingLogins_.end();)
	{
		if (it->second.expiration >= now)
		{
			auto delIt = it++;
			WARNC(CATEGORY_MESSAGES, "Login ticket %s for account %d has expired and will be removed", 
				it->second.ticketId.c_str(), it->second.accountId);
			pendingLogins_.erase(delIt);
		}
		else
		{
			++it;
		}
	}
}


ShardClient::ShardClient()
	: Mercury::UnifiedConnection(0),
	  recoveryTimer_(Service::instance().ioService()),
	  state_(STATE_NOT_CONNECTED)
{
}

ShardClient::~ShardClient()
{
	TRACE("");
}

void ShardClient::reconnectIn(uint32_t msecs)
{
	state_ = STATE_NOT_CONNECTED;

	recoveryTimer_.expires_after(std::chrono::milliseconds(msecs));
	auto self = shared_this();
	recoveryTimer_.async_wait([self](const boost::system::error_code & err) {
		self->reconnectTimerExpired(err);
	});
}

void ShardClient::startup()
{
	SGW_ASSERT(!isConnected());
	SGW_ASSERT(state_ == STATE_NOT_CONNECTED);
	reconnect();
}

void ShardClient::reconnectTimerExpired(const boost::system::error_code & errcode)
{
	if (errcode == boost::system::errc::success)
	{
		reconnect();
	}
	else
	{
		close();
	}
}

void ShardClient::reconnect()
{
	TRACE("Connecting to Authentication Service ...");
	connect(
		Service::instance().getConfigParameter("auth_server_address"),
		(uint16_t)atol(Service::instance().getConfigParameter("auth_server_port").c_str())
	);
}

void ShardClient::onRegistrationAck(Reader & msg)
{
	uint16_t responseCode;
	msg >> responseCode;

	if (responseCode == 0)
	{
		INFO("Registered shard with Authentication Service");
		state_ = STATE_REGISTERED;
	}
	else
	{
		WARN("Failed to register shard with Authentication Service: error %d", responseCode);
		WARN("Will retry connection in %d seconds", ConnectionRecoveryTimeout / 1000);
		close();
	}
}

void ShardClient::onKickPlayer(Reader & msg)
{
	uint32_t accountId;
	msg >> accountId;

	WARN("FES_KICK_PLAYER: Not implemented!");
}

void ShardClient::onRequestLogon(Reader & msg)
{
	uint32_t requestId, accountId, accessLevel;
	std::string accountName;
	msg >> requestId >> accountId >> accessLevel;
	msg.readString(accountName);
	
	// We cannot send the player anywhere if no CellApps are connected --> refuse request
	if (!CellManager::instance().hasCells())
	{
		sendLogonNak(requestId, LogonQueue::NoCellAppsAvailable);
		return;
	}

	std::string ticketId = "00112233445566778899";
	std::string sessionKey = "0000000000000000000000000000000000000000000000000000000000000000";
	generateRandom(ticketId);
	generateRandom(sessionKey);

	// Register the request locally so we can find the session details from the ticket ID
	ShardLogonQueue::instance().logonClient(accountId, accountName, accessLevel, ticketId, sessionKey);
	sendLogonAck(requestId, accountId, ticketId, sessionKey);
}


/**
 * Sends a login acknowledgement message to an outstanding login request.
 *
 * @param requestId  Request ID the authentication server sent us -- this identifies the client trying to log in
 * @param accountId  Account ID of the user
 * @param ticketId   Ticket ID that was generated locally; the client needs to send this for authentication
 * @param sessionKey AES encryption key used for the BigWorld session; the client needs to send this for authentication
 */
void ShardClient::sendLogonAck(uint32_t requestId, uint32_t accountId, std::string const & ticketId, std::string const & sessionKey)
{
	Writer reply(*this);
	reply.beginMessage(Mercury::FES_LOGON_ACK);
	reply << requestId << (uint32_t)accountId;
	reply.writeString(ticketId);
	reply.writeString(sessionKey);
	reply.writeString(Service::instance().getConfigParameter("shard_external_address"));
	reply << boost::lexical_cast<uint16_t>(Service::instance().getConfigParameter("shard_port"));
	reply.endMessage();
}


/**
 * Sends a login negative acknowledgement (failure) message to an outstanding login request.
 *
 * @param requestId   Request ID the authentication server sent us; identifies the client trying to log in
 * @param failureCode Reason for refusing login
 */
void ShardClient::sendLogonNak(uint32_t requestId, uint32_t failureCode)
{
	Writer reply(*this);
	reply.beginMessage(Mercury::FES_LOGON_NAK);
	reply << requestId << failureCode;
	reply.endMessage();
}


/**
 * Sends a registration request to the auth server to add this shard to the running shard list.
 */
void ShardClient::sendRegistrationRequest()
{
	Writer request(*this);
	request.beginMessage(Mercury::FES_REGISTER_SHARD);
	request << boost::lexical_cast<uint32_t>(Service::instance().getConfigParameter("shard_id"));
	request.writeString(Service::instance().getConfigParameter("shard_name"));
	request.writeString(Service::instance().getConfigParameter("shard_auth_key"));
	request.endMessage();
}


void ShardClient::generateRandom(std::string & s)
{
	const char charset[17] = "0123456789ABCDEF";
	std::uniform_int_distribution<> distribution(0, 15);

	for (std::string::size_type i = 0; i < s.length(); i++)
		s[i] = charset[distribution(Service::instance().rng())];
}


/**
 * Called when a message was received from the peer
 *
 * @param msg Message stream reader
 */
void ShardClient::onMessageReceived(Reader & msg)
{
	switch (msg.messageId())
	{
		case Mercury::FES_REGISTER_SHARD_ACK:
			onRegistrationAck(msg);
			break;

		case Mercury::FES_KICK_PLAYER:
			onKickPlayer(msg);
			break;

		case Mercury::FES_REQUEST_LOGON:
			onRequestLogon(msg);
			break;

		default:
			WARN("Unknown message received: %04x", msg.messageId());
			close();
	}
}


/**
 * Callback function called when the connection request was completed.
 *
 * @param errcode Error code; connection was successful if code == errc::success
 */
void ShardClient::onConnected(const boost::system::error_code & errcode)
{
	// The connection was established successfully
	if (errcode == boost::system::errc::success)
	{
		// INFO("Connected to Authentication Service");
		state_ = STATE_CONNECTED;
		sendRegistrationRequest();
	}
	else
	{
		WARN("Connection to Authentication Service failed: %s", errcode.message().c_str());
		reconnectIn(ConnectionRecoveryTimeout);
	}
}


/**
 * Callback function called when the connection to the server was closed.
 *
 * @param errcode Reason why the connection was lost
 */
void ShardClient::onDisconnected(const boost::system::error_code & /*errcode*/)
{
	WARN("Lost connection to Authentication Service");
	reconnectIn(ConnectionRecoveryTimeout);
}

void ShardClient::unregisterConnection()
{
}

}
