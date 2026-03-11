#pragma once

#include <baseapp/minigame.hpp>
#include <mercury/tcp_server.hpp>
#include <array>

class MinigameConnection : public std::enable_shared_from_this<MinigameConnection>
{
	public:
		typedef std::shared_ptr<MinigameConnection> Ptr;

		static MinigameConnection * Create(Mercury::TcpServer<MinigameConnection> & server, uint32_t connectionId);

		MinigameConnection(uint32_t connectionId);
		virtual ~MinigameConnection();

		boost::asio::ip::tcp::socket & socket();
		uint32_t connectionId() const;
		Minigame::Ptr minigame() const;
		
		void connected();
		void queueMessage(std::string const & message);
		void close();
		bool isConnected() const;
		
		void queueExtensionMessage(std::string const & message);
		void closeMinigame();

	protected:
		void close(const boost::system::error_code & errcode);

		void disconnected();
		void minigameClosed();
		bool handleMessage(std::string const & message);
		bool handleVersionCheck(tinyxml2::XMLElement & body);
		bool handleLogin(tinyxml2::XMLElement & body);
		bool handleExtensionRequest(tinyxml2::XMLElement & body);

		void sendMessage();
		void messageSent(const boost::system::error_code & errcode, size_t sentLength);
		void receiveMessage();
		void receivedMessage(const boost::system::error_code & errcode, size_t receivedLength);

	private:
		static const uint32_t ApiVersion = 154;

		inline Ptr shared_this()
		{
			return shared_from_this();
		}

		// Connection handle
		boost::asio::ip::tcp::socket socket_;
		bool registered_;
		bool disconnected_;
		uint32_t connectionId_;

		// Message being sent
		std::string sendMessage_;
		// Amount of bytes already sent for message sendMessage_
		size_t sendOffset_;

		// Max length of minigame message (including the null terminator)
		static const size_t MaxMessageLength = 0x1000;
		// Buffer holding the current message and subsequent messages
		std::array<uint8_t, MaxMessageLength> message_;
		// Amount of readable bytes in message_
		size_t messageReceived_;

		// Mutex guarding the send queue
		std::mutex lock_;
		// Messages that need to be sent
		std::queue<std::string> sendQueue_;

		// ID of player currently playing the minigame
		std::string userId_;
		// entityId of player currently playing the minigame
		uint32_t entityId_;
		// API version of client
		uint32_t apiVersion_;

		Minigame::Ptr minigame_;
};

