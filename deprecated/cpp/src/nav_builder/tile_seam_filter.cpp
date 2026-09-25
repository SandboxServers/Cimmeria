#include "stdafx.hpp"

#include <map>

#include "tile_seam_filter.hpp"

namespace
{
	// Detour's same-line tolerance and slab end shrink (DetourNavMesh.cpp:
	// findConnectingPolys, overlapSlabs), both 0.01.
	const float SLAB_EPS = 0.01f;

	struct PortalEdge
	{
		unsigned int poly;
		float a[3];
		float b[3];
	};

	struct UnionFind
	{
		std::vector<unsigned int> parent;

		explicit UnionFind(size_t n) : parent(n)
		{
			for (size_t i = 0; i < n; i++)
				parent[i] = (unsigned int)i;
		}

		unsigned int find(unsigned int x)
		{
			while (parent[x] != x)
			{
				parent[x] = parent[parent[x]];
				x = parent[x];
			}
			return x;
		}

		void join(unsigned int a, unsigned int b)
		{
			a = find(a);
			b = find(b);
			if (a != b)
				parent[a < b ? b : a] = a < b ? a : b;
		}
	};

	// (u, y) at both ends of an edge on tile side `dir`, sorted by u, where u
	// runs along the edge; `line` is the coordinate constant along it.
	void slab(PortalEdge const & e, int dir, float & line, float mn[2], float mx[2])
	{
		float ua, ub;
		if (dir == 0 || dir == 2)
		{
			line = e.a[0];
			ua = e.a[2];
			ub = e.b[2];
		}
		else
		{
			line = e.a[2];
			ua = e.a[0];
			ub = e.b[0];
		}
		if (ua <= ub)
		{
			mn[0] = ua; mn[1] = e.a[1];
			mx[0] = ub; mx[1] = e.b[1];
		}
		else
		{
			mn[0] = ub; mn[1] = e.b[1];
			mx[0] = ua; mx[1] = e.a[1];
		}
	}

	// Detour's overlapSlabs with px = SLAB_EPS, py = walkableClimb.
	bool overlapSlabs(const float amin[2], const float amax[2], const float bmin[2], const float bmax[2], float py)
	{
		const float minx = (amin[0] + SLAB_EPS) > (bmin[0] + SLAB_EPS) ? (amin[0] + SLAB_EPS) : (bmin[0] + SLAB_EPS);
		const float maxx = (amax[0] - SLAB_EPS) < (bmax[0] - SLAB_EPS) ? (amax[0] - SLAB_EPS) : (bmax[0] - SLAB_EPS);
		if (minx > maxx)
			return false;
		const float ad = (amax[1] - amin[1]) / (amax[0] - amin[0]);
		const float ak = amin[1] - ad * amin[0];
		const float bd = (bmax[1] - bmin[1]) / (bmax[0] - bmin[0]);
		const float bk = bmin[1] - bd * bmin[0];
		const float dmin = (bd * minx + bk) - (ad * minx + ak);
		const float dmax = (bd * maxx + bk) - (ad * maxx + ak);
		if (dmin * dmax < 0)
			return true;
		const float thr = (py * 2) * (py * 2);
		return dmin * dmin <= thr || dmax * dmax <= thr;
	}

	long long sideKey(int tx, int ty, int dir)
	{
		return ((long long)(tx + 0x100000) << 24) ^ ((long long)(ty + 0x100000) << 2) ^ (long long)dir;
	}

	void worldVertex(rcPolyMesh const & m, unsigned short v, float out[3])
	{
		out[0] = m.bmin[0] + m.verts[v * 3] * m.cs;
		out[1] = m.bmin[1] + m.verts[v * 3 + 1] * m.ch;
		out[2] = m.bmin[2] + m.verts[v * 3 + 2] * m.cs;
	}

	int polyVertCount(rcPolyMesh const & m, int p)
	{
		const unsigned short * poly = &m.polys[p * m.nvp * 2];
		int n = 0;
		while (n < m.nvp && poly[n] != RC_MESH_NULL_IDX)
			n++;
		return n;
	}

