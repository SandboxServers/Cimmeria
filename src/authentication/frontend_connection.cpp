#include <stdafx.hpp>
#include <mercury/unified_protocol.hpp>
#include <authentication/service_main.hpp>
#include <authentication/frontend_connection.hpp>


FrontendConnection::FrontendConnection(uint32_t connectionId)
	: Mercury::UnifiedConnection(connectionId), status_(FES_NOT_CONNECTED),
	  lastLogonRequestId_(1)
{
}

FrontendConnection::~FrontendConnection()
{
}

FrontendConnection * FrontendConnection::construct(Mercury::TcpServer<FrontendConnection> & server, uint32_t connectionId)
{
	return new FrontendConnection(connectionId);
}

void FrontendConnection::kick_account(uint32_t accountId)
{
	Writer request(*this);
	request.beginMessage(Mercury::FES_KICK_PLAYER);
	request << accountId;
	request.endMessage();
}

void FrontendConnection::logon(uint32_t accountId, std::string const & accountName, uint32_t accessLevel,
			LogonSuccessCallback successCb, LogonFailureCallback failureCb)
{
	LogonRequest req;
	req.accountId = accountId;
	req.successCb = successCb;
	req.failureCb = failureCb;
	uint32_t requestId = lastLogonRequestId_++;
	pendingLogins_.insert(std::make_pair(requestId, req));
	
	uint64_t now = Service::instance().microTime();
	pendingLogins_[requestId].replyTimer = Service::instance().addTimer(
		[this, requestId](auto &, auto, auto, auto *) { onRequestTimeout(requestId); },
		now + LoginRequestTimeout);
	
	Writer request(*this);
	request.beginMessage(Mercury::FES_REQUEST_LOGON);
	request << (uint32_t)requestId << (uint32_t)accountId << (uint32_t)accessLevel;
	request.writeString(accountName);
	request.endMessage();
}

void FrontendConnection::unregisterConnection()
{
	AuthenticationService::as_instance().frontendServer().unregister_connection(shared_this());
}

void FrontendConnection::onMessageReceived(Reader & msg)
{
	// INFO("Received message 0x%04x", msg.messageId());
	try
	{
		if (status_ == FES_REGISTERED)
		{
			switch (msg.messageId())
			{
				case Mercury::FES_UPDATE_SHARD_STATUS:
					onShardUpdate(msg);
					break;

				case Mercury::FES_LOGON_NOTIFICATION:
					onLogonNotification(msg);
					break;

				case Mercury::FES_LOGOFF_NOTIFICATION:
					onLogoffNotification(msg);
					break;

				case Mercury::FES_LOGON_ACK:
					onLogonAck(msg);
					break;

				case Mercury::FES_LOGON_NAK:
					onLogonRejected(msg);
					break;

				default:
					WARN("Unknown message 0x%04x received from BaseApp", msg.messageId());
			}
		}
		else
		{
			if (msg.messageId() == Mercury::FES_REGISTER_SHARD)
			{
				onShardRegistered(msg);
			}
			else
			{
				WARN("BaseApp: expected FES_REGISTER_SHARD, got cmd 0x%04x", msg.messageId());
				close();
			}
		}
	}
	catch (std::exception & e)
	{
		WARN("%s", e.what());
	}
}

void FrontendConnection::onConnected(const boost::system::error_code & errcode)
{
	assert(errcode == boost::system::errc::success);
	// INFO("Client connected");
	status_ = FES_CONNECTED;
}

void FrontendConnection::onDisconnected(const boost::system::error_code & errcode)
{
	// INFO("Client disconnected");

	for (std::map<uint32_t, LogonRequest>::iterator iter = pendingLogins_.begin();
		iter != pendingLogins_.end(); ++iter)
	{
		Service::instance().cancelTimer(iter->second.replyTimer);
		iter->second.failureCb(LogonQueue::ShardLost);
	}

	if (status_ == FES_REGISTERED)
		AuthenticationService::as_instance().unregisterShard(shared_this());

	status_ = FES_NOT_CONNECTED;
}

void FrontendConnection::sendConnectReply(uint16_t responseCode)
{
	Writer reply(*this);
	reply.beginMessage(Mercury::FES_REGISTER_SHARD_ACK);
	reply << (uint16_t)responseCode;
	reply.endMessage();
}

