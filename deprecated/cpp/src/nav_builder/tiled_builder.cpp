#include "stdafx.hpp"

#include <atomic>
#include <thread>

#include "tiled_builder.hpp"
#include "recast_pipeline.hpp"
#include "tile_seam_filter.hpp"
#include "xrc_writer.hpp"

namespace
{
	struct TileJob
	{
		PipelineResult result;
		rcPolyMesh * mesh;
		rcPolyMeshDetail * detail;
		PipelineStats stats;
		int ntris;
		unsigned int danglingFaces;
		std::vector<int> regionSpans;

		TileJob() : result(PIPELINE_EMPTY), mesh(0), detail(0), ntris(0), danglingFaces(0)
		{
			memset(&stats, 0, sizeof(stats));
		}
	};

	// Tile index range [lo, hi] a coordinate span touches, border included,
	// clamped in float first so a far-off vertex cannot overflow the cast.
	void tileRange(float lo, float hi, float origin, float tileMetres, float border, int count, int & first, int & last)
	{
		float a = floorf((lo - origin - border) / tileMetres);
		float b = floorf((hi - origin + border) / tileMetres);
		if (a < -1.0f) a = -1.0f;
		if (a > (float)count) a = (float)count;
		if (b < -1.0f) b = -1.0f;
		if (b > (float)count) b = (float)count;
		first = (int)a < 0 ? 0 : (int)a;
		last = (int)b >= count ? count - 1 : (int)b;
	}

	unsigned int ilog2NextPow2(unsigned int v)
	{
		unsigned int p = 1, bits = 0;
		while (p < v)
		{
			p <<= 1;
			bits++;
		}
		return bits;
	}
}

