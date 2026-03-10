# NavBuilder — offline navigation mesh generation tool.

add_executable(NavBuilder
    ../../nav_builder/builder.cpp
    ../../nav_builder/chunk.cpp
    ../../nav_builder/mesh.cpp
    ../../nav_builder/mesh_exporter.cpp
)

target_precompile_headers(NavBuilder REUSE_FROM unified_kernel)

target_link_libraries(NavBuilder PRIVATE
    unified_kernel
)

# NavBuilder requires Recast/Detour
if(TARGET recast AND TARGET detour)
    target_link_libraries(NavBuilder PRIVATE recast detour)
else()
    message(WARNING "Recast/Detour not found. NavBuilder will not link correctly.")
endif()

# SDL for mesh visualization (optional)
find_package(SDL QUIET)
if(SDL_FOUND)
    target_link_libraries(NavBuilder PRIVATE ${SDL_LIBRARIES})
    target_include_directories(NavBuilder PRIVATE ${SDL_INCLUDE_DIRS})
endif()
