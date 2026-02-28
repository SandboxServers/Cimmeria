#pragma once
#include <common/service.hpp>
#include <mercury/tcp_server.hpp>
#include <mercury/nub.hpp>
#include <entity/py_client.hpp>
#include <authentication/shard_client.hpp>
#include <baseapp/mercury/sgw/resource.hpp>
#include <baseapp/minigame_connection.hpp>
#include <baseapp/mercury/cell_handler.hpp>

class BaseClientMessageWriter : public MessageWriter
{
public:
	virtual void write(Service::EndpointType endpoint, DistributionPolicy policy, uint32_t entityId, uint8_t messageId, bp::object args);
	virtual void write(Service::EndpointType endpoint, DistributionPolicy policy, uint32_t entityId, uint8_t messageId, void * args, size_t argsSize);
};

class BaseCellMessageWriter : public MessageWriter
{
public:
	virtual void write(Service::EndpointType endpoint, DistributionPolicy policy, uint32_t entityId, uint8_t messageId, bp::object args);
	virtual void write(Service::EndpointType endpoint, DistributionPolicy policy, uint32_t entityId, uint8_t messageId, void * args, size_t argsSize);
};

class BaseService : public Service
{
	public:
		BaseService();

		inline static BaseService & base_instance()
		{
			return reinterpret_cast<BaseService &>(instance());
		}

		inline Mercury::SGW::ResourceManager & resources()
		{
			return resources_;
		}

		inline MinigameRequestManager & minigameManager()
		{
			return minigameManager_;
		}

		inline Mercury::TcpServer<MinigameConnection> * minigameServer() const
		{
			return minigameServer_;
		}
		
		inline Mercury::Nub * clientNub() const
		{
			return clientNub_;
		}

		virtual MessageWriter * messageWriter(EndpointType endpoint, uint32_t entityId);

	protected:
		virtual void initialize();
		virtual void cleanup();
		virtual std::string internalServiceName();
		
		Mercury::TcpServer<py_client> * consoleServer_;
		authentication::ShardClient::Ptr authClient_;
		Mercury::SGW::ResourceManager resources_;
		Mercury::TcpServer<Mercury::CellAppConnection> * cellServer_;
		Mercury::TcpServer<MinigameConnection> * minigameServer_;
		Mercury::Nub * clientNub_;
		Mercury::ClientlessPacketHandler * packetHandler_;
		MinigameRequestManager minigameManager_;
		BaseClientMessageWriter clientWriter_;
		BaseCellMessageWriter cellWriter_;
};

