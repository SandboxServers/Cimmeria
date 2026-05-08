#include <stdafx.hpp>
#include <common/service.hpp>
#include <baseapp/base_service.hpp>
#include <boost/python/errors.hpp>
#include <util/crashdump.hpp>
#include <entity/pyutil.hpp>

int main(int argc, char* argv[])
{
	try
	{
		CrashHandler::initialize("BaseApp");
		Service::init(new BaseService());
		Service::instance().start();

		Service::instance().waitForTermination();
	}
	catch (bp::error_already_set &)
	{
		CRITICAL("A Python exception occurred during initialization");
		{
			PyGilGuard guard;
			PyUtil_ShowErr();
		}
		Service::instance().stop();
		Service::instance().waitForTermination();
		Service::init(nullptr);
	}
	catch (std::exception & e)
	{
		CRITICAL("An exception occurred during initialization: %s", e.what());
		Service::instance().stop();
		Service::instance().waitForTermination();
		Service::init(nullptr);
	}

	return 0;
}

