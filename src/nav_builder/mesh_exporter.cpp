#include "stdafx.hpp"
#include "mesh_exporter.hpp"
#include "mesh.hpp"


MeshExporter::MeshExporter()
	: vertexIndex_(0)
{
}

MeshExporter::~MeshExporter()
{
}


unsigned int MeshExporter::nextVertex()
{
	return vertexIndex_;
}

MeshObjExporter::MeshObjExporter(std::string const & path)
	: writeBufLen_(0)
{
	outFile_.open(path.c_str(), std::ofstream::binary | std::ofstream::out);
	if (outFile_.fail())
	{
		FAULT("Failed to export base mesh: failed to open '%s'!", path.c_str());
		throw std::runtime_error("Failed to export mesh!");
	}

	// .OBJ vertex indices start from 1
	vertexIndex_ = 1;
}

MeshObjExporter::~MeshObjExporter()
{
	flushWriteBuffer();
}

unsigned int MeshObjExporter::addVertex(float x, float y, float z)
{
	if (writeBufLen_ >= writeBufFlushSize_)
		flushWriteBuffer();

	writeBufLen_ += sprintf(&writeBuf_[writeBufLen_], "v %f %f %f\r\n", x, y, z);
	// outFile_ << "v " << x << " " << y << " " << z << std::endl;
	return vertexIndex_++;
}

void MeshObjExporter::addFace(unsigned int a, unsigned int b, unsigned int c)
{
	if (writeBufLen_ >= writeBufFlushSize_)
		flushWriteBuffer();

	SGW_ASSERT(a < vertexIndex_ && b < vertexIndex_ && c < vertexIndex_);
	writeBufLen_ += sprintf(&writeBuf_[writeBufLen_], "f %d %d %d\r\n", a, b, c);
	// outFile_ << "f " << a << " " << b << " " << c << std::endl;
}

void MeshObjExporter::addComment(std::string const & comment)
{
	outFile_ << "# " << comment << std::endl;
}

void MeshObjExporter::flushWriteBuffer()
{
	writeBuf_[writeBufLen_] = 0;
	outFile_ << writeBuf_;
	writeBufLen_ = 0;
}

MeshBufferExporter::MeshBufferExporter(unsigned int vertices, unsigned int faces)
	: vertices_(new float[vertices * 3]), faces_(new unsigned int[faces * 3]),
	vertexBufferSize_(vertices), faceBufferSize_(faces), faceIndex_(0)
{
	for (int i = 0; i < 3; i++)
	{
		min_[i] = 1000000.0f;
		max_[i] = -1000000.0f;
	}
}

MeshBufferExporter::~MeshBufferExporter()
{
	delete [] vertices_;
	delete [] faces_;
}

unsigned int MeshBufferExporter::addVertex(float x, float y, float z)
{
	SGW_ASSERT(vertexIndex_ < vertexBufferSize_);
	vertices_[vertexIndex_ * 3] = x;
	vertices_[vertexIndex_ * 3 + 1] = y;
	vertices_[vertexIndex_ * 3 + 2] = z;
	if (x < min_[0])
		min_[0] = x;
	if (y < min_[1])
		min_[1] = y;
	if (z < min_[2])
		min_[2] = z;
	
	if (x > max_[0])
		max_[0] = x;
	if (y > max_[1])
		max_[1] = y;
	if (z > max_[2])
		max_[2] = z;
	return vertexIndex_++;
}

void MeshBufferExporter::addFace(unsigned int a, unsigned int b, unsigned int c)
{
	SGW_ASSERT(faceIndex_ < faceBufferSize_);
	faces_[faceIndex_ * 3] = a;
	faces_[faceIndex_ * 3 + 1] = b;
	faces_[faceIndex_ * 3 + 2] = c;
	faceIndex_++;
}

void MeshBufferExporter::addComment(std::string const & comment)
{
}


float * MeshBufferExporter::minBounds()
{
	return min_;
}

float * MeshBufferExporter::maxBounds()
{
	return max_;
}

float * MeshBufferExporter::vertices()
{
	return vertices_;
}

unsigned int * MeshBufferExporter::faces()
{
	return faces_;
}

