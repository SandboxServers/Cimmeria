#include <stdafx.hpp>
#include <authentication/service_main.hpp>
#include <boost/lexical_cast.hpp>

AuthenticationService::AuthenticationService()
{
	configVersion_ = "20131025";
}

void AuthenticationService::initialize()
{
	serviceName_ = "AuthenticationService";
	loginServer_.reset(
		new Mercury::TcpServer<LogonConnection>(
			boost::lexical_cast<uint16_t>(getConfigParameter("logon_service_port")),
			LogonConnection::construct
		)
	);
	
	frontendServer_.reset(
		new Mercury::TcpServer<FrontendConnection>(
			boost::lexical_cast<uint16_t>(getConfigParameter("base_service_port")),
			FrontendConnection::construct
		)
	);
}

void AuthenticationService::cleanup()
{
	frontendServer_.reset();
	loginServer_.reset();
}

std::string AuthenticationService::internalServiceName()
{
	return "AuthenticationService";
}

FrontendConnection::Ptr AuthenticationService::shard(const std::string & shardName)
{
	for (auto iter = shards_.begin(); iter != shards_.end(); ++iter)
	{
		if ((*iter)->shardName() == shardName)
			return *iter;
	}

	return FrontendConnection::Ptr();
}

bool AuthenticationService::registerShard(FrontendConnection::Ptr connection)
{
	for (auto iter = shards_.begin(); iter != shards_.end(); ++iter)
	{
		if ((*iter)->shardName() == connection->shardName() ||
			(*iter)->shardId() == connection->shardId())
		{
			WARN("A shard collision was detected!");
			WARN("Offending shard: %d, '%s'", connection->shardId(), connection->shardName().c_str());
			WARN("Registered shard: %d, '%s'", (*iter)->shardId(), (*iter)->shardName().c_str());
			return false;
		}
	}

	INFO("Registered shard: %d, '%s'", connection->shardId(), connection->shardName().c_str());
	shards_.push_back(connection);
	return true;
}

void AuthenticationService::unregisterShard(FrontendConnection::Ptr connection)
{
	for (auto iter = shards_.begin(); iter != shards_.end(); ++iter)
	{
		if (*iter == connection)
		{
			INFO("Unregistered shard: %d, '%s'", (*iter)->shardId(), (*iter)->shardName().c_str());
			for (auto acct_iter = onlineAccounts_.begin(); acct_iter != onlineAccounts_.end();)
			{
				if (acct_iter->second == connection)
				{
					TRACE("Forced logoff for account id %d", acct_iter->first);
					auto del_iter = acct_iter;
					++acct_iter;
					onlineAccounts_.erase(del_iter);
				}
				else
					++acct_iter;
			}

			shards_.erase(iter);
			return;
		}
	}

	WARN("Failed to unregistered shard: %d, '%s' (not registered)", connection->shardId(), connection->shardName().c_str());
}


void AuthenticationService::accountOnline(FrontendConnection::Ptr connection, uint32_t accountId)
{
	auto iter = onlineAccounts_.find(accountId);
	if (iter != onlineAccounts_.end())
	{
		WARN("Logon requested for account %d from shard '%s', but it is already connected to shard '%s'!", 
			iter->first, connection->shardName().c_str(), iter->second->shardName().c_str());
		WARN("Resolution: killing both clients");
		connection->kick_account(accountId);
		iter->second->kick_account(accountId);
	}
	else
	{
		TRACE("Account id %d logged on", accountId);
		onlineAccounts_.insert(std::make_pair(accountId, connection));
	}
}

void AuthenticationService::accountOffline(FrontendConnection::Ptr connection, uint32_t accountId)
{
	auto iter = onlineAccounts_.find(accountId);
	if (iter != onlineAccounts_.end())
	{
		if (iter->second == connection)
		{
			TRACE("Account id %d logged off", iter->first);
			onlineAccounts_.erase(iter);
		}
		else
		{
			WARN("Logoff requested for account %d from shard '%s', but it is connected to shard '%s'!", 
				iter->first, connection->shardName().c_str(), iter->second->shardName().c_str());
			WARN("Resolution: killing client on second shard as well");
			iter->second->kick_account(accountId);
		}
	}
	else
	{
		WARN("Logoff requested for account %d from shard '%s', but it is not connected!", 
			iter->first, connection->shardName().c_str());
	}
}

void AuthenticationService::registerSession(const std::string & sessionId, uint32_t accountId, std::string const & accountName, uint32_t accessLevel)
{
	auto iter = sessions_.find(sessionId);
	if (iter != sessions_.end())
	{
		iter->second.accountId = accountId;
		iter->second.accountName = accountName;
		iter->second.accessLevel = accessLevel;
	}
	else
	{
		LoginSession sess;
		sess.accountId = accountId;
		sess.accountName = accountName;
		sess.accessLevel = accessLevel;
		sessions_.insert(std::make_pair(sessionId, sess));
	}
}

void AuthenticationService::unregisterSession(const std::string & sessionId)
{
	auto iter = sessions_.find(sessionId);
	if (iter != sessions_.end())
		sessions_.erase(iter);
}

AuthenticationService::LoginSession * AuthenticationService::getSession(const std::string & sessionId)
{
	auto iter = sessions_.find(sessionId);
	if (iter == sessions_.end())
		return nullptr;
	else
		return &iter->second;
}


MessageWriter * AuthenticationService::messageWriter(EndpointType endpoint, uint32_t entityId)
{
	throw std::runtime_error("Entity messages not supported on the authentication server");
}
