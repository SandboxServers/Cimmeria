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
		// TODO!
		inline static void initialize(const std::string & dumpFilename) {}
};

#endif
