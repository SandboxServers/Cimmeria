#include "stdafx.hpp"

#include <windows.h>
#include "mesh.hpp"
#include "mesh_exporter.hpp"
#include "chunk.hpp"
#include "build_params.hpp"
#include "Recast.h"
#include "DetourNavMesh.h"

void xrcSavePolyMesh(rcPolyMesh & mesh, rcPolyMeshDetail & detail, float agentHeight, float agentClimb, float agentRadius, std::ostream & stream)
{
	uint32_t vertices = mesh.nverts, polys = mesh.npolys, nvp = mesh.nvp, 
		borderSize = mesh.borderSize;
	stream.write((char *)&agentHeight, sizeof(agentHeight));
	stream.write((char *)&agentClimb, sizeof(agentClimb));
	stream.write((char *)&agentRadius, sizeof(agentRadius));
	stream.write((char *)&vertices, sizeof(vertices));
	stream.write((char *)&polys, sizeof(polys));
	stream.write((char *)&nvp, sizeof(nvp));
	stream.write((char *)&borderSize, sizeof(borderSize));
	stream.write((char *)&mesh.cs, sizeof(mesh.cs));
	stream.write((char *)&mesh.ch, sizeof(mesh.ch));
	stream.write((char *)&mesh.bmin, sizeof(mesh.bmin));
	stream.write((char *)&mesh.bmax, sizeof(mesh.bmax));
	stream.write((char *)mesh.verts, vertices * 3 * sizeof(unsigned short));
	stream.write((char *)mesh.polys, polys * nvp * 2 * sizeof(unsigned short));
	stream.write((char *)mesh.regs, polys * sizeof(unsigned short));
	stream.write((char *)mesh.flags, polys * sizeof(unsigned short));
	stream.write((char *)mesh.areas, polys * sizeof(unsigned char));

	uint32_t detailMeshes = detail.nmeshes, detailVerts = detail.nverts, detailTris = detail.ntris;
	stream.write((char *)&detailMeshes, sizeof(detailMeshes));
	stream.write((char *)&detailVerts, sizeof(detailVerts));
	stream.write((char *)&detailTris, sizeof(detailTris));
	stream.write((char *)detail.meshes, 4 * detailMeshes * sizeof(unsigned int));
	stream.write((char *)detail.verts, 3 * detailVerts * sizeof(float));
	stream.write((char *)detail.tris, 4 * detailTris * sizeof(unsigned char));
}

// Exit codes. 0 = success; every failure path returns one of these.
enum ExitCode
{
	EXIT_OK = 0,
	EXIT_USAGE = 1,
	EXIT_INTERNAL_ERROR = 2,
	EXIT_BUILD_FAILED = 3,
	EXIT_OUTPUT_NOT_WRITABLE = 4
};

// Forwards Recast's own diagnostics to the NavBuilder logger. A bare
// rcContext discards them, which hides e.g. "rcBuildPolyMesh: Too many
// vertices N" (the 16-bit index cap) behind a generic FAULT.
class LoggingContext : public rcContext
{
public:
	LoggingContext()
		: rcContext(true), danglingFaces_(0)
	{
	}

	// The detail-mesh pass emits one "Removing dangling face" warning per
	// degenerate hull triangle - thousands on a real map. They are benign, so
	// print the first few and summarise the rest.
	void reportSuppressed()
	{
		if (danglingFaces_ > MAX_DANGLING_FACE_WARNINGS)
			WARN("Recast: delaunayHull: %u 'Removing dangling face' warnings in total (%u not shown)",
				danglingFaces_, danglingFaces_ - MAX_DANGLING_FACE_WARNINGS);
	}

protected:
	virtual void doLog(const rcLogCategory category, const char * msg, const int len)
	{
		std::string text(msg, len > 0 ? (size_t)len : 0);
		if (category == RC_LOG_WARNING && text.find("Removing dangling face") != std::string::npos
			&& ++danglingFaces_ > MAX_DANGLING_FACE_WARNINGS)
			return;

		switch (category)
		{
		case RC_LOG_ERROR: FAULT("Recast: %s", text.c_str()); break;
		case RC_LOG_WARNING: WARN("Recast: %s", text.c_str()); break;
		default: DEBUG1("Recast: %s", text.c_str()); break;
		}
	}

private:
	static const unsigned int MAX_DANGLING_FACE_WARNINGS = 3;
	unsigned int danglingFaces_;
};

