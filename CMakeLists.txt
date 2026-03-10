cmake_minimum_required(VERSION 3.20)
project(Cimmeria CXX)

# --- Global settings ---
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Output directories
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin)
set(CMAKE_LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib)
set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib)

# --- Platform configuration ---
include(cmake/Platform.cmake)

# --- Vendored dependencies ---

# SOCI (fetched at configure time)
include(FetchContent)
FetchContent_Declare(
    soci
    GIT_REPOSITORY https://github.com/SOCI/soci.git
    GIT_TAG        v4.1.0
)
set(SOCI_SHARED OFF CACHE BOOL "" FORCE)
set(SOCI_STATIC ON CACHE BOOL "" FORCE)
set(SOCI_TESTS OFF CACHE BOOL "" FORCE)
set(SOCI_CXX11 ON CACHE BOOL "" FORCE)
set(WITH_POSTGRESQL ON CACHE BOOL "" FORCE)
set(WITH_SQLITE3 OFF CACHE BOOL "" FORCE)
set(WITH_MYSQL OFF CACHE BOOL "" FORCE)
set(WITH_ODBC OFF CACHE BOOL "" FORCE)
set(WITH_BOOST OFF CACHE BOOL "" FORCE)
set(SOCI_POSTGRESQL_INCLUDE_DIR ${PostgreSQL_INCLUDE_DIRS} CACHE STRING "" FORCE)
FetchContent_MakeAvailable(soci)

# Recast/Detour (vendored in external/)
if(EXISTS ${CMAKE_SOURCE_DIR}/external/recast/CMakeLists.txt)
    add_subdirectory(external/recast EXCLUDE_FROM_ALL)
endif()

# TinyXML2 is compiled as part of UnifiedKernel (src/xml/tinyxml2.cpp)

# --- Project targets ---
add_subdirectory(src/lib)
add_subdirectory(src/server/AuthenticationServer)
add_subdirectory(src/server/BaseApp)
add_subdirectory(src/server/CellApp)
add_subdirectory(src/server/NavBuilder)

# ServerEd (Qt admin tool — optional, only if Qt5 found)
if(EXISTS ${CMAKE_SOURCE_DIR}/tools/ServerEd/CMakeLists.txt)
    add_subdirectory(tools/ServerEd)
endif()
