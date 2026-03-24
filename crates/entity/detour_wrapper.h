// Thin C shim around Detour's C++ class API for Rust FFI.
// Keep this minimal — only expose what the server actually needs.
#pragma once

#ifdef __cplusplus
extern "C" {
#endif

typedef void* DetourNavMeshHandle;
typedef void* DetourQueryHandle;
typedef unsigned int dtPolyRef;
typedef unsigned int dtStatus;

// ── Navmesh tile building ───────────────────────────────────────────────
// Converts raw polygon mesh + detail mesh arrays (from XRC parse) into
// a serialized Detour tile. Caller passes output to detour_create_navmesh.
// Returns 1 on success, 0 on failure.
int detour_build_navmesh_data(
    const unsigned short* verts, int nverts,
    const unsigned short* polys, int npolys, int nvp,
    const unsigned short* flags, const unsigned char* areas,
    const float bmin[3], const float bmax[3],
    float cs, float ch,
    float agentHeight, float agentRadius, float agentMaxClimb,
    // Detail mesh (required for accurate height queries)
    const unsigned int* detailMeshes, int ndetailMeshes,
    const float* detailVerts, int ndetailVerts,
    const unsigned char* detailTris, int ndetailTris,
    // Output: caller must free with detour_free_data
    unsigned char** outData, int* outDataSize);

void detour_free_data(unsigned char* data);

// ── Navmesh + query lifecycle ───────────────────────────────────────────
DetourNavMeshHandle detour_create_navmesh(const unsigned char* data, int dataSize);
void detour_free_navmesh(DetourNavMeshHandle handle);

DetourQueryHandle detour_create_query(DetourNavMeshHandle mesh, int maxNodes);
void detour_free_query(DetourQueryHandle handle);

// ── Queries ─────────────────────────────────────────────────────────────
dtStatus detour_find_nearest_poly(DetourQueryHandle query,
    const float* center, const float* extents,
    dtPolyRef* nearestRef, float* nearestPt);

dtStatus detour_find_path(DetourQueryHandle query,
    dtPolyRef startRef, dtPolyRef endRef,
    const float* startPos, const float* endPos,
    dtPolyRef* path, int* pathCount, int maxPath);

dtStatus detour_find_straight_path(DetourQueryHandle query,
    const float* startPos, const float* endPos,
    const dtPolyRef* path, int pathCount,
    float* straightPath, int* straightPathCount, int maxStraightPath);

// Returns 1 if ray reaches endPos unblocked, 0 if it hits a boundary.
// hitNormal receives the hit surface normal (if blocked), t receives
// the parametric hit distance (1.0 = reached endPos).
int detour_raycast(DetourQueryHandle query,
    dtPolyRef startRef, const float* startPos, const float* endPos,
    float* hitNormal, float* t);

dtStatus detour_get_poly_height(DetourQueryHandle query,
    dtPolyRef ref_, const float* pos, float* height);

dtStatus detour_closest_point_on_poly(DetourQueryHandle query,
    dtPolyRef ref_, const float* pos, float* closest);

dtStatus detour_move_along_surface(DetourQueryHandle query,
    dtPolyRef startRef, const float* startPos, const float* endPos,
    float* resultPos, dtPolyRef* visited, int* visitedCount, int maxVisited);

#ifdef __cplusplus
}
#endif
