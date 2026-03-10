#pragma once

#include <mercury/unified_connection.hpp>
#include <util/singleton.hpp>
#include <boost/asio/steady_timer.hpp>

namespace authentication
{

class ShardLogonQueue : public singleton<ShardLogonQueue>
{
	public:
		// How long will we wait for the game client before we'll mark its ticket as expired
		static const unsigned int TicketExpiration = 30000;

		struct QueueEntry
		{
			uint32_t accountId;
			std::string accountName;
			uint32_t accessLevel;
			std::string ticketId, key;
			uint64_t expiration;
		};

		ShardLogonQueue();
		~ShardLogonQueue();

		void logonClient(uint32_t accountId, std::string const & accountName, uint32_t accessLevel, std::string const & ticketId, std::string const & key);
		bool openClientSession(uint32_t accountId, std::string & accountName, uint32_t & accessLevel, std::string const & ticketId, std::string & key);

	private:
		std::map<uint32_t, QueueEntry> pendingLogins_;

		/*
		 * Checks the expiration of pending logon requests for the BaseApp.
		 * If any of them is expired, it will be removed.
		 */
		void checkExpiration();
};

class ShardClient : public Mercury::UnifiedConnection
{
	public:
		enum State
		{
			// We're not connected to the AS
			STATE_NOT_CONNECTED,
			// Connected to AS but no reply received to the registration request yet
			STATE_CONNECTED,
			// Connected and registered
			STATE_REGISTERED
		};

		// Sleep time after an operation failed (name lookup/connection/registration failed)
		const static uint32_t ConnectionRecoveryTimeout = 30000;
		typedef std::shared_ptr<ShardClient> Ptr;

		ShardClient();
		~ShardClient();

		void startup();

	protected:
		// Overridden handlers of unified connection

		/**
		 * Called when a message was received from the peer. The stream contains
		 * message contents excluding the header and message ID.
		 *
		 * @param msg Message stream reader
		 */
		virtual void onMessageReceived(Reader & msg);

		/**
		 * Callback function called when the connection request was completed.
		 *
		 * @param errcode Error code; connection was successful if code == errc::success
		 */
		virtual void onConnected(const boost::system::error_code & errcode);

		/**
		 * Callback function called when the connection to the server was closed.
		 *
		 * @param errcode Reason why the connection was lost
		 */
		virtual void onDisconnected(const boost::system::error_code & errcode);
		virtual void unregisterConnection();

	private:
		boost::asio::steady_timer recoveryTimer_;
		State state_;

		void reconnectIn(uint32_t msecs);
		void reconnectTimerExpired(const boost::system::error_code & errcode);
		void reconnect();
		void discoverTimeout(const boost::system::error_code & error);

		void onRegistrationAck(Reader & msg);
		void onKickPlayer(Reader & msg);
		void onRequestLogon(Reader & msg);

		/**
		 * Sends a login acknowledgement message to an outstanding login request.
		 *
		 * @param requestId  Request ID the authentication server sent us; identifies the client trying to log in
		 * @param accountId  Account ID of the user
		 * @param ticketId   Ticket ID that was generated locally; the client needs to send this for authentication
		 * @param sessionKey AES encryption key used for the BigWorld session; the client needs to send this for authentication
		 */
		void sendLogonAck(uint32_t requestId, uint32_t accountId, std::string const & ticketId, std::string const & sessionKey);

		/**
		 * Sends a login negative acknowledgement (failure) message to an outstanding login request.
		 *
		 * @param requestId   Request ID the authentication server sent us; identifies the client trying to log in
		 * @param failureCode Reason for refusing login
		 */
		void sendLogonNak(uint32_t requestId, uint32_t failureCode);

		/**
		 * Sends a registration request to the auth server to add this shard to the running shard list.
		 */
		void sendRegistrationRequest();

		void generateRandom(std::string & s);

		/**
		 * Returns a ShardClient shared ptr from enable_shared_from_this<>
		 */
		inline Ptr shared_this()
		{
			return std::static_pointer_cast<ShardClient>(shared_from_this());
		}
};

}
