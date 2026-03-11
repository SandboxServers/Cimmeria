#pragma once

#define TIXML_USE_STL

#ifdef _WIN32
#define _WIN32_WINNT 0x0600
#endif

#ifdef _WIN32
#ifndef BOOST_PYTHON_STATIC_LIB
#define BOOST_PYTHON_STATIC_LIB
#endif
#endif

#include <cstdint>
#include <deque>
#include <functional>
#include <map>
#include <memory>
#include <mutex>
#include <queue>
#include <set>
#include <stack>
#include <stdexcept>
#include <string>
#include <vector>

#include <fwd_decls.hpp>
#include <boost/python/object.hpp>
#include <boost/python/dict.hpp>
#include <boost/python/list.hpp>
#include <boost/asio/io_context.hpp>
#include <boost/asio/ip/tcp.hpp>
#include <boost/asio/ip/udp.hpp>
#include <chrono>
#include <xml/tinyxml2.h>
#include <log/logger.hpp>
#include <soci/soci.h>

namespace bp = boost::python;
