#include <stdafx.hpp>

#include <boost/lexical_cast.hpp>
#include <boost/python/class.hpp>
#include <boost/python/def.hpp>
#include <boost/python/import.hpp>
#include <boost/python/scope.hpp>

#include <baseapp/base_service.hpp>
#include <common/console.hpp>
#include <mercury/base_cell_messages.hpp>
#include <baseapp/mercury/sgw/connect_handler.hpp>
#include <baseapp/mercury/sgw/client_handler.hpp>
#include <baseapp/mercury/cell_handler.hpp>
#include <entity/defs.hpp>
#include <baseapp/entity/base_entity.hpp>
#include <boost/asio/steady_timer.hpp>
#include <baseapp/mercury/sgw/resource.hpp>
#include <baseapp/entity/base_py_util.hpp>
#include <baseapp/entity/ticker.hpp>
#include <baseapp/cell_manager.hpp>
#include <stdlib.h>

bp::list dbQuery(std::string const & q)
{
	bp::list l;
	BaseService::base_instance().dbMgr().synchronousQuery(q, l);
	return l;
}

void dbPerform(std::string const & q)
{
	BaseService::base_instance().dbMgr().synchronousPerform(q);
}


void BaseClientMessageWriter::write(Service::EndpointType endpoint, DistributionPolicy /*policy*/, uint32_t entityId, uint8_t messageId, bp::object args)
{
	SGW_ASSERT(endpoint == Service::ClientMailbox);

	BaseEntity::Ptr entity = BaseEntityManager<BaseEntity>::instance().get(entityId);
	if (!entity)
	{
		WARNC(CATEGORY_MESSAGES, "Client entity %d does not exist!", entityId);
		return;
	}

	if (!entity->controller())
	{
		WARNC(CATEGORY_MESSAGES, "Client entity %d has no controller attached", entityId);
		return;
	}

	entity->controller()->forwardMessage(entityId, messageId, args);
}


void BaseClientMessageWriter::write(Service::EndpointType endpoint, DistributionPolicy policy, uint32_t entityId, uint8_t messageId, void * args, size_t argsSize)
{
	SGW_ASSERT(endpoint == Service::ClientMailbox);

	if (policy.flags & Mercury::DISTRIBUTE_CLIENT)
	{
		BaseEntity::Ptr entity = BaseEntityManager<BaseEntity>::instance().get(entityId);
		if (!entity)
		{
			WARNC(CATEGORY_MESSAGES, "Client entity %d does not exist!", entityId);
			return;
		}

		if (entity->controller())
		{
			entity->controller()->forwardMessage(entityId, messageId, args, argsSize);
		}
		else
		{
			WARNC(CATEGORY_MESSAGES, "Client entity %d has no controller attached", entityId);
			return;
		}
	}

	// TODO: Distribute messages based on LOD levels if flag is DISTRIBUTE_LOD
	if (policy.flags & (Mercury::DISTRIBUTE_LOD | Mercury::DISTRIBUTE_SPACE))
	{
		std::vector<BaseEntity::Ptr> entities;
		BaseEntityManager<BaseEntity>::instance().getEntities(policy.spaceId, entities);
		for (auto it = entities.begin(); it != entities.end(); ++it)
		{
			if ((*it)->entityId() != entityId && (*it)->controller())
				(*it)->controller()->forwardMessage(entityId, messageId, args, argsSize);
		}
	}

	if (policy.flags & Mercury::DISTRIBUTE_RECIPIENT)
	{
		BaseEntity::Ptr recipient = BaseEntityManager<BaseEntity>::instance().get(policy.recipientId);
		if (recipient)
		{
			if (recipient->controller())
				recipient->controller()->forwardMessage(entityId, messageId, args, argsSize);
			else
				WARNC(CATEGORY_MESSAGES, "Recipient entity %d has no controller attached", policy.recipientId);
		}
		else
			WARNC(CATEGORY_MESSAGES, "Recipient entity %d does not exist!", policy.recipientId);
	}
}


void BaseCellMessageWriter::write(Service::EndpointType endpoint, DistributionPolicy /*policy*/, uint32_t entityId, uint8_t messageId, bp::object args)
{
	SGW_ASSERT(endpoint == Service::CellMailbox || endpoint == Service::CellExposedMailbox);

	BaseEntity::Ptr entity = BaseEntityManager<BaseEntity>::instance().get(entityId);
	if (!entity)
	{
		WARNC(CATEGORY_MESSAGES,  "Recipient entity %d does not exist!", entityId);
		return;
	}

	if (entity->spaceId() == 0xffffffff)
	{
		WARNC(CATEGORY_MESSAGES,  "Entity %d is not on a space!", entityId);
		return;
	}

	SpaceData * space = CellManager::instance().findSpace(entity->spaceId());
	if (endpoint == Service::CellMailbox)
		space->handler->sendInternalMessage(entityId, messageId, entity->classDef()->findMethod(Service::CellMailbox, messageId), args);
	else
		space->handler->sendExposedMessage(entityId, messageId, entity->classDef()->findMethod(Service::CellExposedMailbox, messageId), args);
}


