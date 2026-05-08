#pragma once

#include <authentication/logon_queue.hpp>
#include <authentication/frontend_connection.hpp>

class LogonConnection : public boost::enable_shared_from_this<LogonConnection>
{
	public:
		typedef boost::shared_ptr<LogonConnection> Ptr;
		// Max length of HTTP request in bytes
		static const size_t MaxRequestLength = 0x10000;

		LogonConnection(uint32_t connectionId);
		~LogonConnection();

		static LogonConnection * construct(Mercury::TcpServer<LogonConnection> & server, uint32_t connectionId);

		boost::asio::ip::tcp::socket & socket();
		uint32_t connectionId() const;
		
		void connected();
		void sendReply(const std::string & response);
		void close(const boost::system::error_code & errcode);

	protected:
		void onDisconnected(const boost::system::error_code & errcode);

		void onUserAuthentication(const std::string & request);
		void onServerSelection(const std::string & request);

		void onLogonSuccessful(uint32_t accountId, std::string const & accountName, uint32_t accessLevel);
		void onLogonFailed(LogonQueue::FailureCode errorCode);

		void onShardEnterSuccessful(FrontendConnection::ShardSessionData & session, FrontendConnection::Ptr connection);
		void onShardEnterFailed(LogonQueue::FailureCode errorCode);

	private:
		// Connection handle
		boost::asio::ip::tcp::socket socket_;

		// Temporary buffer for holding request
		std::array<uint8_t, MaxRequestLength> request_;
		// How many bytes have we received so far?
		size_t receiveOffset_;
		// How many bytes are we expecting?
		size_t receiveLength_;
		// Length of HTTP header in bytes (including trailing newlines)
		size_t headerLength_;
		// Are we receiving the header or the body?
		bool receivingHeader_;
		// Are we waiting for an internal reply to a HTTP request?
		bool requestPending_;

		// URL the client requested
		std::string requestUri_;
		// Session ID of the client
		std::string requestSession_;

		// Temporary buffer for holding response
		std::string response_;
		// How many bytes have we sent so far?
		size_t sendOffset_;

		uint32_t connectionId_;
		std::mutex lock_;

		size_t headerEndOffset();

		void beginReceivingRequest();
		void receive();
		void onReceived(const boost::system::error_code & errcode, size_t receivedLength);
		void processHeader(std::string const & header);
		void processBody(std::string const & body);
		
		void sendErrorReply();
		void sendResponse(size_t sendOffset);
		void onResponseSent(const boost::system::error_code & errcode, size_t sentLength);
};

