#include <stdafx.hpp>
#include <mercury/stream.hpp>

namespace Mercury {

void OutputStream::write(void const * source, uint32_t bytes)
{
	uint8_t const * pos = reinterpret_cast<uint8_t const *>(source);
	uint32_t remaining = bytes;
	while (remaining)
	{
		uint32_t expanded = 0;
		void * dst = expand(remaining, expanded);
		SGW_ASSERT(expanded > 0 && expanded <= remaining);
		memcpy(dst, pos, expanded);
		remaining -= expanded;
		pos += expanded;
	}
}

/*
 * Allocates a new buffer of (size) bytes. The buffer will
 * be freed when the object is destroyed.
 */
StreamBuffer::StreamBuffer(uint32_t size)
	: buffer_(new char[size]), size_(size), isOwner_(true)
{
}

/*
 * Sets the internal pointer to (buffer).  The buffer will *not*
 * be freed when the object is destroyed.
 */
StreamBuffer::StreamBuffer(char * buffer, uint32_t size)
	: buffer_(buffer), size_(size), isOwner_(false)
{
}

StreamBuffer::~StreamBuffer()
{
	if (isOwner_)
		delete [] buffer_;
}

/*
 * Copies the buffer and transfers ownership to the new buffer
 */
StreamBuffer::StreamBuffer(StreamBuffer const & buf)
	: buffer_(buf.buffer_), size_(buf.size_), isOwner_(buf.isOwner_)
{
	buf.isOwner_ = false;
}

StreamBuffer & StreamBuffer::operator = (StreamBuffer & buf)
{
	buffer_ = buf.buffer_;
	size_ = buf.size_;
	isOwner_ = buf.isOwner_;
	buf.isOwner_ = false;
	return *this;
}

char * StreamBuffer::buffer()
{
	return buffer_;
}

uint32_t StreamBuffer::size() const
{
	return size_;
}

InputStream::InputStream()
	: offset_(0)
{
}

StreamBuffer InputStream::fetchBuffer(uint32_t bytes)
{
	char * buf = fetchContiguous(bytes);
	if (buf)
		return StreamBuffer(buf, bytes);
	else
	{
		StreamBuffer buffer(bytes);
		fetch(buffer.buffer(), bytes);
		return buffer;
	}
}

uint32_t InputStream::offset() const
{
	return offset_;
}

uint32_t InputStream::remaining() const
{
	return size() - offset_;
}

void InputStream::seek(uint32_t offset)
{
	if (offset > size())
		throw std::runtime_error("Attempted to seek beyond end of stream");
	offset_ = offset;
}

void InputStream::read(void * destination, uint32_t bytes)
{
	fetch(destination, bytes);
}

}