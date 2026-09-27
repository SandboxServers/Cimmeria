#include "stdafx.hpp"

#include "xrc_writer.hpp"

namespace
{
	void savePolyMeshBlock(rcPolyMesh & mesh, rcPolyMeshDetail & detail, std::ostream & stream)
	{
		uint32_t vertices = mesh.nverts, polys = mesh.npolys, nvp = mesh.nvp,
			borderSize = mesh.borderSize;
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

	void saveAgent(float agentHeight, float agentClimb, float agentRadius, std::ostream & stream)
	{
		stream.write((char *)&agentHeight, sizeof(agentHeight));
		stream.write((char *)&agentClimb, sizeof(agentClimb));
		stream.write((char *)&agentRadius, sizeof(agentRadius));
	}
}

void xrcSavePolyMesh(rcPolyMesh & mesh, rcPolyMeshDetail & detail, float agentHeight, float agentClimb, float agentRadius, std::ostream & stream)
{
	saveAgent(agentHeight, agentClimb, agentRadius, stream);
	savePolyMeshBlock(mesh, detail, stream);
}

void xrcSaveTiledMesh(std::vector<XrcTile> const & tiles, float agentHeight, float agentClimb, float agentRadius,
	const float orig[3], float tileWidth, float tileHeight, unsigned int maxTilePolys, std::ostream & stream)
{
	const char magic[4] = { 'X', 'R', 'C', 'T' };
	uint32_t version = 1, ntiles = (uint32_t)tiles.size(), maxPolys = maxTilePolys;
	stream.write(magic, sizeof(magic));
	stream.write((char *)&version, sizeof(version));
	saveAgent(agentHeight, agentClimb, agentRadius, stream);
	stream.write((char *)orig, 3 * sizeof(float));
	stream.write((char *)&tileWidth, sizeof(tileWidth));
	stream.write((char *)&tileHeight, sizeof(tileHeight));
	stream.write((char *)&ntiles, sizeof(ntiles));
	stream.write((char *)&maxPolys, sizeof(maxPolys));
	for (auto it = tiles.begin(); it != tiles.end(); ++it)
	{
		int32_t tx = it->tileX, ty = it->tileY;
		stream.write((char *)&tx, sizeof(tx));
		stream.write((char *)&ty, sizeof(ty));
		savePolyMeshBlock(*it->mesh, *it->detail, stream);
	}
}
