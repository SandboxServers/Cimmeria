#pragma once

#define TIXML_USE_STL

#define _WIN32_WINNT 0x0600

#ifndef BOOST_PYTHON_STATIC_LIB
#define BOOST_PYTHON_STATIC_LIB
#endif

#include <stdexcept>
#include <string>
#include <vector>
#include <set>
#include <map>
#include <functional>
#include <deque>
#include <queue>
#include <stack>
#include <mutex>
#include <stdint.h>

#include <fwd_decls.hpp>
#include <boost/array.hpp>
#include <boost/enable_shared_from_this.hpp>
#include <boost/python/object.hpp>
#include <boost/python/dict.hpp>
#include <boost/python/list.hpp>
#include <boost/thread/thread.hpp>
#include <boost/thread/mutex.hpp>
#include <boost/thread/condition_variable.hpp>
#include <boost/function.hpp>
#include <boost/bind.hpp>
#include <boost/asio/io_service.hpp>
#include <boost/asio/ip/tcp.hpp>
#include <boost/asio/ip/udp.hpp>
#include <boost/date_time/posix_time/posix_time_types.hpp>
#include <xml/tinyxml2.h>
#include <log/logger.hpp>
#include <soci.h>

namespace bp = boost::python;

#if defined(__GNUC__) && ((__GNUC__ < 4) || \
	(__GNUC__ == 4 && __GNUC_MINOR__ < 6))
#define nullptr NULL
#endif

