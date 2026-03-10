#include "stdafx.hpp"

#include <filesystem>
#include "mesh.hpp"
#include "mesh_exporter.hpp"
#include "chunk.hpp"
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

	void exportNavmesh(std::string const & navmeshFile)
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
		
		float agentHeight = 0.6f, agentClimb = 0.9f, agentRadius = 0.6f;
		rcConfig config;
		config.cs = 0.3f;
		config.ch = 0.2f;
		config.walkableSlopeAngle = 45.0f;
		config.walkableHeight = (int)ceilf(agentHeight / config.ch);
		config.walkableClimb = (int)floorf(agentClimb / config.ch);
		config.walkableRadius = (int)ceilf(agentRadius / config.cs);
		config.maxEdgeLen = (int)(12.0f / config.cs);
		config.maxSimplificationError = 1.3f;
		config.minRegionArea = (int)rcSqr(8);
		config.mergeRegionArea = (int)rcSqr(20);
		config.maxVertsPerPoly = (int)6;
		config.detailSampleDist = config.cs * 6.0f;
		config.detailSampleMaxError = config.ch;

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
		
		rcCalcGridSize(config.bmin, config.bmax, config.cs, &config.width, &config.height);
		
		DEBUG1("Rasterizing triangles ...");
		rcContext ctx;
		rcHeightfield * heightfield = rcAllocHeightfield();
		if (!rcCreateHeightfield(&ctx, *heightfield, config.width, config.height, config.bmin, config.bmax, config.cs, config.ch))
		{
			FAULT("Failed to create heightfield");
			return;
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
			return;
		}
		if (!rcBuildCompactHeightfield(&ctx, config.walkableHeight, config.walkableClimb, *heightfield, *compact))
		{
			FAULT("Failed to compact heightfield");
			return;
		}
	
		rcFreeHeightField(heightfield);
		
		// Erode the walkable area by agent radius.
		if (!rcErodeWalkableArea(&ctx, config.walkableRadius, *compact))
		{
			FAULT("Failed to erode walkable area");
			return;
		}
		
		if (true /* monotonePartitioning */)
		{
			// Partition the walkable surface into simple regions without holes.
			// Monotone partitioning does not need distancefield.
			if (!rcBuildRegionsMonotone(&ctx, *compact, 0, config.minRegionArea, config.mergeRegionArea))
			{
				FAULT("Failed to build monotone regions");
				return;
			}
		}
		else
		{
			// Prepare for region partitioning, by calculating distance field along the walkable surface.
			if (!rcBuildDistanceField(&ctx, *compact))
			{
				FAULT("Failed to build distance field");
				return;
			}

			// Partition the walkable surface into simple regions without holes.
			if (!rcBuildRegions(&ctx, *compact, 0, config.minRegionArea, config.mergeRegionArea))
			{
				FAULT("Failed to build regions");
				return;
			}
		}
		
		DEBUG1("Simplifying region contours ...");
		rcContourSet * contours = rcAllocContourSet();
		if (!contours)
		{
			FAULT("Failed to allocate contour set");
			return;
		}

		if (!rcBuildContours(&ctx, *compact, config.maxSimplificationError, config.maxEdgeLen, *contours))
		{
			FAULT("Could not create contours");
			return;
		}
		
		DEBUG1("Building polygon mesh ...");
		rcPolyMesh * polyMesh = rcAllocPolyMesh();
		if (!polyMesh)
		{
			FAULT("Failed to allocate polygon mesh");
			return;
		}
		if (!rcBuildPolyMesh(&ctx, *contours, config.maxVertsPerPoly, *polyMesh))
		{
			FAULT("Could not triangulate contours");
			return;
		}

		DEBUG1("Building detail mesh ...");
		rcPolyMeshDetail * detail = rcAllocPolyMeshDetail();
		if (!detail)
		{
			FAULT("Failed to allocate detailed polygon mesh");
			return;
		}

		if (!rcBuildPolyMeshDetail(&ctx, *polyMesh, *compact, config.detailSampleDist, config.detailSampleMaxError, *detail))
		{
			FAULT("Could not build detail mesh");
			return;
		}

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
			return;
		}
		xrcSavePolyMesh(*polyMesh, *detail, agentHeight, agentClimb, agentRadius, navfile);

		rcFreePolyMeshDetail(detail);
		rcFreePolyMesh(polyMesh);
	}

	void addChunks(std::string const & dir)
	{
		namespace fs = std::filesystem;

		if (!fs::is_directory(dir))
		{
			FAULT("Failed to add chunks to mesh: failed to enumerate directory '%s'!", dir.c_str());
			throw std::runtime_error("Failed to add chunks to mesh!");
		}

		for (auto const & entry : fs::directory_iterator(dir))
		{
			std::string fname = entry.path().filename().string();
			if (fname.length() > 12 && fname.substr(fname.length() - 4) == ".obj")
			{
				std::string baseName = dir + "/" + fname.substr(0, fname.length() - 4);
				addChunk(baseName);
			}
		}
	}

	void addChunk(std::string const & path)
	{
		DEBUG1("Loading chunk: %s", path.c_str());
		MapChunk * chunk = new MapChunk;
		chunk->load(path);
		chunks_.push_back(chunk);
	}

private:
	std::vector<MapChunk *> chunks_;
};

int guardedMain(int argc, char ** argv)
{
	if (argc != 5)
	{
		std::cout << "Usage: NavBuilder <type> <space path> <destination obj> <format>" << std::endl;
		return 1;
	}

	MapExporter exporter;
	if (std::string(argv[1]) == "chunked")
		exporter.addChunks(argv[2]);
	else if (std::string(argv[1]) == "whole")
		exporter.addChunk(argv[2]);
	else
	{
		FAULT("Invalid navmesh builder mode: %s", argv[1]);
		Logger::shutdown();
		return 1;
	}

	if (argv[4] == std::string("nav"))
		exporter.exportNavmesh(argv[3]);
	else if (argv[4] == std::string("obj"))
		exporter.exportMesh(argv[3]);
	else
	{
		FAULT("Invalid navmesh export format: %s", argv[4]);
		Logger::shutdown();
		return 1;
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
		exitCode = 2;
	}

	Logger::shutdown();
	return exitCode;
}

