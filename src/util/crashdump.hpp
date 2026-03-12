#pragma once

#ifdef _WIN32

#include <Windows.h>

class CrashHandler
{
	public:
		static void initialize(const std::string & dumpFilename);

	private:
		inline CrashHandler() {}

		static LONG WINAPI onCrash(_EXCEPTION_POINTERS * exceptionPtrs);
		static std::string dumpFilename_;
};

#else

class CrashHandler
{
	public:
		static void initialize(const std::string & appName);

	private:
		inline CrashHandler() {}

		static void onSignal(int sig);
		static std::string appName_;
};

#endif
