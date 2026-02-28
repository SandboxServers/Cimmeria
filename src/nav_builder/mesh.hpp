#pragma once

#include <string>
#include <cstdint>
#include <iostream>
#include <boost/numeric/ublas/matrix.hpp>

using namespace boost::numeric::ublas;

void ParseVector(std::string const & str, float * vec, unsigned int length);
unsigned int htoi(char c);

class Mesh
{
public:
	Mesh();

	std::string const & path() const;
	uint32_t chunkId() const;

	unsigned int numVertices() const;
	unsigned int numFaces() const;

	bool loadOBJ(std::string const & path);
	void exportMesh(class MeshExporter & exporter, matrix<float> & transformMat, float offsetX, float offsetZ);

private:
	struct Vertex
	{
		float x, y, z;
	};

	std::vector<Vertex> vertices_;
	std::vector<unsigned int> triIndices_;
	std::string path_;
	uint32_t chunkId_, meshIndex_;

	unsigned int translateFaceIndex(std::string const & index);
};

