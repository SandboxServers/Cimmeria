#pragma once

// The Recast pipeline from rasterisation to detail mesh, shared by the
// single-mesh build (builder.cpp) and the tiled build (tiled_builder.cpp).
//
// The single-mesh path calls it with `verbose = true` and an empty `where`,
// which reproduces the log lines and FAULT texts the builder has always
// printed. The tiled path calls it once per tile with `verbose = false` and
// `where = "Tile x,y: "`, and prints one summary line per tile itself.

#include <string>
#include <vector>

#include "Recast.h"
#include "build_params.hpp"

// Forwards Recast's own diagnostics to the NavBuilder logger. A bare
// rcContext discards them, which hides e.g. "rcBuildPolyMesh: Too many
// vertices N" (the 16-bit index cap) behind a generic FAULT.
class LoggingContext : public rcContext
{
public:
	// `prefix` goes in front of every forwarded line ("Recast: " by default).
	// `maxDanglingShown` is how many "Removing dangling face" warnings are
	// printed before the rest are only counted.
	explicit LoggingContext(std::string const & prefix = "Recast: ", unsigned int maxDanglingShown = 3);

	// The detail-mesh pass emits one "Removing dangling face" warning per
	// degenerate hull triangle - thousands on a real map. They are benign, so
	// print the first few and summarise the rest.
	void reportSuppressed();

	unsigned int danglingFaces() const { return danglingFaces_; }

protected:
	virtual void doLog(const rcLogCategory category, const char * msg, const int len);

private:
	std::string prefix_;
	unsigned int maxDanglingShown_;
	unsigned int danglingFaces_;
};

// Triangles handed to the pipeline. `tris` indexes `verts` (three ints per
// triangle), `areas` is one walkability byte per triangle
// (rcMarkWalkableTriangles output).
struct RecastInput
{
	const float * verts;
	int nverts;
	const int * tris;
	const unsigned char * areas;
	int ntris;
};

struct PipelineStats
{
	int spans;
	int regions;
	int contours;
	int contourVerts;
	unsigned int adjacencyEdges;
};

enum PipelineResult
{
	PIPELINE_OK,
	// No walkable polygon survived. The single-mesh build treats this as a
	// failure; the tiled build skips the tile.
	PIPELINE_EMPTY,
	// A Recast stage failed or a Recast index cap was hit. Already logged.
	PIPELINE_FAILED
};

// Runs rcCreateHeightfield .. rcBuildPolyMeshDetail on `in` with `config`
// (whose borderSize, if non-zero, is passed to the region builder so the
// poly mesh marks tile-portal edges). On PIPELINE_OK the caller owns
// `*outMesh` and `*outDetail`; every poly's flags are set to 0x01.
//
// `regionSpans`, when given, receives the number of non-border compact
// spans in each region, indexed by region id (rcPolyMesh::regs) - the
// quantity rcBuildRegions compares with minRegionArea.
PipelineResult runRecastPipeline(LoggingContext & ctx, rcConfig const & config, BuildParams const & params,
	RecastInput const & in, std::string const & where, bool verbose,
	rcPolyMesh ** outMesh, rcPolyMeshDetail ** outDetail, PipelineStats & stats,
	std::vector<int> * regionSpans = 0);

// Adjacency edges exactly as buildMeshAdjacency counts them (one per poly
// side with v0 < v1). Recast stores the index in an unsigned short without
// an overflow check, so more than 0xffff means corrupt neighbour links.
unsigned int countAdjacencyEdges(rcPolyMesh const & mesh);
