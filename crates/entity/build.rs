fn main() {
    // `cc` prints `rerun-if-env-changed` lines, and any `rerun-if-*` line
    // replaces cargo's default "rerun when a package file changes". Without
    // these, an edit to the wrapper is silently not compiled into the next
    // build.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=detour_wrapper.cpp");
    println!("cargo:rerun-if-changed=detour_wrapper.h");
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
