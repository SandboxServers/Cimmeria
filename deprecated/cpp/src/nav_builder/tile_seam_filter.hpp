#pragma once

// Removes the small islands a tiled build leaves along its tile seams.
//
// rcBuildRegions drops a connected group of regions smaller than
// minRegionArea, but never one that touches the tile border
// (filterSmallRegions: `connectsToBorder`), because it cannot see whether
// the group continues in the neighbouring tile. In a single-mesh build
// there is no tile border, so every small island goes. In a tiled build
// every small island that happens to straddle a seam survives, once per
// tile it touches: on Agnos in 128-cell tiles that was 1,262 components
// under 10 m^2 against none in the single-mesh build.
//
// This pass does what the region filter could not: it joins the tiles the
// way dtNavMesh::connectExtLinks will at load time (same-line portal
// edges whose slabs overlap within walkableClimb), measures each connected
// component's walkable area, and deletes every component smaller than
// `minArea` square metres - minRegionSize^2 * cs^2, the single-mesh
// threshold.

#include <vector>

#include "xrc_writer.hpp"

struct SeamFilterStats
{
	unsigned int components;
	unsigned int removedComponents;
	unsigned int removedPolys;
	unsigned int emptiedTiles;
};

// Filters `tiles` in place. Every tile's polygon, vertex and detail arrays
// are compacted; tiles left with no polygon are removed from the vector
// (their meshes freed). `climb` is the agent climb in metres, which Detour
// uses as the portal height tolerance.
SeamFilterStats filterSeamFragments(std::vector<XrcTile> & tiles, float minArea, float climb);
