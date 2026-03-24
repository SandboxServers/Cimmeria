// Thin C shim around Detour's C++ class API for Rust FFI.
// See detour_wrapper.h for the public interface.

#include "detour_wrapper.h"

#include <cstring>

#include "DetourNavMesh.h"
#include "DetourNavMeshBuilder.h"
#include "DetourNavMeshQuery.h"
#include "DetourCommon.h"

// Each query handle bundles a dtNavMeshQuery + dtQueryFilter so the
// Rust side doesn't have to manage filter lifetime separately.
struct DetourQueryCtx {
    dtNavMeshQuery query;
    dtQueryFilter filter;
};

// ── Navmesh tile building ───────────────────────────────────────────────

extern "C" int detour_build_navmesh_data(
    const unsigned short* verts, int nverts,
    const unsigned short* polys, int npolys, int nvp,
    const unsigned short* flags, const unsigned char* areas,
    const float bmin[3], const float bmax[3],
    float cs, float ch,
    float agentHeight, float agentRadius, float agentMaxClimb,
    const unsigned int* detailMeshes, int ndetailMeshes,
    const float* detailVerts, int ndetailVerts,
    const unsigned char* detailTris, int ndetailTris,
    unsigned char** outData, int* outDataSize)
{
    dtNavMeshCreateParams params;
    memset(&params, 0, sizeof(params));

    params.verts = verts;
    params.vertCount = nverts;
    params.polys = polys;
    params.polyFlags = flags;
    params.polyAreas = areas;
    params.polyCount = npolys;
    params.nvp = nvp;

    params.detailMeshes = detailMeshes;
    params.detailVerts = detailVerts;
    params.detailVertsCount = ndetailVerts;
    params.detailTris = detailTris;
    params.detailTriCount = ndetailTris;

    params.walkableHeight = agentHeight;
    params.walkableRadius = agentRadius;
    params.walkableClimb = agentMaxClimb;

    dtVcopy(params.bmin, bmin);
    dtVcopy(params.bmax, bmax);
    params.cs = cs;
    params.ch = ch;
    params.buildBvTree = true;

    return dtCreateNavMeshData(&params, outData, outDataSize) ? 1 : 0;
}

extern "C" void detour_free_data(unsigned char* data)
{
    dtFree(data);
}

// ── Navmesh lifecycle ───────────────────────────────────────────────────

extern "C" DetourNavMeshHandle detour_create_navmesh(const unsigned char* data, int dataSize)
{
    // Detour takes ownership of the data when DT_TILE_FREE_DATA is set,
    // but we need a copy because the caller's buffer may be freed separately.
    // Actually — the caller allocated via dtCreateNavMeshData (which uses dtAlloc),
    // so we can transfer ownership directly. But the Rust side passes us a pointer
    // it got from detour_build_navmesh_data, and it will call detour_free_data on
    // that same pointer — so we MUST copy here if using DT_TILE_FREE_DATA.
    //
    // Alternatively, don't use DT_TILE_FREE_DATA and free in detour_free_navmesh.
    // That's simpler. Let's do that.
    //
    // Wait — the C++ reference uses DT_TILE_FREE_DATA and the data allocated by
    // dtCreateNavMeshData. If we copy and pass DT_TILE_FREE_DATA, dtNavMesh will
    // free its copy on destruction. That's the cleanest approach.
    unsigned char* copy = (unsigned char*)dtAlloc(dataSize, DT_ALLOC_PERM);
    if (!copy)
        return nullptr;
    memcpy(copy, data, dataSize);

    dtNavMesh* mesh = dtAllocNavMesh();
    if (!mesh) {
        dtFree(copy);
        return nullptr;
    }

    dtStatus status = mesh->init(copy, dataSize, DT_TILE_FREE_DATA);
    if (dtStatusFailed(status)) {
        dtFreeNavMesh(mesh);
        // DT_TILE_FREE_DATA means mesh->init took ownership even on failure?
        // No — on failure it doesn't take ownership. Free our copy.
        // Actually dtFreeNavMesh already handles cleanup. But if init failed
        // before taking ownership, we'd leak. Let's be safe:
        // dtFreeNavMesh calls the destructor which frees all tiles. If init
        // failed, no tile was added, so copy is leaked. Free it.
        // But wait — dtFreeNavMesh deletes the mesh which calls ~dtNavMesh
        // which calls dtFree on each tile's data. If init failed, the tile
        // was never added, so we need to free copy ourselves.
        // Hmm, dtFreeNavMesh calls dtFree on the mesh, not the data.
        // On init failure with DT_TILE_FREE_DATA, the data is NOT freed.
        // So we must free it here. But we already called dtFreeNavMesh...
        // which only frees the mesh struct, not unreferenced data.
        dtFree(copy);
        return nullptr;
    }

    return (DetourNavMeshHandle)mesh;
}

