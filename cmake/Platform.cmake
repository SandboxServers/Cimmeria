# Platform detection and package configuration for Cimmeria.

# --- Compiler settings ---
if(MSVC)
    add_compile_options(/W3 /EHsc /permissive-)
    add_compile_definitions(
        WIN32_LEAN_AND_MEAN
        NOMINMAX
        _CRT_SECURE_NO_WARNINGS
        _CRT_SECURE_NO_DEPRECATE
        _WIN32_WINNT=0x0600
        BOOST_PYTHON_STATIC_LIB
    )
elseif(APPLE)
    add_compile_options(-Wall -Wextra -Wpedantic)
    set(CMAKE_MACOSX_RPATH TRUE)
    set(CMAKE_INSTALL_RPATH_USE_LINK_PATH TRUE)
elseif(UNIX)
    add_compile_options(-Wall -Wextra -Wpedantic)
    find_package(Threads REQUIRED)
endif()

# --- Find packages ---

# Boost (CONFIG mode — FindBoost was removed in CMake 4.x)
# Note: system is header-only since Boost 1.69, so not listed as a component.
find_package(Boost CONFIG REQUIRED COMPONENTS
    thread
    filesystem
    date_time
    python
)

# OpenSSL
find_package(OpenSSL REQUIRED)

# Python 3
find_package(Python3 REQUIRED COMPONENTS Development)

# PostgreSQL (client library)
find_package(PostgreSQL REQUIRED)