	// Drops every polygon with keep[p] == false and compacts the vertex and
	// detail arrays behind it.
	void compactTile(XrcTile & tile, std::vector<bool> const & keep)
	{
		rcPolyMesh & m = *tile.mesh;
		rcPolyMeshDetail & d = *tile.detail;
		const int nvp = m.nvp;

		std::vector<int> polyMap(m.npolys, -1);
		std::vector<int> vertMap(m.nverts, -1);
		int kept = 0;
		for (int p = 0; p < m.npolys; p++)
		{
			if (!keep[p])
				continue;
			polyMap[p] = kept++;
			const int n = polyVertCount(m, p);
			for (int j = 0; j < n; j++)
				vertMap[m.polys[p * nvp * 2 + j]] = 0;
		}

		int nverts = 0;
		for (int v = 0; v < m.nverts; v++)
		{
			if (vertMap[v] < 0)
				continue;
			vertMap[v] = nverts;
			memmove(&m.verts[nverts * 3], &m.verts[v * 3], 3 * sizeof(unsigned short));
			nverts++;
		}

		// Detail sub-meshes are appended in polygon order, so every copy
		// below moves data towards the front: memmove in place is safe.
		unsigned int dverts = 0, dtris = 0;
		for (int p = 0; p < m.npolys; p++)
		{
			const int q = polyMap[p];
			if (q < 0)
				continue;
			unsigned short * dst = &m.polys[q * nvp * 2];
			const unsigned short * src = &m.polys[p * nvp * 2];
			unsigned short row[2 * 12];
			for (int j = 0; j < nvp; j++)
			{
				row[j] = src[j] == RC_MESH_NULL_IDX ? RC_MESH_NULL_IDX : (unsigned short)vertMap[src[j]];
				const unsigned short nei = src[nvp + j];
				if (nei == RC_MESH_NULL_IDX || (nei & 0x8000))
					row[nvp + j] = nei;
				else
					row[nvp + j] = polyMap[nei] < 0 ? RC_MESH_NULL_IDX : (unsigned short)polyMap[nei];
			}
			memcpy(dst, row, 2 * nvp * sizeof(unsigned short));
			m.regs[q] = m.regs[p];
			m.flags[q] = m.flags[p];
			m.areas[q] = m.areas[p];

			const unsigned int vbase = d.meshes[p * 4], nv = d.meshes[p * 4 + 1];
			const unsigned int tbase = d.meshes[p * 4 + 2], nt = d.meshes[p * 4 + 3];
			memmove(&d.verts[dverts * 3], &d.verts[vbase * 3], nv * 3 * sizeof(float));
			memmove(&d.tris[dtris * 4], &d.tris[tbase * 4], nt * 4);
			d.meshes[q * 4] = dverts;
			d.meshes[q * 4 + 1] = nv;
			d.meshes[q * 4 + 2] = dtris;
			d.meshes[q * 4 + 3] = nt;
			dverts += nv;
			dtris += nt;
		}

		m.npolys = kept;
		m.nverts = nverts;
		d.nmeshes = kept;
		d.nverts = (int)dverts;
		d.ntris = (int)dtris;
	}
}

