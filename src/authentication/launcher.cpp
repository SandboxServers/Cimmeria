#include <stdafx.hpp>
#include <common/service.hpp>
#include <authentication/service_main.hpp>

int main(int argc, char* argv[])
{
	try
	{
		Service::init(new AuthenticationService());
		Service::instance().start();
		Service::instance().waitForTermination();
	}
	catch (std::exception & e)
	{
		CRITICAL("An exception occurred during initialization: %s", e.what());
	}

	return 0;
}

