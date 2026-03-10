#pragma once

#include <mercury/unified_connection.hpp>
#include <mercury/tcp_server.hpp>


class FrontendConnection : public Mercury::UnifiedConnection
{
	public:
		struct ShardSessionData
		{
			uint32_t mailboxId;
			std::string ticketId;
			std::string sessionKey;
			std::string address;
			uint16_t port;
		};

		typedef std::function<void (ShardSessionData /* session */)> LogonSuccessCallback;
		typedef std::function<void (LogonQueue::FailureCode /* error_code */)> LogonFailureCallback;

		enum ConnectReplyCode
		{
			SUCCESS = 0,
			ALREADY_REGISTERED = 1,
			INVALID_SHARD_ID = 2,
			INVALID_AUTH_KEY = 3,
			INTERNAL_ERROR = 4
		};

		// Login requests to the BaseApp time out after this interval (milliseconds)
		static const unsigned int LoginRequestTimeout = 5000;

		typedef std::shared_ptr<FrontendConnection> Ptr;

		FrontendConnection(uint32_t connectionId);
		~FrontendConnection();

		void kick_account(uint32_t accountId);
		void logon(uint32_t accountId, std::string const & accountName, uint32_t accessLevel,
			LogonSuccessCallback successCb, LogonFailureCallback failureCb);

		static FrontendConnection * construct(Mercury::TcpServer<FrontendConnection> & server, uint32_t connection_id);

		const std::string & shardName() const
		{
			return shardName_;
		}

		uint32_t shardId() const
		{
			return shardId_;
		}

		bool isProtected() const
		{
			return shardProtected_;
		}

		Ptr shared_this()
		{
			return boost::static_pointer_cast<FrontendConnection>(shared_from_this());
		}

	protected:
		// Overridden handlers of unified connection
		virtual void onMessageReceived(Reader & msg);
		virtual void onConnected(const boost::system::error_code & errcode);
		virtual void onDisconnected(const boost::system::error_code & errcode);
		virtual void unregisterConnection();

		void sendConnectReply(uint16_t responseCode);

		// Message handlers
		void onShardRegistered(Reader & msg);
		void onShardUpdate(Reader & msg);
		void onLogonNotification(Reader & msg);
		void onLogoffNotification(Reader & msg);
		void onLogonAck(Reader & msg);
		void onLogonRejected(Reader & msg);

		void onRequestTimeout(uint32_t requestId);

	private:
		struct LogonRequest
		{
			uint32_t accountId;
			LogonSuccessCallback successCb;
			LogonFailureCallback failureCb;
			Service::TimerMgr::TimerId replyTimer;
		};

		enum Status
		{
			FES_NOT_CONNECTED,
			FES_CONNECTED,
			FES_REGISTERED
		};

		Status status_;
		std::string shardName_;
		uint32_t shardId_;
		bool shardProtected_;
		uint32_t playerCount_, serverLoad_, serverFlags_;

		std::map<uint32_t, LogonRequest> pendingLogins_;
		uint32_t lastLogonRequestId_;
};

