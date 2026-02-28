#include <stdafx.hpp>

#include <boost/lexical_cast.hpp>
#include <boost/python/def.hpp>
#include <boost/python/extract.hpp>
#include <boost/python/import.hpp>
#include <boost/python/scope.hpp>

#include <cellapp/cell_service.hpp>
#include <common/console.hpp>
#include <cellapp/entity/cell_entity.hpp>
#include <cellapp/entity/entity_manager.hpp>
#include <cellapp/entity/cell_py_util.hpp>
#include <cellapp/base_client.hpp>
#include <entity/defs.hpp>
#include <mercury/base_cell_messages.hpp>

bp::list dbQuery(std::string const & q)
{
	bp::list l;
	CellService::cell_instance().dbMgr().synchronousQuery(q, l);
	return l;
}

void dbPerform(std::string const & q)
{
	CellService::cell_instance().dbMgr().synchronousPerform(q);
}

void CellMessageWriter::write(Service::EndpointType endpoint, DistributionPolicy policy, uint32_t entityId, uint8_t messageId, bp::object args)
{
	PyMethod * method = CellEntityManager<CellEntity>::instance().get(entityId)->classDef()->findMethod(endpoint, messageId);
	SGW_ASSERT(method);
	if (endpoint == Service::BaseMailbox)
	{
		// TODO: Policy ignored - is it OK?
		CellService::cell_instance().baseClient()->sendBaseAppMessage(entityId, messageId, *method, args);
	}
	else if (endpoint == Service::ClientMailbox)
	{
		CellService::cell_instance().baseClient()->sendClientMessage(entityId, messageId, policy, *method, args);
	}
	else
		throw std::runtime_error("Tried to send message to unsupported endpoint type");
}


void CellMessageWriter::write(Service::EndpointType endpoint, DistributionPolicy policy, uint32_t entityId, uint8_t messageId, void * args, size_t argsSize)
{
	throw std::runtime_error("Unsupported write type!");
}


CellService::CellService()
	: client_(nullptr), cellId_(0)
{
	configVersion_ = "20131124";
}

CellService::~CellService()
{
}

uint32_t CellService::cellId() const
{
	return cellId_;
}

BaseAppClient * CellService::baseClient() const
{
	return client_;
}

void CellService::initialize()
{
	serviceName_ = "CellService";
	
	INFO("Welcome to Atrea.CellApp");
	#ifdef DEBUG
	INFO("Built on: " __DATE__ " " __TIME__ " (DEBUG BUILD)");
	#else
	INFO("Built on: " __DATE__ " " __TIME__ " (RELEASE)");
	#endif
	INFO("Initializing Mercury");
	cellId_ = boost::lexical_cast<uint16_t>(getConfigParameter("cell_id"));
	CellEntityManager<CellEntity>::initialize(cellId_);
	srand((unsigned int)time(nullptr));
	
	if (!Service::instance().getConfigParameter("py_console_password").empty())
	{
		consoleServer_ = new Mercury::TcpServer<py_client>(
			boost::lexical_cast<unsigned int>(Service::instance().getConfigParameter("console_port")), 
			&py_client::create);
	}

	boost::thread * t = new boost::thread(&ConsoleThread);
	
	INFO("Initializing Python Env");
	PyRegisterModule(true);
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
			bp::def("createCellPlayer", &PyUtil_CreateCellPlayer);
			bp::def("createCellEntity", &PyUtil_CreateCellEntity);
			bp::def("destroyCellEntity", &PyUtil_DestroyCellEntity);
			bp::def("createSpace", &PyUtil_CreateSpace);
			bp::def("destroySpace", &PyUtil_DestroySpace);
			bp::def("findSpace", &PyUtil_FindSpace);
			bp::def("findEntity", &PyUtil_FindEntity);
			bp::def("findEntityOnSpace", &PyUtil_FindEntityOnSpace);
			bp::def("findEntitiesByDbid", &PyUtil_FindEntitiesByDbid);
			bp::def("findEntities", &PyUtil_FindEntities);
			bp::def("getGameTime", &PyUtil_GetGameTime);
			bp::def("addTimer", &PyUtil_AddTimer);
			bp::def("cancelTimer", &PyUtil_CancelTimer);
			bp::def("findPath", &PyUtil_FindPath);
			bp::def("findDetailedPath", &PyUtil_FindDetailedPath);
			bp::def("reloadClasses", &PyUtil_ReloadClasses);
			CellEntity::registerClass();
		}

		bp::object module = bp::import("Atrea");
		bp::object cell = bp::import("cell");
		loadCellClasses();

		bp::object started = cell.attr("cellStarting");
		if (started)
			started();
	}
	catch (bp::error_already_set &)
	{
		CRITICAL("An exception occurred while initializing Python extensions:");
		PyGilGuard guard;
		PyUtil_ShowErr();
		throw;
	}

	// TODO: This is a hack!
	client_ = new BaseAppClient();
	BaseAppClient::Ptr clientRef(client_);

	INFO("Initializing Spaces");
	SpaceManager::initialize();
	SpaceManager::instance().setEventHandler(boost::bind(&CellService::onSpaceEvent, this, _1, _2));
	SpaceManager::instance().loadSpaces("../entities/cell_spaces.xml", "../entities/spaces.xml");
	
	try
	{
		PyGilGuard guard;
		bp::object cell = bp::import("cell");
		bp::object started = cell.attr("cellStarted");
		if (started)
			started();
	}
	catch (bp::error_already_set &)
	{
		CRITICAL("An exception occurred while initializing Python extensions:");
		PyGilGuard guard;
		PyUtil_ShowErr();
		throw;
	}
	
	client_->connect(getConfigParameter("baseapp_address"), 
		boost::lexical_cast<uint16_t>(getConfigParameter("baseapp_port")));
}

void CellService::cleanup()
{
}

std::string CellService::internalServiceName()
{
	return "CellService";
}

void CellService::loadCellClasses()
{
	bp::object module = bp::import("Atrea");
	PyTypeDb::instance().setupClasses<CellEntity>("cell");

	auto & classes = PyTypeDb::instance().classes();
	for (auto it = classes.begin(); it != classes.end(); ++it)
	{
		if (!it->second->isInterface() && it->second->index() != -1)
		{
			CellMailboxClass * mbox = new CellMailboxClass(module, it->second);
			it->second->setClientMailbox(mbox);
		}
	}
}

MessageWriter * CellService::messageWriter(EndpointType endpoint, uint32_t entityId)
{
	if (endpoint == BaseMailbox || endpoint == ClientMailbox)
		return &writer_;
	else
		throw std::runtime_error("Cannot send messages to this endpoint");
}

void CellService::onSpaceEvent(Space * space, SpaceManager::Event event)
{
	TRACE("onSpaceEvent: space %d, event %d",
		space->spaceId(), event);

	if (client_->isReady())
	{
		if (event == SpaceManager::Created)
			client_->sendSpaceData(space->spaceId(), cellId(), space->worldName());
		else if (event == SpaceManager::Deleted)
			client_->sendSpaceData(space->spaceId(), 0xffffffff, space->worldName());
		else
			WARN("Unhandled space event: %d", event);
	}
}


