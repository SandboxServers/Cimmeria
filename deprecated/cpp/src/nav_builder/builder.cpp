#include "stdafx.hpp"

#include <windows.h>
#include "mesh.hpp"
#include "mesh_exporter.hpp"
#include "chunk.hpp"
#include "build_params.hpp"
#include "recast_pipeline.hpp"
#include "tiled_builder.hpp"
#include "xrc_writer.hpp"
#include "Recast.h"

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

		// Tiled: one rcPolyMesh per tile, so the Recast index caps apply per
		// tile rather than to the whole map. See tiled_builder.hpp.
		if (params.tileSize > 0)
			return buildTiledNavmesh(config, params, exporter.vertices(), (int)vertices,
				exporter.faces(), (int)faces, navmeshFile);

		LoggingContext ctx;
		uint8_t * triAreas = new uint8_t[faces];
		memset(triAreas, 0, faces);
		rcMarkWalkableTriangles(&ctx, config.walkableSlopeAngle, exporter.vertices(), vertices, (int *)exporter.faces(), faces, triAreas);
		RecastInput in = { exporter.vertices(), (int)vertices, (const int *)exporter.faces(), triAreas, (int)faces };

		rcPolyMesh * polyMesh = 0;
		rcPolyMeshDetail * detail = 0;
		PipelineStats stats;
		PipelineResult result = runRecastPipeline(ctx, config, params, in, "", true, &polyMesh, &detail, stats);
		delete [] triAreas;

		// An empty poly mesh is a failed build, not a successful empty one.
		// Recast reports nothing when every region is filtered away or the
		// compact heightfield was corrupt, so exiting 0 with a 60-byte .nav
		// hands the caller a file that loads, has zero polygons, and makes
		// every NPC fall back to straight-line pathing.
		if (result == PIPELINE_EMPTY)
		{
			FAULT("Poly mesh is EMPTY (nverts=0 npolys=0): no walkable surface survived. Check the OBJ axis "
				"order and winding, the bounds= crop, and the span/edge caps logged above");
			return EXIT_BUILD_FAILED;
		}
		if (result != PIPELINE_OK)
			return EXIT_BUILD_FAILED;

		DEBUG1("Exporting mesh ...");
		std::ofstream navfile;
		navfile.open(navmeshFile.c_str(), std::ofstream::out | std::ofstream::binary);
		if (navfile.fail())
		{
			FAULT("Failed to open file '%s' for export", navmeshFile.c_str());
			rcFreePolyMeshDetail(detail);
			rcFreePolyMesh(polyMesh);
			return EXIT_OUTPUT_NOT_WRITABLE;
		}
		xrcSavePolyMesh(*polyMesh, *detail, agentHeight, agentClimb, agentRadius, navfile);
		navfile.flush();
		if (navfile.fail())
		{
			FAULT("Failed to write navmesh to '%s'", navmeshFile.c_str());
			rcFreePolyMeshDetail(detail);
			rcFreePolyMesh(polyMesh);
			return EXIT_OUTPUT_NOT_WRITABLE;
		}

		INFO("Navmesh: nverts=%d npolys=%d edges=%u (caps: 65534 verts, 65535 edges) detailVerts=%d detailTris=%d",
			polyMesh->nverts, polyMesh->npolys, stats.adjacencyEdges, detail->nverts, detail->ntris);

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

