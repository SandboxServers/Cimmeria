#pragma once

// Tiled Recast build (`tile=<cells>`): one rcPolyMesh per tile, written as
// the "XRCT" layout (xrc_writer.hpp) and joined into one dtNavMesh by the
// server's loader.
//
// Why: every Recast index cap (24-bit compact-cell index, 16-bit contour
// vertices, 16-bit adjacency edges; docs/engine/navbuilder-recast-limits.md)
// is per rcPolyMesh. A whole outdoor map at cs=0.3 blows all three; a
// 128-cell tile of it cannot.

#include <string>

#include "Recast.h"
#include "build_params.hpp"

// `config` is the whole-map config the single-mesh build would have used
// (bounds, cs/ch, derived cell counts, width/height). Returns an ExitCode.
int buildTiledNavmesh(rcConfig const & config, BuildParams const & params,
	const float * verts, int nverts, const unsigned int * faces, int nfaces,
	std::string const & navmeshFile);
