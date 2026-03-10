# CellApp — spatial entity simulation, world cells, movement, AoI.

add_executable(CellApp
    ../../cellapp/base_client.cpp
    ../../cellapp/cell_service.cpp
    ../../cellapp/cell_launcher.cpp
    ../../cellapp/space.cpp

    # Cell entities
    ../../cellapp/entity/cell_entity.cpp
    ../../cellapp/entity/cell_py_util.cpp
    ../../cellapp/entity/movement.cpp
    ../../cellapp/entity/navigation.cpp
)

target_precompile_headers(CellApp REUSE_FROM unified_kernel)

target_link_libraries(CellApp PRIVATE
    unified_kernel
)

# CellApp uses Recast/Detour for navigation
if(TARGET recast AND TARGET detour)
    target_link_libraries(CellApp PRIVATE recast detour)
endif()