SeamFilterStats filterSeamFragments(std::vector<XrcTile> & tiles, float minArea, float climb)
{
	SeamFilterStats stats;
	memset(&stats, 0, sizeof(stats));

	std::vector<unsigned int> base(tiles.size() + 1, 0);
	for (size_t t = 0; t < tiles.size(); t++)
		base[t + 1] = base[t] + (unsigned int)tiles[t].mesh->npolys;
	const unsigned int total = base[tiles.size()];
	UnionFind uf(total);

	// In-tile links, and the portal edges on each tile side.
	std::map<long long, std::vector<PortalEdge> > portals;
	for (size_t t = 0; t < tiles.size(); t++)
	{
		rcPolyMesh const & m = *tiles[t].mesh;
		const int nvp = m.nvp;
		for (int p = 0; p < m.npolys; p++)
		{
			const unsigned short * poly = &m.polys[p * nvp * 2];
			const int n = polyVertCount(m, p);
			for (int j = 0; j < n; j++)
			{
				const unsigned short nei = poly[nvp + j];
				if (nei == RC_MESH_NULL_IDX)
					continue;
				if (!(nei & 0x8000))
				{
					uf.join(base[t] + p, base[t] + nei);
					continue;
				}
				const int dir = nei & 0xf;
				if (dir > 3)
					continue;
				PortalEdge e;
				e.poly = base[t] + p;
				worldVertex(m, poly[j], e.a);
				worldVertex(m, poly[(j + 1) % n], e.b);
				portals[sideKey(tiles[t].tileX, tiles[t].tileY, dir)].push_back(e);
			}
		}
	}

	// Cross-tile links, exactly as Detour will make them.
	const int facingDir[4] = { 2, 3, 0, 1 };
	const int facingDx[4] = { -1, 0, 1, 0 };
	const int facingDy[4] = { 0, 1, 0, -1 };
	for (size_t t = 0; t < tiles.size(); t++)
	{
		for (int dir = 0; dir < 4; dir++)
		{
			auto mine = portals.find(sideKey(tiles[t].tileX, tiles[t].tileY, dir));
			if (mine == portals.end())
				continue;
			auto theirs = portals.find(sideKey(tiles[t].tileX + facingDx[dir], tiles[t].tileY + facingDy[dir], facingDir[dir]));
			if (theirs == portals.end())
				continue;
			for (auto a = mine->second.begin(); a != mine->second.end(); ++a)
			{
				float la, amin[2], amax[2];
				slab(*a, dir, la, amin, amax);
				for (auto b = theirs->second.begin(); b != theirs->second.end(); ++b)
				{
					float lb, bmin[2], bmax[2];
					slab(*b, facingDir[dir], lb, bmin, bmax);
					if (fabsf(la - lb) > SLAB_EPS)
						continue;
					if (overlapSlabs(amin, amax, bmin, bmax, climb))
						uf.join(a->poly, b->poly);
				}
			}
		}
	}

	// Area per component, measured the way rcBuildRegions measures it: the
	// compact spans of every region the component's polygons came from, so
	// a thin walkway that contour simplification narrows is still counted
	// at its voxel width. A tile without span counts (none today) falls
	// back to polygon area.
	std::vector<double> area(total, 0.0);
	for (size_t t = 0; t < tiles.size(); t++)
	{
		rcPolyMesh const & m = *tiles[t].mesh;
		std::vector<int> const & spans = tiles[t].regionSpans;
		if (!spans.empty())
		{
			std::vector<bool> counted(spans.size(), false);
			for (int p = 0; p < m.npolys; p++)
			{
				const unsigned short reg = m.regs[p];
				if (reg >= spans.size() || counted[reg])
					continue;
				counted[reg] = true;
				area[uf.find(base[t] + p)] += (double)spans[reg] * m.cs * m.cs;
			}
			continue;
		}
		for (int p = 0; p < m.npolys; p++)
		{
			const unsigned short * poly = &m.polys[p * m.nvp * 2];
			const int n = polyVertCount(m, p);
			double twice = 0.0;
			for (int j = 0; j < n; j++)
			{
				const unsigned short * u = &m.verts[poly[j] * 3];
				const unsigned short * v = &m.verts[poly[(j + 1) % n] * 3];
				twice += (double)u[0] * v[2] - (double)v[0] * u[2];
			}
			area[uf.find(base[t] + p)] += fabs(twice) * 0.5 * m.cs * m.cs;
		}
	}
	for (unsigned int i = 0; i < total; i++)
	{
		if (uf.find(i) != i)
			continue;
		stats.components++;
		if (area[i] < minArea)
			stats.removedComponents++;
	}

	std::vector<XrcTile> keptTiles;
	for (size_t t = 0; t < tiles.size(); t++)
	{
		rcPolyMesh const & m = *tiles[t].mesh;
		std::vector<bool> keep(m.npolys, true);
		int removed = 0;
		for (int p = 0; p < m.npolys; p++)
		{
			if (area[uf.find(base[t] + p)] < minArea)
			{
				keep[p] = false;
				removed++;
			}
		}
		stats.removedPolys += removed;
		if (removed == m.npolys)
		{
			rcFreePolyMesh(tiles[t].mesh);
			rcFreePolyMeshDetail(tiles[t].detail);
			stats.emptiedTiles++;
			continue;
		}
		if (removed > 0)
			compactTile(tiles[t], keep);
		keptTiles.push_back(tiles[t]);
	}
	tiles.swap(keptTiles);
	return stats;
}
