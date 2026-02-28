#include <stdafx.hpp>
#include <iostream>
#include <cstdio>
#ifdef _WIN32
#include <windows.h>
#endif

// Logging category name for entity events
const char * CATEGORY_ENTITY = "ENTITY  ";
// Network events category
const char * CATEGORY_MERCURY = "MERCURY ";
// Network messages category
const char * CATEGORY_MESSAGES = "MESSAGES";
// Space events category
const char * CATEGORY_SPACE = "SPACE   ";


#define CRASHDUMP_ON_ASSERT

Logger * Logger::instance_ = nullptr;

LogMessageQueue::LogMessageQueue(unsigned int size, bool allowDroppingMessages)
	: buffer_(size), droppedMessages_(0), allowDroppingMessages_(allowDroppingMessages)
{
}

void LogMessageQueue::push(LogMessage const & entry)
{
	boost::mutex::scoped_lock lock(mutex_);
	if (buffer_.full())
	{
		if (allowDroppingMessages_)
		{
			droppedMessages_++;
			return;
		}
		else
		{
			while (buffer_.full())
			{
				fullCond_.wait(lock);
			}
		}
	}

	SGW_ASSERT(!buffer_.full());
	bool empty = buffer_.empty();
	buffer_.push_back(entry);
	lock.unlock();
	if (empty)
		emptyCond_.notify_one();
}

void LogMessageQueue::pop(LogMessage & entry)
{
	boost::mutex::scoped_lock lock(mutex_);
	while (buffer_.empty())
	{
		emptyCond_.wait(lock);
	}
	
	bool full = buffer_.full();
	entry = buffer_.front();
	buffer_.pop_front();
	lock.unlock();
	if (full && !allowDroppingMessages_)
		fullCond_.notify_one();
}

bool LogMessageQueue::empty()
{
	boost::mutex::scoped_lock lock(mutex_);
	return buffer_.empty();
}


void Logger::initialize()
{
	SGW_ASSERT(instance_ == nullptr);
	instance_ = new Logger(500, false);
}

Logger & Logger::instance()
{
	return *instance_;
}

void Logger::shutdown()
{
	SGW_ASSERT(instance_ != nullptr);
	delete instance_;
	instance_ = nullptr;
}

void Logger::abort()
{
	// Sleep a bit to make sure that the logger thread can log the exception
#ifdef _WIN32
	Sleep(1000);
#else
	sleep(1);
#endif
#ifdef CRASHDUMP_ON_ASSERT
	uint32_t * dummy = 0;
	*dummy = 0;
#else
	abort();
#endif
}

Logger::Logger(unsigned int queueSize, bool allowDroppingMessages)
	: queue_(queueSize, allowDroppingMessages), level_(LogMessage::LL_TRACE),
	thread_(boost::bind(&Logger::processMessages, this))
{
}

Logger::~Logger()
{
	LogMessage entry;
	entry.level = LogMessage::LL_TRACE;
	entry.category = "        ";
	entry.message = "Shutting down logger";
	queue_.push(entry);
	
	thread_.interrupt();
	thread_.join();
}

void Logger::log(LogMessage::Level level, const char * category, const char * msg, va_list args)
{
	if (level_ > level)
		return;

	char msg_buf[2048];
#ifdef _WIN32
	_vsnprintf(msg_buf, sizeof(msg_buf), msg, args);
#else
	vsnprintf(msg_buf, sizeof(msg_buf), msg, args);
#endif

	LogMessage entry;
	entry.level = level;
	entry.category = category;
	entry.message = msg_buf;
	queue_.push(entry);
}

LogMessage::Level Logger::logLevel() const
{
	return level_;
}

void Logger::setLogLevel(LogMessage::Level level)
{
	level_ = level;
}

void Logger::outputMessage(LogMessage::Level level, std::string const & category, std::string const & msg)
{
	time_t now = time(nullptr);
	tm * local = localtime(&now);
	char time_fmt[10];
	snprintf(time_fmt, sizeof(time_fmt), "%02d:%02d:%02d ", local->tm_hour, local->tm_min, local->tm_sec);

#ifdef _WIN32
	WORD text_attrs;
	switch (level)
	{
	case LogMessage::LL_TRACE: text_attrs = FOREGROUND_BLUE | FOREGROUND_GREEN | FOREGROUND_RED; break;
	case LogMessage::LL_DEBUG2: text_attrs = FOREGROUND_BLUE | FOREGROUND_GREEN | FOREGROUND_RED; break;
	case LogMessage::LL_DEBUG1: text_attrs = FOREGROUND_BLUE | FOREGROUND_GREEN | FOREGROUND_RED | FOREGROUND_INTENSITY; break;
	case LogMessage::LL_INFO: text_attrs = FOREGROUND_BLUE | FOREGROUND_GREEN | FOREGROUND_RED | FOREGROUND_INTENSITY; break;
	case LogMessage::LL_WARNING: text_attrs = FOREGROUND_RED | FOREGROUND_GREEN | FOREGROUND_INTENSITY; break;
	case LogMessage::LL_ERROR: text_attrs = FOREGROUND_RED | FOREGROUND_INTENSITY; break;
	case LogMessage::LL_CRITICAL: text_attrs = FOREGROUND_RED | FOREGROUND_INTENSITY; break;
	default: text_attrs = FOREGROUND_BLUE | FOREGROUND_GREEN | FOREGROUND_RED | FOREGROUND_INTENSITY;
	}

	SetConsoleTextAttribute(GetStdHandle(STD_OUTPUT_HANDLE), text_attrs);
	std::cout << "[" << time_fmt << category << "] " << msg << std::endl;
#else
	const char * color;
	switch (level)
	{
	case LogMessage::LL_TRACE: color = "\x1b[1;30m"; break;
	case LogMessage::LL_DEBUG2: color = "\x1b[1;30m"; break;
	case LogMessage::LL_DEBUG1: color = "\x1b[0;37m"; break;
	case LogMessage::LL_INFO: color = "\x1b[1;37m"; break;
	case LogMessage::LL_WARNING: color = "\x1b[0;33m"; break;
	case LogMessage::LL_ERROR: color = "\x1b[0;31m"; break;
	case LogMessage::LL_CRITICAL: color = "\x1b[0;31m"; break;
	default: color = "\x1b[0;36m";
	}

	std::cout << color << "[" << time_fmt << category << "] " << msg << std::endl;
#endif

	std::cout.flush();
}

void Logger::processMessages()
{
	try
	{
		LogMessage message;
		for (;;)
		{
			queue_.pop(message);
			outputMessage(message.level, message.category, message.message);
		}
	}
	catch (boost::thread_interrupted &)
	{
	}
}



