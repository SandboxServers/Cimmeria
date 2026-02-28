#include <stdafx.hpp>
#include <mercury/memory_stream.hpp>

namespace Mercury {

packet_processing_exception::packet_processing_exception(const std::string & message)
	: std::runtime_error(message.c_str())
{}

MemoryStream::MemoryStream(uint32_t size)
	: buffer_(0), size_(0)
{
	resize(size);
}

MemoryStream::MemoryStream(uint8_t * buf, uint32_t size)
	: buffer_(0), size_(0)
{
	resize(size);
	memcpy(buffer_, buf, size);
}

MemoryStream::~MemoryStream()
{
	delete [] buffer_;
}

void MemoryStream::resize(uint32_t size)
{
	char * newBuf = new char[size];
	memcpy(newBuf, buffer_, (size > size_) ? size_ : size);
	delete [] buffer_;
	buffer_ = newBuf;
	size_ = size;
}

void * MemoryStream::expand(uint32_t bytes, uint32_t & expanded)
{
	if (offset_ == size_)
		throw std::runtime_error("MemoryStream expand failed: out of buffer space");

	void * base = &buffer_[offset_];
	uint32_t maxExpanded = size_ - offset_;
	expanded = (maxExpanded > bytes) ? bytes : maxExpanded;
	offset_ += expanded;
	return base;
}

void * MemoryStream::expandAtomic(uint32_t bytes)
{
	if (offset_ + bytes > size_)
		throw std::runtime_error("MemoryStream expand failed: out of buffer space");

	void * base = &buffer_[offset_];
	offset_ += bytes;
	return base;
}

void MemoryStream::fetch(void * destination, uint32_t bytes)
{
	if (offset_ + bytes > size_)
		throw std::runtime_error("MemoryStream fetch failed: reached end of stream");

	memcpy(destination, &buffer_[offset_], bytes);
	offset_ += bytes;
}

char * MemoryStream::fetchContiguous(uint32_t bytes)
{
	if (offset_ + bytes > size_)
		throw std::runtime_error("MemoryStream fetch failed: reached end of stream");

	char * ptr = &buffer_[offset_];
	offset_ += bytes;
	return ptr;
}

void MemoryStream::copyTo(OutputStream & destination, uint32_t bytes)
{
	if (offset_ + bytes > size_)
		throw std::runtime_error("MemoryStream copy failed: reached end of stream");

	destination.write(&buffer_[offset_], bytes);
	offset_ += bytes;
}

uint32_t MemoryStream::size() const
{
	return size_;
}

char * MemoryStream::buffer() const
{
	return buffer_;
}

MemoryStream & MemoryStream::pop(void * destination, uint32_t size)
{
	if (offset_ + size > size_)
		throw std::runtime_error("MemoryStream pop failed: reached end of stream");
	size_ -= size;
	memcpy(destination, &buffer_[size_], size);
	return *this;
}

}