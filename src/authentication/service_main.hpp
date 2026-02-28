#pragma once

#include <common/service.hpp>
#include <mercury/tcp_server.hpp>
#include <authentication/logon_connection.hpp>
#include <authentication/frontend_connection.hpp>
#include <authentication/logon_queue.hpp>

class AuthenticationService : public Service
{
	public:
		struct LoginSession
		{
			uint32_t accountId, accessLevel;
			std::string accountName;
		};

		AuthenticationService();

		inline static AuthenticationService & as_instance()
		{
			return reinterpret_cast<AuthenticationService &>(instance());
		}

		inline LogonQueue & logonQueue()
		{
			return logonQueue_;
		}

		inline Mercury::TcpServer<LogonConnection> & loginServer()
		{
			return *loginServer_;
		}

		inline Mercury::TcpServer<FrontendConnection> & frontendServer()
		{
			return *frontendServer_;
		}

		inline const std::vector<FrontendConnection::Ptr> & shards()
		{
			return shards_;
		}

		bool registerShard(FrontendConnection::Ptr connection);
		void unregisterShard(FrontendConnection::Ptr connection);
		void accountOnline(FrontendConnection::Ptr connection, uint32_t accountId);
		void accountOffline(FrontendConnection::Ptr connection, uint32_t accountId);

		void registerSession(const std::string & sessionId, uint32_t accountId, std::string const & accountName, uint32_t accessLevel);
		void unregisterSession(const std::string & sessionId);
		LoginSession * getSession(const std::string & sessionId);

		FrontendConnection::Ptr shard(const std::string & shardName);
		
		virtual MessageWriter * messageWriter(EndpointType endpoint, uint32_t entityId);

	protected:
		virtual void initialize();
		virtual void cleanup();
		virtual std::string internalServiceName();
		
		std::unique_ptr<Mercury::TcpServer<FrontendConnection>> frontendServer_;
		std::unique_ptr<Mercury::TcpServer<LogonConnection>> loginServer_;
		LogonQueue logonQueue_;
		std::map<uint32_t, FrontendConnection::Ptr> onlineAccounts_;
		std::vector<FrontendConnection::Ptr> shards_;
		std::map<std::string, LoginSession> sessions_;
};

