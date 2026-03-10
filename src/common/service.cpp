#include <stdafx.hpp>
#include <common/service.hpp>
#include <boost/lexical_cast.hpp>
#include <boost/python/def.hpp>
#include <boost/python/extract.hpp>
#include <boost/python/import.hpp>
#include <boost/python/object.hpp>
#include <boost/python/scope.hpp>
#include <entity/pyutil.hpp>
#include <soci/postgresql/soci-postgresql.h>

Service::Service()
	: work_(boost::asio::make_work_guard(service_)), timer_(service_)
{
	Logger::initialize();
	atexit(&Service::exitHandler);
	// FIXME: We shouldn't seed the RNG with the current time, use a proper randomness source
	rng_.seed(static_cast<unsigned int>(std::time(0)));
}

Service::~Service()
{
	service_.stop();
}

void Service::exitHandler()
{
	if (m_ptr)
	{
		instance().stop();
		init(nullptr);
	}
	Logger::shutdown();
}

void Service::start()
{
	INFO("Service initialization started");
	uint64_t load_start = microTime();
	loadConfiguration();

	const std::string & logLevel = getConfigParameter("log_level");
	if (logLevel == "TRACE")
		Logger::instance().setLogLevel(LogMessage::LL_TRACE);
	else if (logLevel == "DEBUG2")
		Logger::instance().setLogLevel(LogMessage::LL_DEBUG2);
	else if (logLevel == "DEBUG1")
		Logger::instance().setLogLevel(LogMessage::LL_DEBUG1);
	else if (logLevel == "INFO")
		Logger::instance().setLogLevel(LogMessage::LL_INFO);
	else if (logLevel == "WARNING")
		Logger::instance().setLogLevel(LogMessage::LL_WARNING);
	else if (logLevel == "ERROR")
		Logger::instance().setLogLevel(LogMessage::LL_ERROR);
	else if (logLevel == "CRITICAL")
		// Errors should always be displayed, so we won't go below ERROR loglevel!
		Logger::instance().setLogLevel(LogMessage::LL_ERROR);
	else
		throw std::runtime_error("Unknown log level specified in configuration!");

	serviceName_ = internalServiceName();
	dbMgr_.initialize();
	initialize();
	tick(boost::system::error_code());

	INFO("Service initialization completed (time: %d ms)", microTime() - load_start);
}

void Service::stop()
{
	cleanup();
}

soci::session & Service::db()
{
	return dbMgr_.session();
}

Database & Service::dbMgr()
{
	return dbMgr_;
}

uint64_t Service::microTime() const
{
#ifdef _WIN32
	return GetTickCount64();
#else
	struct timespec tp;
	clock_gettime(CLOCK_MONOTONIC, &tp);
	return tp.tv_sec * 1000 + tp.tv_nsec / 1000000;
#endif
}


/*
 * Registers a new timer. When the timer expires, the handler callback function is called.
 *
 * @param handler  Handler to call when the timer expires
 * @param time     Expiration time in milliseconds (current time is Service::microTime())
 * @param userData Optional user pointer passed to the handler function
 */
Service::TimerMgr::TimerId Service::addTimer(TimerMgr::TimerHandler handler, uint64_t time, void * userData)
{
	return timers_.addTimer(handler, time, userData);
}


/*
 * Cancels an active timer by its ID.
 * WARNING: Only use this method to cancel timers that are known to be active.
 *
 * @param timer Timer ID to cancel
 */
void Service::cancelTimer(TimerMgr::TimerId timer)
{
	timers_.cancelTimer(timer);
}


uint16_t Service::internalPort()
{
	return boost::lexical_cast<uint16_t>(Service::instance().getConfigParameter("service_port"));
}

const std::string & Service::getConfigParameter(const std::string & parameter) const
{
	auto iter = configOptions_.find(parameter);
	if (iter == configOptions_.end())
	{
		CRITICAL("Failed to retrieve configuration parameter '%s'", parameter.c_str());
		throw std::runtime_error("Failed to retrieve configuration parameter");
	}

	return iter->second;
}


/*
 * Exposes all configuration variables from the config .xml file via the Atrea.config module.
 */
void Service::exposeConfiguration()
{
	PyGilGuard guard;
	bp::object atrea(bp::import("Atrea"));
	bp::object vars(bp::handle<>(PyModule_New("config")));
	atrea.attr("__file__") = "<synthetic>";
	atrea.attr("config") = vars;
	bp::extract<bp::dict>(bp::getattr(bp::import("sys"), "modules"))()["Atrea.config"] = vars;

	for (auto const & var : configOptions_)
	{
		vars.attr(var.first.c_str()) = var.second;
	}
}


void Service::loadConfiguration()
{
	std::string config_path = std::string("../config/" + internalServiceName() +".config");
	tinyxml2::XMLDocument config_doc;
	if (config_doc.LoadFile(config_path.c_str()) != tinyxml2::XML_NO_ERROR)
	{
		CRITICAL("Couldn't load configuration file %s", config_path.c_str());
		throw std::runtime_error("Failed to load configuration.");
	}

	tinyxml2::XMLElement * root = config_doc.FirstChildElement("config");
	if (root == nullptr)
	{
		CRITICAL("Improper root element in configuration file %s", config_path.c_str());
		throw std::runtime_error("Failed to load configuration.");
	}

	for (tinyxml2::XMLElement * attribute = root->FirstChildElement(); attribute; attribute = attribute->NextSiblingElement())
	{
		const char * value = attribute->GetText();
		configOptions_.insert(
			std::make_pair(attribute->Value(), value ? value : ""));
	}

	auto versionIt = configOptions_.find("version");
	if (versionIt == configOptions_.end())
	{
		// Configuration file has no version info; assume that it is older than the first versioned config
		CRITICAL("Configuration file '%s' is outdated. Please upgrade it to the latest version.", config_path.c_str());
		throw std::runtime_error("Exiting due to a configuration load error");
	}

	if (versionIt->second != configVersion_)
	{
		// Configuration file is older/newer than the current one
		CRITICAL("Configuration file '%s' is outdated. Please upgrade it to the latest version.", config_path.c_str());
		CRITICAL("(File version: %s; Server version: %s)", versionIt->second.c_str(), configVersion_.c_str());
		throw std::runtime_error("Exiting due to a configuration load error");
	}
}

void Service::waitForTermination()
{
	service_.run();
}


/*
 * Called when the asio timer expired
 */
void Service::tick(const boost::system::error_code & error)
{
	// Make sure that we don't tick if tick() is called after
	// canceling the timer (with operation canceled error code)
	if (error == boost::system::errc::operation_canceled)
		return;

	SGW_ASSERT(error == boost::system::errc::success);

	timers_.tick(microTime());
	unsigned int next = TickInterval - ((microTime() - lastTick_) % TickInterval);
	if (next == 0)
		next = TickInterval;
	timer_.expires_after(std::chrono::milliseconds(next));
	timer_.async_wait([this](const boost::system::error_code & ec) { tick(ec); });
}


MessageWriter::~MessageWriter()
{
}
