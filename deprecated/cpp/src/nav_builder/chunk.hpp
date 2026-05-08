#pragma once

#include "mesh.hpp"

class MapChunk
{
public:
	MapChunk();
	~MapChunk();

	void load(std::string const & path);
	void exportVertices(class MeshExporter & exporter);
	
	float positionX() const;
	float positionZ() const;
	float sizeX() const;
	float sizeZ() const;

	unsigned int numVertices() const;
	unsigned int numFaces() const;

private:
	float positionX_, positionZ_;
	float sizeX_, sizeZ_;
	std::string path_;
	unsigned int chunkId_;
	Mesh * geometry_;

	bool loadGeometry(std::string const & path);
};
