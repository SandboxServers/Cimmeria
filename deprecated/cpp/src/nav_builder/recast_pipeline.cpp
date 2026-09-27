#include "stdafx.hpp"

#include "recast_pipeline.hpp"

LoggingContext::LoggingContext(std::string const & prefix, unsigned int maxDanglingShown)
	: rcContext(true), prefix_(prefix), maxDanglingShown_(maxDanglingShown), danglingFaces_(0)
{
}

void LoggingContext::reportSuppressed()
{
	if (danglingFaces_ > maxDanglingShown_)
		WARN("%sdelaunayHull: %u 'Removing dangling face' warnings in total (%u not shown)",
			prefix_.c_str(), danglingFaces_, danglingFaces_ - maxDanglingShown_);
}

void LoggingContext::doLog(const rcLogCategory category, const char * msg, const int len)
{
	std::string text(msg, len > 0 ? (size_t)len : 0);
	if (category == RC_LOG_WARNING && text.find("Removing dangling face") != std::string::npos
		&& ++danglingFaces_ > maxDanglingShown_)
		return;

	switch (category)
	{
	case RC_LOG_ERROR: FAULT("%s%s", prefix_.c_str(), text.c_str()); break;
	case RC_LOG_WARNING: WARN("%s%s", prefix_.c_str(), text.c_str()); break;
	default: DEBUG1("%s%s", prefix_.c_str(), text.c_str()); break;
	}
}

unsigned int countAdjacencyEdges(rcPolyMesh const & mesh)
{
	unsigned int adjacencyEdges = 0;
	for (int i = 0; i < mesh.npolys; i++)
	{
		const unsigned short * poly = &mesh.polys[i * mesh.nvp * 2];
		for (int j = 0; j < mesh.nvp; j++)
		{
			if (poly[j] == RC_MESH_NULL_IDX)
				break;
			unsigned short next = (j + 1 >= mesh.nvp || poly[j + 1] == RC_MESH_NULL_IDX) ? poly[0] : poly[j + 1];
			if (poly[j] < next)
				adjacencyEdges++;
		}
	}
	return adjacencyEdges;
}

namespace
{
	// Frees whatever the pipeline allocated when it leaves early.
	struct PipelineScratch
	{
		rcHeightfield * heightfield;
		rcCompactHeightfield * compact;
		rcContourSet * contours;
		rcPolyMesh * polyMesh;
		rcPolyMeshDetail * detail;

		PipelineScratch() : heightfield(0), compact(0), contours(0), polyMesh(0), detail(0) {}
		~PipelineScratch()
		{
			rcFreeHeightField(heightfield);
			rcFreeCompactHeightfield(compact);
			rcFreeContourSet(contours);
			rcFreePolyMesh(polyMesh);
			rcFreePolyMeshDetail(detail);
		}
	};
}

