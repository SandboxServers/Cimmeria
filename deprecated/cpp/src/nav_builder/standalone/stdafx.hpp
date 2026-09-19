#pragma once

// Standalone replacement for deprecated/cpp/src/stdafx.hpp.
//
// The real precompiled header drags in Boost (python, asio, thread), SOCI,
// TinyXML and the unified_kernel logger - none of which NavBuilder uses, and
// none of which setup.ps1 provisions any more. tools/build-navbuilder.ps1 puts
// this directory first on the include path, so `#include "stdafx.hpp"` in the
// nav_builder sources resolves here and the tool builds from nothing but the
// MSVC toolchain and external/recast.
//
// The legacy NavBuilder.vcxproj build is unaffected: it never sees this file.

#include <stdexcept>
#include <string>
#include <vector>
#include <iostream>
#include <fstream>
#include <cstdarg>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <cmath>
#include <ctime>
#include <stdint.h>

// Minimal synchronous stand-in for log/logger.hpp. Same line format as the
// original Logger::outputMessage ("[HH:MM:SS <category>] <message>" on
// stdout), minus the console colouring and the worker thread.
class Logger
{
public:
	static void initialize() {}
	static void shutdown() { fflush(stdout); }

	static void log(const char * level, const char * msg, ...)
	{
		char text[4096];
		va_list args;
		va_start(args, msg);
		vsnprintf(text, sizeof(text), msg, args);
		va_end(args);

		time_t now = time(nullptr);
		tm local;
		localtime_s(&local, &now);
		printf("[%02d:%02d:%02d %s] %s\n", local.tm_hour, local.tm_min, local.tm_sec, level, text);
		fflush(stdout);
	}
};

#define TRACE(msg, ...) Logger::log("TRACE   ", msg, ##__VA_ARGS__)
#define DEBUG1(msg, ...) Logger::log("DEBUG   ", msg, ##__VA_ARGS__)
#define DEBUG2(msg, ...) Logger::log("DEBUG   ", msg, ##__VA_ARGS__)
#define INFO(msg, ...) Logger::log("INFO    ", msg, ##__VA_ARGS__)
#define WARN(msg, ...) Logger::log("WARNING ", msg, ##__VA_ARGS__)
#define FAULT(msg, ...) Logger::log("ERROR   ", msg, ##__VA_ARGS__)
#define CRITICAL(msg, ...) Logger::log("CRITICAL", msg, ##__VA_ARGS__)

// Always-on, like the original (ENABLE_RELEASE_ASSERTS).
#define SGW_ASSERT(expr) { if (!(expr)) { \
	Logger::log("ASSERT  ", "%s(): ASSERTION FAILED: " #expr, __FUNCTION__); \
	fflush(stdout); exit(2); \
}}
