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
		config.walkableHeight = (int)ceilf(agentHeight / config.ch);
		config.walkableClimb = (int)floorf(agentClimb / config.ch);
		config.walkableRadius = (int)ceilf(agentRadius / config.cs);
		config.maxEdgeLen = (int)(params.maxEdgeLen / config.cs);
		config.maxSimplificationError = params.maxSimplificationError;
		config.minRegionArea = (int)rcSqr(params.minRegionSize);
		config.mergeRegionArea = (int)rcSqr(params.mergeRegionSize);
		config.maxVertsPerPoly = params.maxVertsPerPoly;
		config.detailSampleDist = params.detailSampleDist < 0.9f ? 0.0f : config.cs * params.detailSampleDist;
		config.detailSampleMaxError = config.ch * params.detailSampleMaxError;

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
		
		INFO("Contours: %d", contours->nconts);

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

		if (polyMesh->npolys == 0)
			WARN("Navmesh is EMPTY - no walkable surface survived (wrong OBJ axis order or winding?)");
		INFO("Navmesh: nverts=%d npolys=%d (cap 65534) detailVerts=%d detailTris=%d",
			polyMesh->nverts, polyMesh->npolys, detail->nverts, detail->ntris);

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