class MapExporter
{
public:
	MapExporter()
	{
	}

	void exportMesh(std::string const & meshFile)
	{
		MeshObjExporter exporter(meshFile);
		
		INFO("Saving chunks ...");
		for (auto it = chunks_.begin(); it != chunks_.end(); ++it)
		{
			(*it)->exportVertices(exporter);
		}
	}

	// Returns EXIT_OK, or the exit code describing why no mesh was written.
	int exportNavmesh(std::string const & navmeshFile, BuildParams const & params)
	{
		unsigned int vertices = 0, faces = 0;
		for (auto it = chunks_.begin(); it != chunks_.end(); ++it)
		{
			vertices += (*it)->numVertices();
			faces += (*it)->numFaces();
		}
		
		DEBUG1("Exporting chunks to vertex buffer ...");
		MeshBufferExporter exporter(vertices, faces);
		for (auto it = chunks_.begin(); it != chunks_.end(); ++it)
		{
			(*it)->exportVertices(exporter);
		}
		
		INFO("Parameters: %s", params.describe().c_str());
		INFO("Input: %u vertices, %u triangles in %u chunks", vertices, faces, (unsigned int)chunks_.size());

		float agentHeight = params.agentHeight, agentClimb = params.agentClimb, agentRadius = params.agentRadius;
		rcConfig config;
		memset(&config, 0, sizeof(config));
		config.cs = params.cs;
		config.ch = params.ch;
		config.walkableSlopeAngle = params.slope;
		// Every one of these is a user-supplied quotient or square, and
		// a bare `(int)` cast of a value the type cannot represent is
		// undefined behaviour. `cs=1e-30` is in range on its own and
		// still sends `agentRadius / cs` past INT_MAX; `minRegionSize`
		// squared overflows above ~46341. navbuilderToInt throws, which
		// main turns into a usage exit.
		//
		// The quotients stay in FLOAT, as the original builder computed
		// them; only the range check widens to double. That is not
		// pedantry: 12.0f / 0.3f rounds to exactly 40.0f, while the same
		// division in double is 39.9999984 and truncates to 39 -- a
		// different maxEdgeLen, and therefore a different navmesh at
		// DEFAULT parameters, breaking byte-parity with NavBuilder_d.exe.
		config.walkableHeight = navbuilderToInt("walkableHeight", (double)ceilf(agentHeight / config.ch));
		config.walkableClimb = navbuilderToInt("walkableClimb", (double)floorf(agentClimb / config.ch));
		config.walkableRadius = navbuilderToInt("walkableRadius", (double)ceilf(agentRadius / config.cs));
		config.maxEdgeLen = navbuilderToInt("maxEdgeLen", (double)(params.maxEdgeLen / config.cs));
		config.maxSimplificationError = params.maxSimplificationError;
		config.minRegionArea = navbuilderToInt("minRegionArea",
			(double)params.minRegionSize * (double)params.minRegionSize);
		config.mergeRegionArea = navbuilderToInt("mergeRegionArea",
			(double)params.mergeRegionSize * (double)params.mergeRegionSize);
		config.maxVertsPerPoly = params.maxVertsPerPoly;
		config.detailSampleDist = params.detailSampleDist < 0.9f ? 0.0f : config.cs * params.detailSampleDist;
		config.detailSampleMaxError = config.ch * params.detailSampleMaxError;

		// The cell counts Recast actually receives. Logged because they
		// are where a float-vs-double slip shows up (maxEdgeLen 40 vs
		// 39 at defaults) long before anyone diffs two .nav files;
		// tests/navbuilder_axis_roundtrip.rs pins the default line.
		INFO("Derived cells: walkableHeight=%d walkableClimb=%d walkableRadius=%d maxEdgeLen=%d minRegionArea=%d mergeRegionArea=%d",
			config.walkableHeight, config.walkableClimb, config.walkableRadius,
			config.maxEdgeLen, config.minRegionArea, config.mergeRegionArea);

		for (unsigned int i = 0; i < 3; i++)
		{
			config.bmin[i] = exporter.minBounds()[i];
			config.bmax[i] = exporter.maxBounds()[i];
		}

		for (auto it = chunks_.begin(); it != chunks_.end(); ++it)
		{
			float minX = (*it)->positionX(), minZ = (*it)->positionZ();
			float maxX = minX + (*it)->sizeX(), maxZ = minZ + (*it)->sizeZ();

			if (minX < config.bmin[0])
				config.bmin[0] = minX;
			if (minZ < config.bmin[2])
				config.bmin[2] = minZ;

			if (maxX > config.bmax[0])
				config.bmax[0] = maxX;
			if (maxZ > config.bmax[2])
				config.bmax[2] = maxZ;
		}
		
		// Optional horizontal crop. Recast clips every triangle to the
		// heightfield bounds while rasterizing, so this is all it takes.
		if (params.hasBounds)
		{
			config.bmin[0] = params.bounds[0];
			config.bmin[2] = params.bounds[1];
			config.bmax[0] = params.bounds[2];
			config.bmax[2] = params.bounds[3];
		}

		rcCalcGridSize(config.bmin, config.bmax, config.cs, &config.width, &config.height);
		INFO("Bounds: (%.2f, %.2f, %.2f) - (%.2f, %.2f, %.2f), grid %d x %d",
			config.bmin[0], config.bmin[1], config.bmin[2],
			config.bmax[0], config.bmax[1], config.bmax[2], config.width, config.height);

		// rcSpan stores smin/smax in 13 bits (Recast.h: RC_SPAN_HEIGHT_BITS),
		// and rasterization clamps every span to RC_SPAN_MAX_HEIGHT with no
		// diagnostic. Anything more than 8191 * ch above bmin[1] is flattened
		// onto that ceiling. Measured on Tollana: one prop at y = -1728 put
		// the whole city (y ~ 0) past the cap at ch = 0.2, and the build
		// "succeeded" with a single sheet at y = -90 and nothing else.
		const float heightCells = (config.bmax[1] - config.bmin[1]) / config.ch;
		if (heightCells > (float)RC_SPAN_MAX_HEIGHT)
		{
			FAULT("Vertical extent %.2f m is %.0f cells at ch=%.2f; rcSpan heights are 13-bit (max %d), so every "
				"surface above y=%.2f would be clamped onto one ceiling. Raise ch.",
				config.bmax[1] - config.bmin[1], heightCells, config.ch, RC_SPAN_MAX_HEIGHT,
				config.bmin[1] + RC_SPAN_MAX_HEIGHT * config.ch);
			return EXIT_BUILD_FAILED;
		}

		DEBUG1("Rasterizing triangles ...");
		LoggingContext ctx;
		rcHeightfield * heightfield = rcAllocHeightfield();
		if (!rcCreateHeightfield(&ctx, *heightfield, config.width, config.height, config.bmin, config.bmax, config.cs, config.ch))
		{
			FAULT("Failed to create heightfield");
			return EXIT_BUILD_FAILED;
		}

		uint8_t * triAreas = new uint8_t[faces];
		memset(triAreas, 0, faces);
		rcMarkWalkableTriangles(&ctx, config.walkableSlopeAngle, exporter.vertices(), vertices, (int *)exporter.faces(), faces, triAreas);
		rcRasterizeTriangles(&ctx, exporter.vertices(), vertices, (int *)exporter.faces(), triAreas, faces, *heightfield, config.walkableClimb);
		delete [] triAreas;
		
		DEBUG1("Filtering walkable surfaces ...");
		// See: http://digestingduck.blogspot.com/2010/01/rough-fringes.html
		rcFilterLowHangingWalkableObstacles(&ctx, config.walkableClimb, *heightfield);
		rcFilterLedgeSpans(&ctx, config.walkableHeight, config.walkableClimb, *heightfield);
		rcFilterWalkableLowHeightSpans(&ctx, config.walkableHeight, *heightfield);
		
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
		const int spanCount = rcGetHeightFieldSpanCount(&ctx, *heightfield);
		INFO("Heightfield: %d spans over %d x %d columns (cap 16777215; rcCompactCell::index is 24-bit)",
			spanCount, config.width, config.height);
		if (spanCount > 0xffffff)
		{
			FAULT("Heightfield has %d spans; rcCompactCell indexes them with 24 bits (max 16777215), so the "
				"compact heightfield would be silently corrupt and the build would produce an empty mesh. "
				"Raise cs, or crop with bounds=", spanCount);
			return EXIT_BUILD_FAILED;
		}

		DEBUG1("Partitioning surface ...");
		rcCompactHeightfield * compact = rcAllocCompactHeightfield();
		if (!compact)
		{
			FAULT("Failed to create compact heightfield");
			return EXIT_BUILD_FAILED;
		}
		if (!rcBuildCompactHeightfield(&ctx, config.walkableHeight, config.walkableClimb, *heightfield, *compact))
		{
			FAULT("Failed to compact heightfield");
			return EXIT_BUILD_FAILED;
		}
	
		rcFreeHeightField(heightfield);
		
		// Erode the walkable area by agent radius.
		if (!rcErodeWalkableArea(&ctx, config.walkableRadius, *compact))
		{
			FAULT("Failed to erode walkable area");
			return EXIT_BUILD_FAILED;
		}
		
		if (!params.watershed)
		{
			// Partition the walkable surface into simple regions without holes.
			// Monotone partitioning does not need distancefield.
			if (!rcBuildRegionsMonotone(&ctx, *compact, 0, config.minRegionArea, config.mergeRegionArea))
			{
				FAULT("Failed to build monotone regions");
				return EXIT_BUILD_FAILED;
			}
		}
		else
		{
			// Prepare for region partitioning, by calculating distance field along the walkable surface.
			if (!rcBuildDistanceField(&ctx, *compact))
			{
				FAULT("Failed to build distance field");
				return EXIT_BUILD_FAILED;
			}

			// Partition the walkable surface into simple regions without holes.
			if (!rcBuildRegions(&ctx, *compact, 0, config.minRegionArea, config.mergeRegionArea))
			{
				FAULT("Failed to build regions");
				return EXIT_BUILD_FAILED;
			}
		}
		
		// Region ids are 16-bit with the top bit reserved (RC_BORDER_REG).
		// rcBuildRegions checks for overflow; rcBuildRegionsMonotone does not
		// and crashes or wraps on very large maps - prefer partition=watershed
		// there.
		INFO("Regions: %d (after merge/filter; ids are 15-bit)", compact->maxRegions);

		DEBUG1("Simplifying region contours ...");
		rcContourSet * contours = rcAllocContourSet();
		if (!contours)
		{
			FAULT("Failed to allocate contour set");
			return EXIT_BUILD_FAILED;
		}

		if (!rcBuildContours(&ctx, *compact, config.maxSimplificationError, config.maxEdgeLen, *contours))
		{
			FAULT("Could not create contours");
			return EXIT_BUILD_FAILED;
		}
		
		// rcBuildPolyMesh caps the *sum* of contour vertices (before it
		// de-duplicates them) at 0xfffe, so print the number it will test.
		int contourVerts = 0;
		for (int i = 0; i < contours->nconts; i++)
		{
			if (contours->conts[i].nverts >= 3)
				contourVerts += contours->conts[i].nverts;
		}
		INFO("Contours: %d with %d vertices (cap 65534)", contours->nconts, contourVerts);

		DEBUG1("Building polygon mesh ...");
		rcPolyMesh * polyMesh = rcAllocPolyMesh();
		if (!polyMesh)
		{
			FAULT("Failed to allocate polygon mesh");
			return EXIT_BUILD_FAILED;
		}
		if (!rcBuildPolyMesh(&ctx, *contours, config.maxVertsPerPoly, *polyMesh))
		{
			FAULT("Could not triangulate contours (a single Recast poly mesh is capped at 0xfffe = 65534 vertices; see the Recast line above)");
			return EXIT_BUILD_FAILED;
		}

		// rcBuildPolyMesh's buildMeshAdjacency() stores edge indices in
		// unsigned shorts (RecastMesh.cpp: `firstEdge[v0] = (unsigned short)edgeCount`)
		// with no overflow check. Past 0xffff edges the neighbour links are
		// silently corrupted: the mesh still saves and loads, but falls apart
		// into thousands of disconnected fragments. Count edges exactly the way
		// Recast does and refuse to write such a mesh.
		unsigned int adjacencyEdges = 0;
		for (int i = 0; i < polyMesh->npolys; i++)
		{
			const unsigned short * poly = &polyMesh->polys[i * polyMesh->nvp * 2];
			for (int j = 0; j < polyMesh->nvp; j++)
			{
				if (poly[j] == RC_MESH_NULL_IDX)
					break;
				unsigned short next = (j + 1 >= polyMesh->nvp || poly[j + 1] == RC_MESH_NULL_IDX) ? poly[0] : poly[j + 1];
				if (poly[j] < next)
					adjacencyEdges++;
			}
		}
		INFO("Poly mesh: nverts=%d npolys=%d adjacencyEdges=%u (caps: 65534 verts, 65535 edges; edges ~ nverts + npolys)",
			polyMesh->nverts, polyMesh->npolys, adjacencyEdges);

		// An empty poly mesh is a failed build, not a successful empty one.
		// Recast reports nothing when every region is filtered away or the
		// compact heightfield was corrupt, so exiting 0 with a 60-byte .nav
		// hands the caller a file that loads, has zero polygons, and makes
		// every NPC fall back to straight-line pathing.
		if (polyMesh->npolys == 0)
		{
			FAULT("Poly mesh is EMPTY (nverts=0 npolys=0): no walkable surface survived. Check the OBJ axis "
				"order and winding, the bounds= crop, and the span/edge caps logged above");
			return EXIT_BUILD_FAILED;
		}

		if (adjacencyEdges > 0xffff)
		{
			FAULT("Poly mesh has %u adjacency edges; Recast indexes them with 16 bits (max 65535), so polygon "
				"connectivity is corrupt. Reduce detail (maxSimplificationError, minRegionSize) or crop with bounds=",
				adjacencyEdges);
			return EXIT_BUILD_FAILED;
		}

		DEBUG1("Building detail mesh ...");
		rcPolyMeshDetail * detail = rcAllocPolyMeshDetail();
		if (!detail)
		{
			FAULT("Failed to allocate detailed polygon mesh");
			return EXIT_BUILD_FAILED;
		}

		if (!rcBuildPolyMeshDetail(&ctx, *polyMesh, *compact, config.detailSampleDist, config.detailSampleMaxError, *detail))
		{
			FAULT("Could not build detail mesh");
			return EXIT_BUILD_FAILED;
		}
		ctx.reportSuppressed();

		rcFreeCompactHeightfield(compact);
		rcFreeContourSet(contours);

		// Mark all polygons as walkable
		// (needed as the poly filter won't work if flags is set to zero)
		for (int i = 0; i < polyMesh->npolys; i++)
			polyMesh->flags[i] = 0x01;
		
		DEBUG1("Exporting mesh ...");
		std::ofstream navfile;
		navfile.open(navmeshFile.c_str(), std::ofstream::out | std::ofstream::binary);
		if (navfile.fail())
		{
			FAULT("Failed to open file '%s' for export", navmeshFile.c_str());
			return EXIT_OUTPUT_NOT_WRITABLE;
		}
		xrcSavePolyMesh(*polyMesh, *detail, agentHeight, agentClimb, agentRadius, navfile);
		navfile.flush();
		if (navfile.fail())
		{
			FAULT("Failed to write navmesh to '%s'", navmeshFile.c_str());
			return EXIT_OUTPUT_NOT_WRITABLE;
		}

		INFO("Navmesh: nverts=%d npolys=%d edges=%u (caps: 65534 verts, 65535 edges) detailVerts=%d detailTris=%d",
			polyMesh->nverts, polyMesh->npolys, adjacencyEdges, detail->nverts, detail->ntris);

		rcFreePolyMeshDetail(detail);
		rcFreePolyMesh(polyMesh);
		return EXIT_OK;
	}

