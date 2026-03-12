#include "stdafx.hpp"
#include "mesh.hpp"
#include "mesh_exporter.hpp"

#include <iostream>
#include <fstream>

void ParseVector(std::string const & str, float * vec, unsigned int length)
{
	std::size_t pos = 0;
	for (unsigned int i = 0; i < length; i++)
	{
		std::size_t beg = str.find_first_not_of("\t ", pos);
		if (beg == std::string::npos)
			throw std::runtime_error("Failed to parse float vector string!");
			
		std::size_t end = str.find_first_not_of("-0123456789.", beg);
		if (end == std::string::npos)
			end = str.length();
		if (end == beg)
			throw std::runtime_error("Failed to parse float vector string!");

		vec[i] = (float)atof(str.substr(beg, end - beg).c_str());
		pos = end;
	}
}

unsigned int htoi(char c)
{
	if (c >= '0' && c <= '9')
		return c - '0';
	else if (c >= 'a' && c <= 'f')
		return c - 'a' + 0x0A;
	else
		return c - 'A' + 0x0A;
}


Mesh::Mesh()
{
}

std::string const & Mesh::path() const
{
	return path_;
}

uint32_t Mesh::chunkId() const
{
	return chunkId_;
}

unsigned int Mesh::numVertices() const
{
	return (unsigned int)vertices_.size();
}

unsigned int Mesh::numFaces() const
{
	return (unsigned int)triIndices_.size() / 3;
}

bool Mesh::loadOBJ(std::string const & path)
{
	path_ = path;
	std::ifstream f(path.c_str(), std::ifstream::binary | std::ifstream::in);
	if (f.fail())
	{
		FAULT("Failed to open OBJ model file '%s'", path.c_str());
		return false;
	}
	
	std::vector<unsigned int> faces;
	std::string line, keyword;
	bool ignore = false;
	while (!std::getline(f, line).eof())
	{
		std::size_t start = line.find_first_not_of(' ');
		if (start == std::string::npos)
			continue;

		std::size_t kwend = line.find_first_of(" \r", start);
		if (kwend == std::string::npos)
			keyword = line.substr(start);
		else
			keyword = line.substr(start, kwend - start);
		
		if (keyword == "o")
		{
			if (line.length() >= kwend + 9 && line.substr(kwend + 1, 8) == "Terrain_")
			{
				ignore = true;
				// TRACE("Not exporting terrain object '%s'", line.substr(kwend + 1).c_str());
			}
			else
				ignore = false;
		}
		else if (keyword == "v" && !ignore)
		{
			std::size_t e1 = line.find_first_not_of("0123456789-.", kwend + 1);
			std::size_t e2 = line.find_first_not_of("0123456789-.", e1 + 1);
			float x = (float)atof(line.substr(kwend + 1, e1 - kwend - 1).c_str());
			float y = (float)atof(line.substr(e1 + 1, e2 - e1 - 1).c_str());
			float z = (float)atof(line.substr(e2 + 1).c_str());
			Vertex v;
			v.x = z / 100.0f;
			v.y = y / 100.0f;
			v.z = x / 100.0f;
			vertices_.push_back(v);
		}
		else if (keyword == "f" && !ignore)
		{
			std::size_t pos = kwend + 1;
			faces.clear();
			while (pos < line.length() - 1)
			{
				std::size_t end = line.find_first_not_of("0123456789-./", pos);
				if (end == std::string::npos)
					end = line.length();
				faces.push_back(translateFaceIndex(line.substr(pos, end - pos)));
				pos = end + 1;
			}

			for (unsigned int i = 2; i < faces.size(); ++i)
			{
				triIndices_.push_back(faces[i]);
				triIndices_.push_back(faces[i - 1]);
				triIndices_.push_back(faces[0]);
			}
		}
	}

	return true;
}

unsigned int Mesh::translateFaceIndex(std::string const & index)
{
	int val;
	std::size_t slash = index.find_first_of('/');
	if (slash != std::string::npos)
		val = atol(index.substr(0, slash).c_str());
	else
		val = atol(index.c_str());

	if (val == 0 || (val > 0 && static_cast<size_t>(val) > vertices_.size()) || (val < 0 && static_cast<size_t>(-val) > vertices_.size()))
		throw std::runtime_error("Invalid vertex index in object!");

	if (val < 0)
		return vertices_.size() + val;
	else
		return val - 1;
}

void Mesh::exportMesh(MeshExporter & exporter, matrix<float> & transformMat, float offsetX, float offsetZ)
{
	exporter.addComment(std::string(" ===== MESH: ") + path_ + " =====");

	unsigned int baseVertex = exporter.nextVertex();
	vector<float> vertex(4), transformed(4);
	for (auto it = vertices_.begin(); it != vertices_.end(); ++it)
	{
		vertex[0] = it->x;
		vertex[1] = it->y;
		vertex[2] = it->z;
		vertex[3] = 1.0f;
		transformed = prod(transformMat, vertex);
		exporter.addVertex(transformed[0] + offsetX, transformed[1], transformed[2] + offsetZ);
	}
		
	for (unsigned int i = 0; i < triIndices_.size(); i += 3)
	{
		// TODO: Fix winding direction?
		exporter.addFace(triIndices_[i] + baseVertex, triIndices_[i + 1] + baseVertex,
			triIndices_[i + 2] + baseVertex);
	}
}

