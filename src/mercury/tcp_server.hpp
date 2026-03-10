#pragma once

#include <common/service.hpp>

namespace Mercury
{

template <typename _T>
class TcpServer
{
	public:
		typedef std::function<_T * (TcpServer<_T> & server, uint32_t connection_id)> ConnectionFactoryType;

		TcpServer(uint16_t port, ConnectionFactoryType factory)
			: service_(Service::instance().ioService()),
			  socket_(Service::instance().ioService(), boost::asio::ip::tcp::endpoint(boost::asio::ip::tcp::v4(), port)),
			  connectionFactory_(factory),
			  nextConnectionId_(1)
		{
			socket_.listen(30);
			acceptConnections();
		}

		void unregister_connection(typename _T::Ptr conn)
		{
			for (uint32_t i = 0; i < connections_.size(); i++)
			{
				if (connections_[i]->connectionId() == conn->connectionId())
				{
					connections_.erase(connections_.begin() + i);
					LOGF(LL_TRACE, "Closed connection");
					return;
				}
			}
	
			LOGF(LL_WARNING, "No connection with connection id %d was found", conn->connectionId());
		}

	private:
		boost::asio::ip::tcp::acceptor socket_;
		boost::asio::io_context & service_;
		ConnectionFactoryType connectionFactory_;
		std::vector<typename _T::Ptr> connections_;
		uint32_t nextConnectionId_;

		void acceptConnections()
		{
			typename _T::Ptr conn = typename _T::Ptr(connectionFactory_(*this, nextConnectionId_++));
			socket_.async_accept(conn->socket(),
				[this, conn](const boost::system::error_code & err) {
					acceptedConnection(err, conn);
				});
		}

		void acceptedConnection(const boost::system::error_code & errcode, typename _T::Ptr conn)
		{
			if (errcode == boost::system::errc::success)
			{
				LOGF(LL_TRACE, "Accepted connection");
				conn->connected();
				connections_.push_back(conn);
			}
			else
				LOGF(LL_ERROR, "Failed to accept connection: %s", errcode.message().c_str());

			acceptConnections();
		}
};

}

