#pragma once

#include <common/service.hpp>
#include <entity/py_client.hpp>
#include <cellapp/space.hpp>

class CellMessageWriter : public MessageWriter
{
public:
	virtual void write(Service::EndpointType endpoint, DistributionPolicy policy, uint32_t entityId, uint8_t messageId, bp::object args);
	virtual void write(Service::EndpointType endpoint, DistributionPolicy policy, uint32_t entityId, uint8_t messageId, void * args, size_t argsSize);
};

class CellService : public Service
{
	public:
		CellService();
		~CellService();

		inline static CellService & cell_instance()
		{
			return reinterpret_cast<CellService &>(instance());
		}

		void loadCellClasses();
		virtual MessageWriter * messageWriter(EndpointType endpoint, uint32_t entityId);

		uint32_t cellId() const;
		class BaseAppClient * baseClient() const;

		void onSpaceEvent(Space * space, SpaceManager::Event event);

	protected:
		virtual void initialize();
		virtual void cleanup();
		virtual std::string internalServiceName();
		
		Mercury::TcpServer<py_client> * consoleServer_;
		Mercury::Nub * clientNub_;
		class BaseAppClient * client_;
		CellMessageWriter writer_;
		uint32_t cellId_;
};