extern "C" void detour_free_navmesh(DetourNavMeshHandle handle)
{
    if (handle)
        dtFreeNavMesh((dtNavMesh*)handle);
}

// ── Query lifecycle ─────────────────────────────────────────────────────

extern "C" DetourQueryHandle detour_create_query(DetourNavMeshHandle mesh, int maxNodes)
{
    if (!mesh) return nullptr;

    DetourQueryCtx* ctx = new DetourQueryCtx;
    // Match C++ reference: only include polys with flag 0x01
    ctx->filter.setIncludeFlags(0x01);
    ctx->filter.setExcludeFlags(0);

    dtStatus status = ctx->query.init((const dtNavMesh*)mesh, maxNodes);
    if (dtStatusFailed(status)) {
        delete ctx;
        return nullptr;
    }

    return (DetourQueryHandle)ctx;
}

extern "C" void detour_free_query(DetourQueryHandle handle)
{
    delete (DetourQueryCtx*)handle;
}

// ── Queries ─────────────────────────────────────────────────────────────

extern "C" dtStatus detour_find_nearest_poly(DetourQueryHandle query,
    const float* center, const float* extents,
    dtPolyRef* nearestRef, float* nearestPt)
{
    DetourQueryCtx* ctx = (DetourQueryCtx*)query;
    return ctx->query.findNearestPoly(center, extents, &ctx->filter, nearestRef, nearestPt);
}

extern "C" dtStatus detour_find_path(DetourQueryHandle query,
    dtPolyRef startRef, dtPolyRef endRef,
    const float* startPos, const float* endPos,
    dtPolyRef* path, int* pathCount, int maxPath)
{
    DetourQueryCtx* ctx = (DetourQueryCtx*)query;
    return ctx->query.findPath(startRef, endRef, startPos, endPos, &ctx->filter, path, pathCount, maxPath);
}

extern "C" dtStatus detour_find_straight_path(DetourQueryHandle query,
    const float* startPos, const float* endPos,
    const dtPolyRef* path, int pathCount,
    float* straightPath, int* straightPathCount, int maxStraightPath)
{
    DetourQueryCtx* ctx = (DetourQueryCtx*)query;
    return ctx->query.findStraightPath(startPos, endPos, path, pathCount,
        straightPath, nullptr, nullptr, straightPathCount, maxStraightPath);
}

extern "C" int detour_raycast(DetourQueryHandle query,
    dtPolyRef startRef, const float* startPos, const float* endPos,
    float* hitNormal, float* t)
{
    DetourQueryCtx* ctx = (DetourQueryCtx*)query;
    float hitT = 0.0f;
    float hitNorm[3] = {0};
    dtStatus status = ctx->query.raycast(startRef, startPos, endPos, &ctx->filter,
        &hitT, hitNorm, nullptr, nullptr, 0);

    if (hitNormal) {
        hitNormal[0] = hitNorm[0];
        hitNormal[1] = hitNorm[1];
        hitNormal[2] = hitNorm[2];
    }
    if (t) *t = hitT;

    if (dtStatusFailed(status))
        return 0;

    // hitT >= 1.0 means ray reached endPos without hitting anything
    return (hitT >= 1.0f) ? 1 : 0;
}

extern "C" dtStatus detour_get_poly_height(DetourQueryHandle query,
    dtPolyRef ref_, const float* pos, float* height)
{
    DetourQueryCtx* ctx = (DetourQueryCtx*)query;
    return ctx->query.getPolyHeight(ref_, pos, height);
}

extern "C" dtStatus detour_closest_point_on_poly(DetourQueryHandle query,
    dtPolyRef ref_, const float* pos, float* closest)
{
    DetourQueryCtx* ctx = (DetourQueryCtx*)query;
    return ctx->query.closestPointOnPoly(ref_, pos, closest, nullptr);
}

extern "C" dtStatus detour_move_along_surface(DetourQueryHandle query,
    dtPolyRef startRef, const float* startPos, const float* endPos,
    float* resultPos, dtPolyRef* visited, int* visitedCount, int maxVisited)
{
    DetourQueryCtx* ctx = (DetourQueryCtx*)query;
    return ctx->query.moveAlongSurface(startRef, startPos, endPos, &ctx->filter,
        resultPos, visited, visitedCount, maxVisited);
}
