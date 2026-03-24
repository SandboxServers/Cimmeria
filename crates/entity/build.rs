fn main() {
    cc::Build::new()
        .cpp(true)
        .include("../../external/recast/Detour/Include")
        // Detour library sources
        .file("../../external/recast/Detour/Source/DetourAssert.cpp")
        .file("../../external/recast/Detour/Source/DetourAlloc.cpp")
        .file("../../external/recast/Detour/Source/DetourCommon.cpp")
        .file("../../external/recast/Detour/Source/DetourNavMesh.cpp")
        .file("../../external/recast/Detour/Source/DetourNavMeshBuilder.cpp")
        .file("../../external/recast/Detour/Source/DetourNavMeshQuery.cpp")
        .file("../../external/recast/Detour/Source/DetourNode.cpp")
        // Our thin C wrapper
        .file("detour_wrapper.cpp")
        .compile("detour");
}
