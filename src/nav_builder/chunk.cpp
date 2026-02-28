#include "stdafx.hpp"
#include "chunk.hpp"
#include "mesh.hpp"
#include "mesh_exporter.hpp"

MapChunk::MapChunk()
	: geometry_(nullptr)
{
}

MapChunk::~MapChunk()
{
	delete geometry_;
}

void MapChunk::load(std::string const & path)
{
	chunkId_ = 0;
	if (*path.rbegin() == 'o' && path.length() > 9)
	{
		for (size_t i = path.length() - 9; i < path.length() - 1; i++)
		{
			chunkId_ <<= 4;
			chunkId_ |= htoi(path[i]);
		}
		
		positionX_ = static_cast<int16_t>(chunkId_);
		positionZ_ = static_cast<int16_t>(chunkId_ >> 16);
		sizeX_ = sizeZ_ = 100;
	}
	else
		FAULT("Failed to determine chunk ID from filename: %s; assuming ID 0", path.c_str());

	std::string geometryPath = path + ".obj";
	loadGeometry(geometryPath);
	path_ = path;
}

void MapChunk::exportVertices(MeshExporter & exporter)
{
	exporter.addComment(std::string(" ===== CHUNK: ") + path_ + " =====");

	if (geometry_)
	{
		matrix<float> transMatrix(4, 4);
		for (unsigned int x = 0; x < 4; x++)
			for (unsigned int y = 0; y < 4; y++)
				transMatrix(x, y) = 0.0f;
		for (unsigned int x = 0; x < 4; x++)
			transMatrix(x, x) = 1.0f;

		/*float angle = -3.14159265359f / 2;
		transMatrix(0, 0) = cos(-angle);
		transMatrix(0, 2) = sin(-angle);
		transMatrix(2, 0) = -sin(-angle);
		transMatrix(2, 2) = cos(-angle);*/
		geometry_->exportMesh(exporter, transMatrix, 0.0f, 0.0f);
	}
}
	
float MapChunk::positionX() const
{
	return positionX_;
}

float MapChunk::positionZ() const
{
	return positionZ_;
}

float MapChunk::sizeX() const
{
	return sizeX_;
}

float MapChunk::sizeZ() const
{
	return sizeZ_;
}

unsigned int MapChunk::numVertices() const
{
	unsigned int vertices = 0;
	if (geometry_)
		vertices += geometry_->numVertices();

	return vertices;
}

unsigned int MapChunk::numFaces() const
{
	unsigned int faces = 0;
	if (geometry_)
		faces += geometry_->numFaces();

	return faces;
}

bool MapChunk::loadGeometry(std::string const & path)
{
	std::ifstream f(path.c_str(), std::ifstream::binary | std::ifstream::in);
	if (f.fail())
	{
		WARN("Failed to open chunk geometry file '%s'", path.c_str());
		return false;
	}
	f.close();

	Mesh * geom = new Mesh();
	if (!geom->loadOBJ(path))
	{
		delete geom;
		return false;
	}
	else
	{
		geometry_ = geom;
		return true;
	}
}

