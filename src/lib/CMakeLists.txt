# UnifiedKernel — shared static library for all Cimmeria servers.

add_library(unified_kernel STATIC
    # Common
    ../common/console.cpp
    ../common/database.cpp
    ../common/service.cpp

    # Entity system
    ../entity/defs.cpp
    ../entity/entity.cpp
    ../entity/mailbox.cpp
    ../entity/method.cpp
    ../entity/python.cpp
    ../entity/pyutil.cpp
    ../entity/py_client.cpp

    # Logging
    ../log/logger.cpp

    # Mercury networking
    ../mercury/bundle.cpp
    ../mercury/channel.cpp
    ../mercury/condemned_channels.cpp
    ../mercury/encryption_filter.cpp
    ../mercury/packet.cpp
    ../mercury/memory_stream.cpp
    ../mercury/message.cpp
    ../mercury/message_handler.cpp
    ../mercury/nub.cpp
    ../mercury/sgw/entity_message_handler.cpp
    ../mercury/stream.cpp
    ../mercury/transparent_filter.cpp
    ../mercury/unified_connection.cpp

    # Utilities
    ../util/crashdump.cpp

    # Vendored: TinyXML2
    ../xml/tinyxml2.cpp
)

target_include_directories(unified_kernel PUBLIC
    ${CMAKE_SOURCE_DIR}/src
)

target_compile_definitions(unified_kernel PUBLIC
    TIXML_USE_STL
)

# Precompiled header
target_precompile_headers(unified_kernel PRIVATE
    ${CMAKE_SOURCE_DIR}/src/stdafx.hpp
)

# Link dependencies
target_link_libraries(unified_kernel PUBLIC
    Boost::thread
    Boost::filesystem
    Boost::date_time
    Boost::python
    OpenSSL::Crypto
    Python3::Python
    PostgreSQL::PostgreSQL
    soci_core_static
    soci_postgresql_static
)

# Platform-specific
if(WIN32)
    target_link_libraries(unified_kernel PUBLIC
        dbghelp
        Winmm
    )
elseif(UNIX)
    target_link_libraries(unified_kernel PUBLIC
        Threads::Threads
    )
endif()
