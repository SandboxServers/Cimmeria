#include <stdafx.hpp>
#include <boost/python/exec.hpp>
#include <boost/python/import.hpp>
#include <common/console.hpp>
#include <entity/pyutil.hpp>
#include <iostream>
#include <thread>
#ifdef _WIN32
#include <Windows.h>
#endif

void ConsoleThread()
{
	std::this_thread::sleep_for(std::chrono::milliseconds(1500));
	while (true)
	{
		std::string cmd;
		std::getline(std::cin, cmd);

		LogMessage::Level orig_level = Logger::instance().logLevel();
		Logger::instance().setLogLevel(LogMessage::LL_WARNING);
		while (true)
		{
#ifdef _WIN32
			SetConsoleTextAttribute(GetStdHandle(STD_OUTPUT_HANDLE), FOREGROUND_BLUE | FOREGROUND_GREEN | FOREGROUND_RED | FOREGROUND_INTENSITY);
#else
			std::cout << "\033[37m";
#endif
			std::cout << ">>> ";
			std::getline(std::cin, cmd);
#ifdef _WIN32
			SetConsoleTextAttribute(GetStdHandle(STD_OUTPUT_HANDLE), FOREGROUND_BLUE | FOREGROUND_GREEN | FOREGROUND_RED);
#else
			std::cout << "\033[39m";
#endif

			if (cmd == "exit" || cmd == "quit")
				break;
			
			PyGilGuard guard;
			try
			{
				bp::object main = bp::import("__main__");
				bp::object global(main.attr("__dict__"));
				Logger::instance().setLogLevel(orig_level);
				bp::exec(cmd.c_str(), global, global);
				Logger::instance().setLogLevel(LogMessage::LL_WARNING);
			}
			catch (bp::error_already_set &)
			{
#ifdef _WIN32
				SetConsoleTextAttribute(GetStdHandle(STD_OUTPUT_HANDLE), FOREGROUND_GREEN | FOREGROUND_RED | FOREGROUND_INTENSITY);
#else
				std::cout << "\033[33m";
#endif
				PyUtil_ShowErr("Console exception");
			}
			catch (std::exception & e)
			{
				WARN("%s", e.what());
			}
		}
		Logger::instance().setLogLevel(orig_level);
	}
}

