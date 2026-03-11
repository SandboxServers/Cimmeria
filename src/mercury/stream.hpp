#pragma once

#include <type_traits>
#include <stdint.h>

namespace Mercury {

/*
 * A generic output stream that can be written to.
 */
class OutputStream
{
public:
	virtual ~OutputStream() {};

	/*
	 * Tries to reserve at most the specified amount of bytes on the stream and
	 * returns a pointer to the reserved area and the size of the successful reservation
	 */
	virtual void * expand(uint32_t bytes, uint32_t & expanded) = 0;
	/*
	 * Reserve the specified amount of bytes on the stream and
	 * returns a pointer to the reserved area
	 */
	virtual void * expandAtomic(uint32_t bytes) = 0;
	void write(void const * source, uint32_t bytes);
};

/*
 * An opaque buffer holding data returned by stream functions.
 */
class StreamBuffer
{
public:
	StreamBuffer(uint32_t size);
	StreamBuffer(char * buffer, uint32_t size);
	~StreamBuffer();
	
	StreamBuffer(StreamBuffer const & buf);
	StreamBuffer & operator = (StreamBuffer & buf);
	
	inline char * data()
	{
		return buffer();
	}

	char * buffer();
	uint32_t size() const;

private:
	char * buffer_;
	uint32_t size_;
	mutable bool isOwner_;
};

/*
 * A generic output stream that can be read from.
 */
class InputStream
{
public:
	InputStream();
	virtual ~InputStream() {};

	/*
	 * Reads the specified amount of bytes from the stream
	 * and copies it to the destination buffer
	 */
	virtual void fetch(void * destination, uint32_t bytes) = 0;

	/*
	 * Tries to return a pointer to the specified amount of contiguous
	 * bytes on the stream (avoiding memory copies); if the function fails
	 * (due to end of stream or non-contiguous stream), the function
	 * returns nullptr.
	 */
	virtual char * fetchContiguous(uint32_t bytes) = 0;
	
	/*
	 * Tries to return a pointer to the specified type from a contiguous
	 * memory chunk. If the function fails (due to end of stream or 
	 * non-contiguous stream), the function returns nullptr.
	 */
	template <typename _T>
	_T * fetchPtr()
	{
		return reinterpret_cast<_T *>(fetchContiguous(sizeof(_T)));
	}

	/*
	 * Returns a buffer holding the specified number of bytes.
	 * Depending on the stream, it may return an allocated chunk of memory,
	 * or a pointer to an internal stream buffer.
	 * Using the StreamBuffer after the stream is destroyed
	 * is undefined behavior.
	 */
	StreamBuffer fetchBuffer(uint32_t bytes);
	
	/*
	 * An alias of fetchBuffer() for compatibility reasons.
	 */
	inline StreamBuffer readContiguous(uint32_t bytes)
	{
		return fetchBuffer(bytes);
	}

	/*
	 * Copies the specified amount of bytes to the destination buffer.
	 */
	virtual void copyTo(OutputStream & destination, uint32_t bytes) = 0;

	/*
	 * Returns the size of the buffer in bytes
	 */
	virtual uint32_t size() const = 0;

	/*
	 * Returns the current read/write offset in bytes
	 */
	uint32_t offset() const;

	/*
	 * Returns the minimal count of remaining readable/writeable bytes
	 * NOTE: In case of expanding buffers (eg. packet streamers)
	 * the actual readable/writable size may be considerably larger
	 * than this value.
	 */
	uint32_t remaining() const;

	/*
	 * Seeks to the specified offset in the stream
	 */
	void seek(uint32_t offset);

	/*
	 * Alias for fetch()
	 */
	void read(void * destination, uint32_t bytes);

protected:
	uint32_t offset_;
};


class IOStream : public InputStream, public OutputStream {};

template <typename _T>
inline InputStream & operator >> (InputStream & stream, _T & value)
{
	static_assert(std::is_trivially_copyable<_T>::value, "Cannot serialize non-POD type");
	stream.fetch(&value, sizeof(_T));
	return stream;
}

template <typename _T>
inline OutputStream & operator << (OutputStream & stream, _T const & value)
{
	static_assert(std::is_trivially_copyable<_T>::value, "Cannot serialize non-POD type");
	*reinterpret_cast<_T *>(stream.expandAtomic(sizeof(_T))) = value;
	return stream;
}

}