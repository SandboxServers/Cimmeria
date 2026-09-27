#pragma once

// Writers for the two XRC .nav layouts the server loads
// (crates/entity/src/navigation/load.rs).
//
// Single mesh (what NavBuilder has always written):
//
//   agentHeight, agentClimb, agentRadius       3 x f32
//   <poly-mesh block>
//
// Tiled ("XRCT"):
//
//   magic "XRCT", version u32 = 1
//   agentHeight, agentClimb, agentRadius       3 x f32
//   orig                                       3 x f32  dtNavMeshParams::orig
//   tileWidth, tileHeight                      2 x f32  world metres along X / Z
//   ntiles, maxTilePolys                       2 x u32
//   ntiles x { tileX i32, tileY i32, <poly-mesh block> }
//
// <poly-mesh block>:
//
//   nverts, npolys, nvp, borderSize            4 x u32
//   cs, ch                                     2 x f32
//   bmin, bmax                                 6 x f32
//   verts   nverts x 3 x u16,  polys npolys x nvp x 2 x u16
//   regs    npolys x u16,      flags npolys x u16,  areas npolys x u8
//   detailMeshes, detailVerts, detailTris      3 x u32
//   meshes  detailMeshes x 4 x u32,  verts detailVerts x 3 x f32,
//   tris    detailTris x 4 x u8
//
// All little-endian, no padding. The first four bytes of a single-mesh
// file are agentHeight as an f32; "XRCT" read that way is ~3.4e12, so the
// magic cannot collide with a real agent height.

#include <ostream>
#include <vector>

#include "Recast.h"

void xrcSavePolyMesh(rcPolyMesh & mesh, rcPolyMeshDetail & detail, float agentHeight, float agentClimb, float agentRadius, std::ostream & stream);

struct XrcTile
{
	int tileX;
	int tileY;
	rcPolyMesh * mesh;
	rcPolyMeshDetail * detail;
	// Not written: compact spans per region id, for the seam filter
	// (tile_seam_filter.hpp).
	std::vector<int> regionSpans;
};

void xrcSaveTiledMesh(std::vector<XrcTile> const & tiles, float agentHeight, float agentClimb, float agentRadius,
	const float orig[3], float tileWidth, float tileHeight, unsigned int maxTilePolys, std::ostream & stream);
