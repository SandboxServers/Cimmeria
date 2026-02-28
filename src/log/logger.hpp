#pragma once

#include <stdarg.h>
#include <boost/circular_buffer.hpp>

#define ENABLE_RELEASE_ASSERTS
#define MIN_LOG_LEVEL LogMessage::LL_TRACE

#define LOG(level, msg, ...) Logger::instance().log<LogMessage::level>("        ", msg, ##__VA_ARGS__)
#if defined(_MSC_VER)
#define LOGF(level, msg, ...) Logger::instance().log<LogMessage::level>("        ", __FUNCTION__ "(): " msg, ##__VA_ARGS__)
#else
#define LOGF(level, msg, ...) Logger::instance().log<LogMessage::level>("        ", "%s(): " msg, __FUNCTION__, ##__VA_ARGS__)
#endif

#define LOGC(level, category, msg, ...) Logger::instance().log<LogMessage::level>(category, msg, ##__VA_ARGS__)
#if defined(_MSC_VER)
#define LOGCF(level, category, msg, ...) Logger::instance().log<LogMessage::level>(category, __FUNCTION__ "(): " msg, ##__VA_ARGS__)
#else
#define LOGCF(level, category, msg, ...) Logger::instance().log<LogMessage::level>(category, "%s(): " msg, __FUNCTION__, ##__VA_ARGS__)
#endif

// Shortcuts for the longer LOG(LEVEL, msg, ...) invocation
#define TRACE(msg, ...) LOG(LL_TRACE, msg, ##__VA_ARGS__)
#define DEBUG1(msg, ...) LOG(LL_DEBUG1, msg, ##__VA_ARGS__)
#define DEBUG2(msg, ...) LOG(LL_DEBUG2, msg, ##__VA_ARGS__)
#define INFO(msg, ...) LOG(LL_INFO, msg, ##__VA_ARGS__)
#define WARN(msg, ...) LOG(LL_WARNING, msg, ##__VA_ARGS__)
#define FAULT(msg, ...) LOG(LL_ERROR, msg, ##__VA_ARGS__)
#define CRITICAL(msg, ...) LOG(LL_CRITICAL, msg, ##__VA_ARGS__)

// Shortcuts for the longer LOGC(LEVEL, CATEGORY, msg, ...) invocation
#define TRACEC(category, msg, ...) LOGC(LL_TRACE, category, msg, ##__VA_ARGS__)
#define DEBUG1C(category, msg, ...) LOGC(LL_DEBUG1, category, msg, ##__VA_ARGS__)
#define DEBUG2C(category, msg, ...) LOGC(LL_DEBUG2, category, msg, ##__VA_ARGS__)
#define INFOC(category, msg, ...) LOGC(LL_INFO, category, msg, ##__VA_ARGS__)
#define WARNC(category, msg, ...) LOGC(LL_WARNING, category, msg, ##__VA_ARGS__)
#define FAULTC(category, msg, ...) LOGC(LL_ERROR, category, msg, ##__VA_ARGS__)
#define CRITICALC(category, msg, ...) LOGC(LL_CRITICAL, category, msg, ##__VA_ARGS__)

#if defined(DEBUG) || defined(ENABLE_RELEASE_ASSERTS)
#if defined(_MSC_VER)
#define SGW_ASSERT(expr) { if (!(expr)) { \
	Logger::instance().log<LogMessage::LL_CRITICAL>("ASSERT  ", __FUNCTION__ "(): ASSERTION FAILED: " #expr); \
	Logger::abort(); \
}}
#else
#define SGW_ASSERT(expr) { if (!(expr)) { \
	Logger::instance().log<LogMessage::LL_CRITICAL>("ASSERT  ", "%s(): ASSERTION FAILED: " #expr, __FUNCTION__); \
	Logger::abort(); \
}}
#endif
#else
#define SGW_ASSERT(expr)
#endif

struct LogMessage
{
	enum Level
	{
		// Trace messages (for development only!)
		LL_TRACE = 0,
		// More verbose debugging messages (not shown in release builds)
		LL_DEBUG2 = 1,
		// Debugging messages (not shown in release builds)
		LL_DEBUG1 = 2,
		// Informative messages
		LL_INFO = 3,
		// Events that don't happen in normal circumstances
		LL_WARNING = 4,
		// Recoverable error events (renamed to avoid collision with win32 macros)
		LL_ERROR = 5,
		// Events preventing the server node from working properly
		LL_CRITICAL = 6
	};

	Level level;
	std::string category;
	std::string message;
};

class LogMessageQueue
{
public:
	LogMessageQueue(unsigned int size, bool allowDroppingMessages);

	void push(LogMessage const & entry);
	void pop(LogMessage & entry);
	bool empty();

private:
	LogMessageQueue(LogMessageQueue const &);

	boost::mutex mutex_;
	boost::condition_variable emptyCond_;
	boost::condition_variable fullCond_;
	boost::circular_buffer<LogMessage> buffer_;
	unsigned int droppedMessages_;
	bool allowDroppingMessages_;
};

class Logger
{
public:
	static void initialize();
	static void shutdown();
	static void abort();
	static Logger & instance();

	~Logger();

	template <LogMessage::Level _L>
	void log(const char * category, const char * msg, ...)
	{
		if (_L >= MIN_LOG_LEVEL)
		{
			va_list args;
			va_start(args, msg);
			log(_L, category, msg, args);
			va_end(args);
		}
	}

	void log(LogMessage::Level level, const char * category, const char * msg, va_list args);

	void setLogLevel(LogMessage::Level level);
	LogMessage::Level logLevel() const;

private:
	Logger(unsigned int queueSize, bool allowDroppingMessages);
	void outputMessage(LogMessage::Level level, std::string const & category, std::string const & msg);
	void processMessages();

	LogMessageQueue queue_;
	LogMessage::Level level_;
	boost::thread thread_;

	static Logger * instance_;
};

// TODO: This should be moved to somewhere else!
// Logging category name for entity events
extern const char * CATEGORY_ENTITY;
// Network events category
extern const char * CATEGORY_MERCURY;
// Network messages category
extern const char * CATEGORY_MESSAGES;
// Space events category
extern const char * CATEGORY_SPACE;
