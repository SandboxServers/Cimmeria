#include <stdafx.hpp>
#include <util/crashdump.hpp>

#ifdef _WIN32

#include <dbghelp.h>
#include <time.h>

std::string CrashHandler::dumpFilename_;

void CrashHandler::initialize(const std::string & dumpFilename)
{
	dumpFilename_ = dumpFilename;
	// Only set an exception filter when no debugger is attached to make debugging easier
	if (IsDebuggerPresent() == FALSE)
	{
		::SetUnhandledExceptionFilter(&CrashHandler::onCrash);
	}
}

LONG WINAPI CrashHandler::onCrash(_EXCEPTION_POINTERS * exceptionPtrs)
{
	CRITICAL(" *** SERVER CRASHED *** ");
	CRITICAL("Exception 0x%08x at IP %p", exceptionPtrs->ExceptionRecord->ExceptionCode, exceptionPtrs->ExceptionRecord->ExceptionAddress);
	CRITICAL("Writing crash dump to file ...");

	LONG retval = EXCEPTION_CONTINUE_SEARCH;

	char date[64];
	time_t now = time(nullptr);
	tm * ltime = localtime(&now);
	sprintf(date, "%04d-%02d-%02d %02d_%02d", ltime->tm_year + 1900, ltime->tm_mon + 1, ltime->tm_mday, ltime->tm_hour, ltime->tm_min, ltime->tm_sec);

	std::string dumpPath = dumpFilename_ +"_" +date +".dmp";

	HANDLE hFile = ::CreateFileA(dumpPath.c_str(), GENERIC_WRITE, FILE_SHARE_WRITE, NULL, CREATE_ALWAYS,
								FILE_ATTRIBUTE_NORMAL, NULL);

	if (hFile != INVALID_HANDLE_VALUE)
	{
		_MINIDUMP_EXCEPTION_INFORMATION ExInfo;

		ExInfo.ThreadId = ::GetCurrentThreadId();
		ExInfo.ExceptionPointers = exceptionPtrs;
		ExInfo.ClientPointers = NULL;

		BOOL bOK = ::MiniDumpWriteDump(GetCurrentProcess(), GetCurrentProcessId(), hFile, MiniDumpWithFullMemory, &ExInfo, NULL, NULL);
		if (bOK)
		{
			CRITICAL("Crash dump written to '%s'", dumpPath.c_str());
			retval = EXCEPTION_EXECUTE_HANDLER;
		}
		else
		{
			CRITICAL("Failed to write crash dump to '%s' (error code: %d)", dumpPath.c_str(), GetLastError());
		}
		::CloseHandle(hFile);
	}
	else
	{
		CRITICAL("Failed to create crash dump file '%s' (error code: %d)", dumpPath.c_str(), GetLastError());
	}

	return retval;
}

#else // !_WIN32

#include <csignal>
#include <cstdio>
#include <cstdlib>
#include <execinfo.h>
#include <unistd.h>

std::string CrashHandler::appName_;

void CrashHandler::initialize(const std::string & appName)
{
	appName_ = appName;

	struct sigaction sa;
	sa.sa_handler = &CrashHandler::onSignal;
	sa.sa_flags = SA_RESETHAND; // Reset to SIG_DFL after first invocation
	sigemptyset(&sa.sa_mask);

	sigaction(SIGSEGV, &sa, nullptr);
	sigaction(SIGBUS,  &sa, nullptr);
	sigaction(SIGABRT, &sa, nullptr);
	sigaction(SIGFPE,  &sa, nullptr);
}

void CrashHandler::onSignal(int sig)
{
	// These calls are not fully async-signal-safe, but are best-effort
	// for crash diagnostics. The handler will re-raise after printing.
	const char * sigName = "UNKNOWN";
	switch (sig)
	{
		case SIGSEGV: sigName = "SIGSEGV"; break;
		case SIGBUS:  sigName = "SIGBUS";  break;
		case SIGABRT: sigName = "SIGABRT"; break;
		case SIGFPE:  sigName = "SIGFPE";  break;
	}

	fprintf(stderr, "\n *** %s CRASHED (signal %s) ***\n", appName_.c_str(), sigName);

	void * frames[64];
	int count = backtrace(frames, 64);
	fprintf(stderr, "Backtrace (%d frames):\n", count);
	backtrace_symbols_fd(frames, count, STDERR_FILENO);
	fprintf(stderr, "\n");

	// Re-raise with default handler to produce a core dump
	raise(sig);
}

#endif
