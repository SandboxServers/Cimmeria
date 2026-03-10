# BaseApp — persistent entity state, player base data, shard management.

add_executable(BaseApp
    # Auth client
    ../../authentication/logon_queue.cpp
    ../../authentication/shard_client.cpp

    # Base service
    ../../baseapp/base_service.cpp
    ../../baseapp/cell_manager.cpp
    ../../baseapp/launcher.cpp
    ../../baseapp/minigame.cpp
    ../../baseapp/minigame_connection.cpp

    # Base entities
    ../../baseapp/entity/base_entity.cpp
    ../../baseapp/entity/base_py_util.cpp
    ../../baseapp/entity/cached_entity.cpp
    ../../baseapp/entity/ticker.cpp

    # Mercury handlers
    ../../baseapp/mercury/cell_handler.cpp
    ../../baseapp/mercury/sgw/client_handler.cpp
    ../../baseapp/mercury/sgw/connect_handler.cpp
    ../../baseapp/mercury/sgw/messages.cpp
    ../../baseapp/mercury/sgw/resource.cpp
)

target_precompile_headers(BaseApp REUSE_FROM unified_kernel)

target_link_libraries(BaseApp PRIVATE
    unified_kernel
)

# BaseApp uses Recast/Detour for navigation
if(TARGET recast AND TARGET detour)
    target_link_libraries(BaseApp PRIVATE recast detour)
endif()
