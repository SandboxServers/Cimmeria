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

# Include paths — on non-Windows, create a shadow include directory that
# excludes the vendored OpenSSL 0.9.8i headers in src/openssl/.
# These conflict with system OpenSSL 3.x because -I beats -isystem.
if(NOT WIN32)
    set(_shadow_inc ${CMAKE_BINARY_DIR}/include)
    file(MAKE_DIRECTORY ${_shadow_inc})
    file(GLOB _src_items LIST_DIRECTORIES true ${CMAKE_SOURCE_DIR}/src/*)
    foreach(_item ${_src_items})
        get_filename_component(_name ${_item} NAME)
        if(NOT _name STREQUAL "openssl")
            if(NOT EXISTS ${_shadow_inc}/${_name})
                file(CREATE_LINK ${_item} ${_shadow_inc}/${_name} SYMBOLIC)
            endif()
        endif()
    endforeach()
    target_include_directories(unified_kernel PUBLIC ${_shadow_inc})
else()
    target_include_directories(unified_kernel PUBLIC ${CMAKE_SOURCE_DIR}/src)
endif()

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
    soci_core
    soci_postgresql
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
