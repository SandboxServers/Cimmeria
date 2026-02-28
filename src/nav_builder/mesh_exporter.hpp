#pragma once

#include <fstream>

class MeshExporter
{
public:
	MeshExporter();
	~MeshExporter();

	unsigned int nextVertex();

	virtual unsigned int addVertex(float x, float y, float z) = 0;
	virtual void addFace(unsigned int a, unsigned int b, unsigned int c) = 0;
	virtual void addComment(std::string const & comment) = 0;

protected:
	unsigned int vertexIndex_;
};

class MeshObjExporter : public MeshExporter
{
public:
	MeshObjExporter(std::string const & path);
	~MeshObjExporter();

	virtual unsigned int addVertex(float x, float y, float z);
	virtual void addFace(unsigned int a, unsigned int b, unsigned int c);
	virtual void addComment(std::string const & comment);
	void flushWriteBuffer();

private:
	std::ofstream outFile_;
	const static unsigned int writeBufSize_ = 0x1000;
	const static unsigned int writeBufFlushSize_ = 0xF00;
	char writeBuf_[writeBufSize_];
	unsigned int writeBufLen_;
};

class MeshBufferExporter : public MeshExporter
{
public:
	MeshBufferExporter(unsigned int vertices, unsigned int faces);
	~MeshBufferExporter();

	virtual unsigned int addVertex(float x, float y, float z);
	virtual void addFace(unsigned int a, unsigned int b, unsigned int c);
	virtual void addComment(std::string const & comment);

	float * minBounds();
	float * maxBounds();

	float * vertices();
	unsigned int * faces();

private:
	float * vertices_;
	unsigned int * faces_;
	unsigned int vertexBufferSize_;
	unsigned int faceBufferSize_;
	unsigned int faceIndex_;
	float min_[3], max_[3];
};

