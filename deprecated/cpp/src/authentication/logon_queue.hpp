#pragma once


class LogonQueue
{
	public:
		enum FailureCode
		{
			// No error occured
			Success = 0,
			// Illegal username in request
			MalformedUserId,
			// Illegal password in request
			MalformedPassword,
			// Invalid product SKU / service name
			InvalidService,
			// Invalid username or password
			BadUserPassword,
			// Account is disabled (accounts.enabled = false)
			AccountDisabled,
			// Account does not have sufficient access level to log in to this shard
			AccessDenied,
			// No shards are available to the authentication server
			NoServersAvailable,
			// Tried to log in to nonexistent shard
			NoSuchServer,
			// The selected shard is currently offline
			ServerOffline,
			// A request to the database server has failed
			DbRequestFailed,
			// A request to the shard timed out
			ShardRequestTimedOut,
			// Lost connection to shard while waiting for a response
			ShardLost,
			// Internal error
			InternalError,
			// The shard rejected the logon attempt
			ShardRejected,
			// HTTP session cookie expired
			SessionExpired,
			// No CellApps are available to the BaseApp
			NoCellAppsAvailable,
			// This client version is not supported
			VersionMismatch,
			// This should contain the highest possible failure code
			FailureMax = VersionMismatch
		};

		typedef boost::function<void (uint32_t /*accountId*/, std::string const &, uint32_t /*accessLevel*/)> SuccessCallback;
		typedef boost::function<void (FailureCode /*errorMsg*/)> FailureCallback;

		struct Request
		{
			std::string serviceName, userName, password;
			SuccessCallback successCb;
			FailureCallback errorCb;
		};

		LogonQueue();
		~LogonQueue();

		void queueRequest(Request const & request);

	private:
		struct QueueItem
		{
			typedef boost::shared_ptr<QueueItem> Ptr;

			Request request;
			std::string dbPassword;
			int accountId, accessLevel;
			std::string enabled;
		};

		std::mutex lock_;
		std::queue<QueueItem::Ptr> queue_;

		void processQueueItem();
		static void doLogonQuery(soci::session & s, QueueItem::Ptr req);
};