int buildTiledNavmesh(rcConfig const & config, BuildParams const & params,
	const float * verts, int nverts, const unsigned int * faces, int nfaces,
	std::string const & navmeshFile)
{
	const int ts = params.tileSize;
	const int tw = (config.width + ts - 1) / ts;
	const int th = (config.height + ts - 1) / ts;
	const float tileMetres = ts * config.cs;

	// RecastDemo's Sample_TileMesh border: enough for the agent-radius
	// erosion plus the filters to see past the tile edge, so the two sides
	// of a tile seam agree.
	rcConfig tileConfig = config;
	tileConfig.tileSize = ts;
	tileConfig.borderSize = config.walkableRadius + 3;
	tileConfig.width = ts + tileConfig.borderSize * 2;
	tileConfig.height = ts + tileConfig.borderSize * 2;
	const float border = tileConfig.borderSize * config.cs;

	const int tileCount = tw * th;
	int threads = params.threads < tileCount ? params.threads : tileCount;
	if (threads < 1)
		threads = 1;
	INFO("Tiles: %d x %d of %d cells (%.2f m), border %d cells, %d worker threads",
		tw, th, ts, tileMetres, tileConfig.borderSize, threads);

	std::vector<unsigned char> areas(nfaces, 0);
	{
		LoggingContext markCtx;
		rcMarkWalkableTriangles(&markCtx, config.walkableSlopeAngle, verts, nverts, (const int *)faces, nfaces, areas.data());
	}

	// Bin every triangle into the tiles its XZ box (plus the border)
	// touches. Unwalkable triangles are kept: they rasterise as solid spans
	// and are what makes a wall a wall.
	std::vector<std::vector<int> > bins(tileCount);
	for (int f = 0; f < nfaces; f++)
	{
		const float * a = &verts[faces[f * 3] * 3];
		const float * b = &verts[faces[f * 3 + 1] * 3];
		const float * c = &verts[faces[f * 3 + 2] * 3];
		float minX = a[0], maxX = a[0], minZ = a[2], maxZ = a[2];
		minX = b[0] < minX ? b[0] : minX; maxX = b[0] > maxX ? b[0] : maxX;
		minX = c[0] < minX ? c[0] : minX; maxX = c[0] > maxX ? c[0] : maxX;
		minZ = b[2] < minZ ? b[2] : minZ; maxZ = b[2] > maxZ ? b[2] : maxZ;
		minZ = c[2] < minZ ? c[2] : minZ; maxZ = c[2] > maxZ ? c[2] : maxZ;
		if (!(minX == minX) || !(minZ == minZ) || !(maxX == maxX) || !(maxZ == maxZ))
			continue;

		int x0, x1, z0, z1;
		tileRange(minX, maxX, config.bmin[0], tileMetres, border, tw, x0, x1);
		tileRange(minZ, maxZ, config.bmin[2], tileMetres, border, th, z0, z1);
		for (int z = z0; z <= z1; z++)
			for (int x = x0; x <= x1; x++)
				bins[z * tw + x].push_back(f);
	}

	std::vector<TileJob> jobs(tileCount);
	std::atomic<int> nextTile(0);
	std::atomic<bool> failed(false);

	auto worker = [&]()
	{
		std::vector<int> tris;
		std::vector<unsigned char> tileAreas;
		for (;;)
		{
			int i = nextTile++;
			if (i >= tileCount || failed)
				break;
			std::vector<int> & bin = bins[i];
			if (bin.empty())
				continue;

			const int tx = i % tw, ty = i / tw;
			char where[64];
			snprintf(where, sizeof(where), "Tile %d,%d: ", tx, ty);

			try
			{
				rcConfig c = tileConfig;
				c.bmin[0] = config.bmin[0] + tx * tileMetres - border;
				c.bmin[2] = config.bmin[2] + ty * tileMetres - border;
				c.bmax[0] = config.bmin[0] + (tx + 1) * tileMetres + border;
				c.bmax[2] = config.bmin[2] + (ty + 1) * tileMetres + border;

				tris.resize(bin.size() * 3);
				tileAreas.resize(bin.size());
				for (size_t k = 0; k < bin.size(); k++)
				{
					const unsigned int f = (unsigned int)bin[k];
					tris[k * 3] = (int)faces[f * 3];
					tris[k * 3 + 1] = (int)faces[f * 3 + 1];
					tris[k * 3 + 2] = (int)faces[f * 3 + 2];
					tileAreas[k] = areas[f];
				}
				// The bin is not needed again; give the memory back while
				// the other workers are still running.
				std::vector<int>().swap(bin);

				LoggingContext ctx(std::string("Recast ") + where, 0);
				RecastInput in = { verts, nverts, tris.data(), tileAreas.data(), (int)tileAreas.size() };
				TileJob & job = jobs[i];
				job.ntris = in.ntris;
				job.result = runRecastPipeline(ctx, c, params, in, where, false, &job.mesh, &job.detail, job.stats,
					&job.regionSpans);
				job.danglingFaces = ctx.danglingFaces();
				if (job.result == PIPELINE_FAILED)
					failed = true;
				else if (job.result == PIPELINE_OK)
					INFO("%s%d tris, %d spans, %d regions, %d contour verts, nverts=%d npolys=%d edges=%u detailVerts=%d detailTris=%d",
						where, job.ntris, job.stats.spans, job.stats.regions, job.stats.contourVerts,
						job.mesh->nverts, job.mesh->npolys, job.stats.adjacencyEdges, job.detail->nverts, job.detail->ntris);
				else
					DEBUG1("%s%d tris, nothing walkable", where, job.ntris);
			}
			catch (std::exception & e)
			{
				FAULT("%sInternal error: %s", where, e.what());
				failed = true;
			}
		}
	};

	std::vector<std::thread> pool;
	for (int t = 1; t < threads; t++)
		pool.push_back(std::thread(worker));
	worker();
	for (auto it = pool.begin(); it != pool.end(); ++it)
		it->join();

	// Owns every mesh from here on: the jobs' until they move into `tiles`,
	// then whatever the seam filter leaves in `tiles`.
	std::vector<XrcTile> tiles;
	struct FreeTiles
	{
		std::vector<TileJob> & jobs;
		std::vector<XrcTile> & tiles;
		~FreeTiles()
		{
			for (auto it = jobs.begin(); it != jobs.end(); ++it)
			{
				rcFreePolyMesh(it->mesh);
				rcFreePolyMeshDetail(it->detail);
			}
			for (auto it = tiles.begin(); it != tiles.end(); ++it)
			{
				rcFreePolyMesh(it->mesh);
				rcFreePolyMeshDetail(it->detail);
			}
		}
	} freeTiles = { jobs, tiles };

	if (failed)
	{
		FAULT("Tiled build failed (see the tile lines above); no navmesh written");
		return EXIT_BUILD_FAILED;
	}

	// Tiles go out in row-major order, so the file does not depend on which
	// worker finished first.
	int noGeometry = 0, emptyTiles = 0;
	unsigned int dangling = 0;
	int maxSpans = 0, maxContourVerts = 0;
	for (int i = 0; i < tileCount; i++)
	{
		TileJob & job = jobs[i];
		dangling += job.danglingFaces;
		if (job.result != PIPELINE_OK)
		{
			if (job.ntris == 0)
				noGeometry++;
			else
				emptyTiles++;
			continue;
		}
		XrcTile tile = { i % tw, i / tw, job.mesh, job.detail };
		tile.regionSpans.swap(job.regionSpans);
		tiles.push_back(tile);
		job.mesh = 0;
		job.detail = 0;
		if (job.stats.spans > maxSpans) maxSpans = job.stats.spans;
		if (job.stats.contourVerts > maxContourVerts) maxContourVerts = job.stats.contourVerts;
	}
	if (dangling > 0)
		WARN("Recast: delaunayHull: %u 'Removing dangling face' warnings across all tiles (not shown)", dangling);

	// The single-mesh region filter's threshold, in square metres.
	const float minArea = params.minRegionSize * params.minRegionSize * config.cs * config.cs;
	const SeamFilterStats seam = filterSeamFragments(tiles, params.seamFilter ? minArea : 0.0f, params.agentClimb);
	INFO("Seam filter: %u components across tiles; removed %u under %.1f m^2 (%u polys, %u tiles emptied)%s",
		seam.components, seam.removedComponents, params.seamFilter ? minArea : 0.0f, seam.removedPolys,
		seam.emptiedTiles, params.seamFilter ? "" : " [seamFilter=0]");

	unsigned int maxTilePolys = 0, totalEdges = 0, maxEdges = 0;
	long long totalVerts = 0, totalPolys = 0, totalDetailVerts = 0, totalDetailTris = 0;
	int maxVerts = 0;
	for (auto it = tiles.begin(); it != tiles.end(); ++it)
	{
		const unsigned int edges = countAdjacencyEdges(*it->mesh);
		totalVerts += it->mesh->nverts;
		totalPolys += it->mesh->npolys;
		totalDetailVerts += it->detail->nverts;
		totalDetailTris += it->detail->ntris;
		totalEdges += edges;
		if ((unsigned int)it->mesh->npolys > maxTilePolys) maxTilePolys = it->mesh->npolys;
		if (it->mesh->nverts > maxVerts) maxVerts = it->mesh->nverts;
		if (edges > maxEdges) maxEdges = edges;
	}

	if (tiles.empty())
	{
		FAULT("Poly mesh is EMPTY: no tile kept a walkable polygon. Check the OBJ axis "
			"order and winding, the bounds= crop, and the tile lines above");
		return EXIT_BUILD_FAILED;
	}

	// A 32-bit dtPolyRef is salt | tile | poly, and dtNavMesh::init refuses
	// fewer than 10 salt bits, so tile and poly indices share 22 bits.
	const unsigned int tileBits = ilog2NextPow2((unsigned int)tiles.size());
	const unsigned int polyBits = ilog2NextPow2(maxTilePolys);
	INFO("Poly refs: %u tiles -> %u tile bits, largest tile %u polys -> %u poly bits (tile + poly bits must be <= 22)",
		(unsigned int)tiles.size(), tileBits, maxTilePolys, polyBits);
	if (tileBits + polyBits > 22)
	{
		FAULT("Tile bits %u + poly bits %u exceed the 22 a 32-bit dtPolyRef leaves after its 10 salt bits; "
			"the server could not load this mesh. Change tile= (fewer, larger tiles if tile bits dominate)",
			tileBits, polyBits);
		return EXIT_BUILD_FAILED;
	}

	DEBUG1("Exporting mesh ...");
	std::ofstream navfile;
	navfile.open(navmeshFile.c_str(), std::ofstream::out | std::ofstream::binary);
	if (navfile.fail())
	{
		FAULT("Failed to open file '%s' for export", navmeshFile.c_str());
		return EXIT_OUTPUT_NOT_WRITABLE;
	}
	float orig[3] = { config.bmin[0], config.bmin[1], config.bmin[2] };
	xrcSaveTiledMesh(tiles, params.agentHeight, params.agentClimb, params.agentRadius,
		orig, tileMetres, tileMetres, maxTilePolys, navfile);
	navfile.flush();
	if (navfile.fail())
	{
		FAULT("Failed to write navmesh to '%s'", navmeshFile.c_str());
		return EXIT_OUTPUT_NOT_WRITABLE;
	}

	INFO("Navmesh: tiles=%u of %d x %d (%d without geometry, %d with nothing walkable) nverts=%lld npolys=%lld "
		"edges=%u detailVerts=%lld detailTris=%lld",
		(unsigned int)tiles.size(), tw, th, noGeometry, emptyTiles, totalVerts, totalPolys, totalEdges,
		totalDetailVerts, totalDetailTris);
	INFO("Largest tile: spans=%d (cap 16777215) contourVerts=%d (cap 65534) nverts=%d npolys=%u edges=%u (cap 65535)",
		maxSpans, maxContourVerts, maxVerts, maxTilePolys, maxEdges);
	return EXIT_OK;
}
