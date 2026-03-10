# ServerEd — Qt-based server administration tool (optional).

find_package(Qt5 QUIET COMPONENTS Core Gui Widgets Xml Network Sql)
if(NOT Qt5_FOUND)
    message(STATUS "Qt5 not found — skipping ServerEd build.")
    return()
endif()

set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTOUIC ON)
set(CMAKE_AUTORCC ON)

file(GLOB SERVERED_SOURCES *.cpp)
file(GLOB SERVERED_HEADERS *.h)
file(GLOB SERVERED_UI *.ui)
file(GLOB SERVERED_QRC *.qrc)

add_executable(ServerEd
    ${SERVERED_SOURCES}
    ${SERVERED_HEADERS}
    ${SERVERED_UI}
    ${SERVERED_QRC}
)

target_link_libraries(ServerEd PRIVATE
    Qt5::Core
    Qt5::Gui
    Qt5::Widgets
    Qt5::Xml
    Qt5::Network
    Qt5::Sql
)

set_target_properties(ServerEd PROPERTIES
    RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin
)