	void addChunks(std::string const & dir)
	{
		WIN32_FIND_DATAA findData;
		std::string pattern = dir + "/*.*";
		HANDLE h = FindFirstFileA(pattern.c_str(), &findData);
		if (h == INVALID_HANDLE_VALUE || h == (HANDLE)0xffffffff)
		{
			FAULT("Failed to add chunks to mesh: failed to enumerate directory '%s': %d!", 
				dir.c_str(), GetLastError());
			throw std::runtime_error("Failed to add chunks to mesh!");
		}

		do
		{
			std::string fname = findData.cFileName;
			if (fname.length() > 12 && fname.substr(fname.length() - 4) == ".obj")
			{
				std::string baseName = dir + "/" + fname.substr(0, fname.length() - 4);
				addChunk(baseName);
			}
		} while (FindNextFileA(h, &findData) == TRUE);
	}

	void addChunk(std::string const & path)
	{
		DEBUG1("Loading chunk: %s", path.c_str());
		MapChunk * chunk = new MapChunk;
		chunk->load(path);
		chunks_.push_back(chunk);
	}

	unsigned int numChunks() const
	{
		return (unsigned int)chunks_.size();
	}

private:
	std::vector<MapChunk *> chunks_;
};

int guardedMain(int argc, char ** argv)
{
	if (argc < 5)
	{
		std::cout << BuildParams::usage();
		return EXIT_USAGE;
	}

	BuildParams params;
	try
	{
		params.parse(argc, argv, 5);
	}
	catch (std::exception & e)
	{
		FAULT("%s", e.what());
		std::cout << BuildParams::usage();
		return EXIT_USAGE;
	}

	MapExporter exporter;
	if (std::string(argv[1]) == "chunked")
		exporter.addChunks(argv[2]);
	else if (std::string(argv[1]) == "whole")
		exporter.addChunk(argv[2]);
	else
	{
		FAULT("Invalid navmesh builder mode: %s", argv[1]);
		return EXIT_USAGE;
	}

	if (exporter.numChunks() == 0)
	{
		FAULT("No chunk OBJs found in '%s'", argv[2]);
		return EXIT_BUILD_FAILED;
	}

	if (argv[4] == std::string("nav"))
		return exporter.exportNavmesh(argv[3], params);
	else if (argv[4] == std::string("obj"))
		exporter.exportMesh(argv[3]);
	else
	{
		FAULT("Invalid navmesh export format: %s", argv[4]);
		return EXIT_USAGE;
	}

	return 0;
}

int main(int argc, char ** argv)
{
	Logger::initialize();
	int exitCode;
	try
	{
		exitCode = guardedMain(argc, argv);
	}
	catch (std::exception & e)
	{
		FAULT("Internal error: %s", e.what());
		exitCode = EXIT_INTERNAL_ERROR;
	}

	Logger::shutdown();
	return exitCode;
}