void BaseCellMessageWriter::write(Service::EndpointType endpoint, DistributionPolicy /*policy*/, uint32_t entityId, uint8_t messageId, void * args, size_t argsSize)
{
	SGW_ASSERT(endpoint == Service::CellMailbox || endpoint == Service::CellExposedMailbox);
	
	BaseEntity::Ptr entity = BaseEntityManager<BaseEntity>::instance().get(entityId);
	if (!entity)
	{
		WARNC(CATEGORY_MESSAGES,  "Recipient entity %d does not exist!", entityId);
		return;
	}

	if (entity->spaceId() == 0xffffffff)
	{
		WARNC(CATEGORY_MESSAGES,  "Entity %d is not on a space!", entityId);
		return;
	}

	SpaceData * space = CellManager::instance().findSpace(entity->spaceId());
	if (endpoint == Service::CellMailbox)
		space->handler->sendInternalMessage(entityId, messageId, args, argsSize);
	else
		space->handler->sendExposedMessage(entityId, messageId, args, argsSize);
}


BaseService::BaseService()
	: consoleServer_(nullptr), minigameServer_(nullptr), clientNub_(nullptr),
	packetHandler_(nullptr)
{
	configVersion_ = "20131124";
}

void BaseService::initialize()
{
	serviceName_ = "BaseService";
	
	INFO("Welcome to Atrea.BaseApp");
	#ifdef DEBUG
	INFO("Built on: " __DATE__ " " __TIME__ " (DEBUG BUILD)");
	#else
	INFO("Built on: " __DATE__ " " __TIME__ " (RELEASE)");
	#endif
	INFO("Initializing Mercury");
	Mercury::BaseChannel::initialize();
	BaseEntityManager<BaseEntity>::initialize();
	Mercury::SGW::ClientHandler::initialize();
	PyRegisterModule(false);
	authentication::ShardLogonQueue::init(new authentication::ShardLogonQueue());
	CellManager::initialize();
	CachedEntity::staticInit();
	CachedEntityManager::init(new CachedEntityManager());

	INFO("Initializing Python Env");
	exposeConfiguration();
	try
	{
		PyGilGuard guard;
		bp::object mainmod = bp::import("__main__");
		bp::dict main_ns = bp::extract<bp::dict>(mainmod.attr("__dict__"));

		bp::object system_module = bp::import("sys");
		bp::dict system_dict = bp::extract<bp::dict>(system_module.attr("__dict__"));
		bp::list path = bp::extract<bp::list>(system_dict["path"]);
		
		path.append("../python");

		{
			bp::object kernelmod = bp::import("Atrea");
			bp::scope active_scope(kernelmod);

			bp::def("dbQuery", &dbQuery);
			bp::def("dbPerform", &dbPerform);
			bp::def("sendResource", &PyUtil_SendResource);

			bp::def("createBaseEntity", &PyUtil_CreateBaseEntity);
			bp::def("createBaseEntityFromDb", &PyUtil_CreateBaseEntityFromDb);
			bp::def("createCellEntity", &PyUtil_CreateCellEntity);
			bp::def("findAllEntities", &PyUtil_FindAllEntities);
			bp::def("switchEntity", &PyUtil_SwitchEntity);

			bp::def("registerMinigameSession", &PyUtil_RegisterMinigameSession);
			bp::def("cancelMinigameSession", &PyUtil_CancelMinigameSession);

			bp::def("getGameTime", &PyUtil_GetGameTime);
			bp::def("addTimer", &PyUtil_AddTimer);
			bp::def("cancelTimer", &PyUtil_CancelTimer);

			bp::def("reloadClasses", &PyUtil_ReloadClasses);
			BaseEntity::registerClass();
			PythonMinigame::registerClass();
		}
		
		bp::object base = bp::import("base");
		PyTypeDb::instance().setupClasses<BaseEntity>("base");

		bp::object started = base.attr("baseStarted");
		if (started)
			started();
	}
	catch (bp::error_already_set &)
	{
		PyGilGuard guard;
		PyUtil_ShowErr("An exception occurred while initializing Python extensions");
		throw;
	}

	authClient_ = authentication::ShardClient::Ptr(new authentication::ShardClient());
	authClient_->startup();
	clientNub_ = new Mercury::Nub(ioService(), boost::lexical_cast<uint16_t>(Service::instance().getConfigParameter("shard_port")), 
		new Mercury::SGW::ConnectHandler());
	clientNub_->setTickInterval(boost::lexical_cast<unsigned int>(Service::instance().getConfigParameter("nub_tickrate")));
	if (!Service::instance().getConfigParameter("py_console_password").empty())
	{
		consoleServer_ = new Mercury::TcpServer<py_client>(
			boost::lexical_cast<unsigned int>(Service::instance().getConfigParameter("console_port")), 
			&py_client::create);
	}
	minigameServer_ = new Mercury::TcpServer<MinigameConnection>(
		boost::lexical_cast<unsigned int>(Service::instance().getConfigParameter("minigame_port")), 
		&MinigameConnection::Create);
	
	cellServer_ = new Mercury::TcpServer<Mercury::CellAppConnection>(
		boost::lexical_cast<unsigned int>(Service::instance().getConfigParameter("service_port")), 
		&Mercury::CellAppConnection::Create);


	Ticker::initialize(clientNub_);
	Ticker::instance().initTicks(CellManager::instance().ticks(), CellManager::instance().tickRate());

	servicePort_ = internalPort();

	new std::thread(&ConsoleThread);
}

void BaseService::cleanup()
{
	delete clientNub_;
	authClient_.reset();
	delete minigameServer_;
	delete consoleServer_;
}

std::string BaseService::internalServiceName()
{
	return "BaseService";
}

MessageWriter * BaseService::messageWriter(EndpointType endpoint, uint32_t /*entityId*/)
{
	if (endpoint == ClientMailbox)
		return &clientWriter_;
	else if (endpoint == CellMailbox || endpoint == CellExposedMailbox)
		return &cellWriter_;
	else
		throw std::runtime_error("Cannot send messages to this endpoint");
}