PipelineResult runRecastPipeline(LoggingContext & ctx, rcConfig const & config, BuildParams const & params,
	RecastInput const & in, std::string const & where, bool verbose,
	rcPolyMesh ** outMesh, rcPolyMeshDetail ** outDetail, PipelineStats & stats,
	std::vector<int> * regionSpans)
{
	const char * at = where.c_str();
	memset(&stats, 0, sizeof(stats));
	*outMesh = 0;
	*outDetail = 0;
	PipelineScratch s;

	if (verbose)
		DEBUG1("Rasterizing triangles ...");
	s.heightfield = rcAllocHeightfield();
	if (!s.heightfield || !rcCreateHeightfield(&ctx, *s.heightfield, config.width, config.height, config.bmin, config.bmax, config.cs, config.ch))
	{
		FAULT("%sFailed to create heightfield", at);
		return PIPELINE_FAILED;
	}

	if (!rcRasterizeTriangles(&ctx, in.verts, in.nverts, in.tris, in.areas, in.ntris, *s.heightfield, config.walkableClimb))
	{
		FAULT("%sFailed to rasterize triangles", at);
		return PIPELINE_FAILED;
	}

	if (verbose)
		DEBUG1("Filtering walkable surfaces ...");
	// See: http://digestingduck.blogspot.com/2010/01/rough-fringes.html
	rcFilterLowHangingWalkableObstacles(&ctx, config.walkableClimb, *s.heightfield);
	rcFilterLedgeSpans(&ctx, config.walkableHeight, config.walkableClimb, *s.heightfield);
	rcFilterWalkableLowHeightSpans(&ctx, config.walkableHeight, *s.heightfield);

	// rcCompactCell packs the index of a column's first span into a
	// 24-bit field (Recast.h: `unsigned int index : 24`) and
	// rcBuildCompactHeightfield assigns it with no overflow check. Past
	// 0xffffff spans every column after the wrap points at the wrong
	// run of spans; rcBuildRegions then finds nothing walkable and the
	// whole build comes out empty with no Recast diagnostic at all.
	//
	// The span count grows as 1/cs^2, so this is the limit that bites
	// first when you refine cs on a map-sized area. Measured on the
	// Castle interior crop bounds=150,500,700,1150: cs=0.25 builds,
	// cs=0.2 builds (and then trips the edge cap), cs=0.15 produced
	// "Regions: 1" and an empty mesh at exit 0 before this check.
	stats.spans = rcGetHeightFieldSpanCount(&ctx, *s.heightfield);
	if (verbose)
		INFO("Heightfield: %d spans over %d x %d columns (cap 16777215; rcCompactCell::index is 24-bit)",
			stats.spans, config.width, config.height);
	if (stats.spans > 0xffffff)
	{
		FAULT("%sHeightfield has %d spans; rcCompactCell indexes them with 24 bits (max 16777215), so the "
			"compact heightfield would be silently corrupt and the build would produce an empty mesh. "
			"Raise cs, or crop with bounds=", at, stats.spans);
		return PIPELINE_FAILED;
	}

	if (verbose)
		DEBUG1("Partitioning surface ...");
	s.compact = rcAllocCompactHeightfield();
	if (!s.compact)
	{
		FAULT("%sFailed to create compact heightfield", at);
		return PIPELINE_FAILED;
	}
	if (!rcBuildCompactHeightfield(&ctx, config.walkableHeight, config.walkableClimb, *s.heightfield, *s.compact))
	{
		FAULT("%sFailed to compact heightfield", at);
		return PIPELINE_FAILED;
	}

	rcFreeHeightField(s.heightfield);
	s.heightfield = 0;

	// Erode the walkable area by agent radius.
	if (!rcErodeWalkableArea(&ctx, config.walkableRadius, *s.compact))
	{
		FAULT("%sFailed to erode walkable area", at);
		return PIPELINE_FAILED;
	}

	if (!params.watershed)
	{
		// Partition the walkable surface into simple regions without holes.
		// Monotone partitioning does not need distancefield.
		if (!rcBuildRegionsMonotone(&ctx, *s.compact, config.borderSize, config.minRegionArea, config.mergeRegionArea))
		{
			FAULT("%sFailed to build monotone regions", at);
			return PIPELINE_FAILED;
		}
	}
	else
	{
		// Prepare for region partitioning, by calculating distance field along the walkable surface.
		if (!rcBuildDistanceField(&ctx, *s.compact))
		{
			FAULT("%sFailed to build distance field", at);
			return PIPELINE_FAILED;
		}

		// Partition the walkable surface into simple regions without holes.
		if (!rcBuildRegions(&ctx, *s.compact, config.borderSize, config.minRegionArea, config.mergeRegionArea))
		{
			FAULT("%sFailed to build regions", at);
			return PIPELINE_FAILED;
		}
	}

	// Region ids are 16-bit with the top bit reserved (RC_BORDER_REG).
	// rcBuildRegions checks for overflow; rcBuildRegionsMonotone does not
	// and crashes or wraps on very large maps - prefer partition=watershed
	// there.
	stats.regions = s.compact->maxRegions;
	if (verbose)
		INFO("Regions: %d (after merge/filter; ids are 15-bit)", stats.regions);

	if (regionSpans)
	{
		regionSpans->assign(s.compact->maxRegions + 1, 0);
		for (int i = 0; i < s.compact->spanCount; i++)
		{
			const unsigned short reg = s.compact->spans[i].reg;
			if (reg != 0 && !(reg & RC_BORDER_REG) && reg <= s.compact->maxRegions)
				(*regionSpans)[reg]++;
		}
	}

	if (verbose)
		DEBUG1("Simplifying region contours ...");
	s.contours = rcAllocContourSet();
	if (!s.contours)
	{
		FAULT("%sFailed to allocate contour set", at);
		return PIPELINE_FAILED;
	}

	if (!rcBuildContours(&ctx, *s.compact, config.maxSimplificationError, config.maxEdgeLen, *s.contours))
	{
		FAULT("%sCould not create contours", at);
		return PIPELINE_FAILED;
	}

	// rcBuildPolyMesh caps the *sum* of contour vertices (before it
	// de-duplicates them) at 0xfffe, so print the number it will test.
	stats.contours = s.contours->nconts;
	for (int i = 0; i < s.contours->nconts; i++)
	{
		if (s.contours->conts[i].nverts >= 3)
			stats.contourVerts += s.contours->conts[i].nverts;
	}
	if (verbose)
		INFO("Contours: %d with %d vertices (cap 65534)", stats.contours, stats.contourVerts);

	if (verbose)
		DEBUG1("Building polygon mesh ...");
	s.polyMesh = rcAllocPolyMesh();
	if (!s.polyMesh)
	{
		FAULT("%sFailed to allocate polygon mesh", at);
		return PIPELINE_FAILED;
	}
	if (!rcBuildPolyMesh(&ctx, *s.contours, config.maxVertsPerPoly, *s.polyMesh))
	{
		FAULT("%sCould not triangulate contours (a single Recast poly mesh is capped at 0xfffe = 65534 vertices; see the Recast line above)", at);
		return PIPELINE_FAILED;
	}

	// rcBuildPolyMesh's buildMeshAdjacency() stores edge indices in
	// unsigned shorts (RecastMesh.cpp: `firstEdge[v0] = (unsigned short)edgeCount`)
	// with no overflow check. Past 0xffff edges the neighbour links are
	// silently corrupted: the mesh still saves and loads, but falls apart
	// into thousands of disconnected fragments. Count edges exactly the way
	// Recast does and refuse to write such a mesh.
	stats.adjacencyEdges = countAdjacencyEdges(*s.polyMesh);
	if (verbose)
		INFO("Poly mesh: nverts=%d npolys=%d adjacencyEdges=%u (caps: 65534 verts, 65535 edges; edges ~ nverts + npolys)",
			s.polyMesh->nverts, s.polyMesh->npolys, stats.adjacencyEdges);

	// An empty poly mesh is a failed single-mesh build (the caller says so)
	// and a skipped tile in a tiled one.
	if (s.polyMesh->npolys == 0)
		return PIPELINE_EMPTY;

	if (stats.adjacencyEdges > 0xffff)
	{
		FAULT("%sPoly mesh has %u adjacency edges; Recast indexes them with 16 bits (max 65535), so polygon "
			"connectivity is corrupt. Reduce detail (maxSimplificationError, minRegionSize) or crop with bounds=",
			at, stats.adjacencyEdges);
		return PIPELINE_FAILED;
	}

	if (verbose)
		DEBUG1("Building detail mesh ...");
	s.detail = rcAllocPolyMeshDetail();
	if (!s.detail)
	{
		FAULT("%sFailed to allocate detailed polygon mesh", at);
		return PIPELINE_FAILED;
	}

	if (!rcBuildPolyMeshDetail(&ctx, *s.polyMesh, *s.compact, config.detailSampleDist, config.detailSampleMaxError, *s.detail))
	{
		FAULT("%sCould not build detail mesh", at);
		return PIPELINE_FAILED;
	}
	if (verbose)
		ctx.reportSuppressed();

	// Mark all polygons as walkable
	// (needed as the poly filter won't work if flags is set to zero)
	for (int i = 0; i < s.polyMesh->npolys; i++)
		s.polyMesh->flags[i] = 0x01;

	*outMesh = s.polyMesh;
	*outDetail = s.detail;
	s.polyMesh = 0;
	s.detail = 0;
	return PIPELINE_OK;
}
