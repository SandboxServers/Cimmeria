#pragma once

#include <mercury/stream.hpp>

namespace Mercury {

class packet_processing_exception : public std::runtime_error
{
	public:
		packet_processing_exception(const std::string & message);
};

class MemoryStream : public IOStream
{
public:
	MemoryStream(uint32_t size);
	MemoryStream(uint8_t * buf, uint32_t size);
	virtual ~MemoryStream();

	void resize(uint32_t size);
	
	virtual void * expand(uint32_t bytes, uint32_t & expanded);
	virtual void * expandAtomic(uint32_t bytes);
	virtual void fetch(void * destination, uint32_t bytes);
	virtual char * fetchContiguous(uint32_t bytes);
	virtual void copyTo(OutputStream & destination, uint32_t bytes);
	virtual uint32_t size() const;

	char * buffer() const;

	template <typename _T>
	MemoryStream & pop(_T & value)
	{
		static_assert(std::is_trivially_copyable<_T>::value, "Cannot pop a non-POD type");
		if (offset_ + sizeof(_T) > size_)
			throw std::runtime_error("MemoryStream pop failed: reached end of stream");
		size_ -= sizeof(_T);
		value = *(_T *)&buffer_[size_];
		return *this;
	}

	MemoryStream & pop(void * destination, uint32_t size);

protected:
	char * buffer_;
	uint32_t size_;
};

}