void FrontendConnection::onShardRegistered(Reader & msg)
{
	if (status_ != FES_CONNECTED)
	{
		WARN("Registration request received, but the shard is already registered");
		return;
	}
	
	std::string authkey, shardName, shardKey, shardProtected;
	soci::indicator shardIndicator;
	msg >> shardId_;
	msg.readString(shardName);
	msg.readString(authkey);
	
	try
	{
		soci::session & sess = AuthenticationService::as_instance().db();
		sess << "SELECT name, key, protected FROM shards WHERE shard_id = :id",
			soci::use(shardId_), soci::into(shardName, shardIndicator), soci::into(shardKey), soci::into(shardProtected);

		if (!sess.got_data())
		{
			WARN("Shard registration failed: No data found for shard id %d", shardId_);
			sendConnectReply(INVALID_SHARD_ID);
			return;
		}

		if (authkey != shardKey)
		{
			WARN("Shard registration failed: Invalid authentication key for shard id %d", shardId_);
			sendConnectReply(INVALID_AUTH_KEY);
			return;
		}

		if (shardName != "" && shardIndicator == soci::i_ok)
		{
			if (shardName.length() > 100)
			{
				WARN("Shard registration failed: Shard name too long!");
				sendConnectReply(INTERNAL_ERROR);
				return;
			}
			shardName_ = shardName;
		}
		shardProtected_ = (shardProtected == "t");
		shardName_ = shardName;
	}
	catch (soci::soci_error & e)
	{
		sendConnectReply(INTERNAL_ERROR);
		FAULT("Database error: %s", e.what());
		return;
	}

	playerCount_ = serverLoad_ = serverFlags_ = 0;

	if (AuthenticationService::as_instance().registerShard(shared_this()))
	{
		status_ = FES_REGISTERED;
		sendConnectReply(SUCCESS);
		INFO("Registered shard '%s'", shardName_.c_str());
	}
	else
	{
		WARN("Shard registration rejected for '%s'", shardName_.c_str());
		sendConnectReply(ALREADY_REGISTERED);
	}
}

void FrontendConnection::onShardUpdate(Reader & msg)
{
	TRACE("Received shard update for shard '%s'", shardName_.c_str());
	msg >> playerCount_ >> serverLoad_ >> serverFlags_;
}

void FrontendConnection::onLogonNotification(Reader & msg)
{
	uint32_t accountId;
	msg >> accountId;
	AuthenticationService::as_instance().accountOnline(shared_this(), accountId);
	TRACE("Account id %d logged to shard '%s'", accountId, shardName_.c_str());
}

void FrontendConnection::onLogoffNotification(Reader & msg)
{
	uint32_t accountId;
	msg >> accountId;
	AuthenticationService::as_instance().accountOffline(shared_this(), accountId);
	TRACE("Account id %d logged off from shard '%s'", accountId, shardName_.c_str());
}

void FrontendConnection::onLogonAck(Reader & msg)
{
	ShardSessionData session;
	uint32_t requestId;
	msg >> requestId >> session.mailboxId;
	msg.readString(session.ticketId);
	msg.readString(session.sessionKey);
	msg.readString(session.address);
	msg >> session.port;

	auto iter = pendingLogins_.find(requestId);
	if (iter != pendingLogins_.end())
	{
		DEBUG2("Received acknowledgement response to logon rid %d", requestId);
		Service::instance().cancelTimer(iter->second.replyTimer);
		iter->second.successCb(std::ref(session));
		pendingLogins_.erase(iter);
	}
	else
	{
		WARN("Received response to unknown request rid %d (maybe it timed out?)", requestId);
	}
}

void FrontendConnection::onLogonRejected(Reader & msg)
{
	uint32_t requestId, reasonCode;
	msg >> requestId >> reasonCode;

	auto iter = pendingLogins_.find(requestId);
	if (iter != pendingLogins_.end())
	{
		DEBUG2("Received negative acknowledgement response to logon rid %d", requestId);
		if (reasonCode > LogonQueue::FailureMax)
			reasonCode = LogonQueue::InternalError;
		Service::instance().cancelTimer(iter->second.replyTimer);
		iter->second.failureCb(static_cast<LogonQueue::FailureCode>(reasonCode));
		pendingLogins_.erase(iter);
	}
	else
	{
		WARN("Received response to unknown request rid %d (maybe it timed out?)", requestId);
	}
}

void FrontendConnection::onRequestTimeout(uint32_t requestId)
{
	auto iter = pendingLogins_.find(requestId);
	if (iter == pendingLogins_.end())
	{
		FAULT("Unknown request ID %d in pending login list", requestId);
		return;
	}

	WARN("Request to shard \"%s\" timed out (rid: %d)", shardName_.c_str(), requestId);
	iter->second.failureCb(LogonQueue::ShardRequestTimedOut);
	pendingLogins_.erase(iter);
}