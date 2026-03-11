#pragma once

#include <mercury/memory_stream.hpp>

namespace Mercury
{

class Message : public MemoryStream
{
	public:
		enum Length
		{
			// No length field, "length" contains the fixed length
			CONSTANT_LENGTH = 0,
			// Has an uint16 length field
			WORD_LENGTH = 2,
			// Has an uint32 length field
			DWORD_LENGTH = 4
		};
		
		/*
		 * An entry in the table that defines sizes for static packets
		 */
		struct Format
		{
			// Size of length field
			Length lengthType;
			// Length of the packet, if constant length type specified. Otherwise its value is ignored.
			unsigned int length;
			// Internal name for packet
			const char * name;
			// Is this message dispatched to a client controlled entity?
			bool isEntityMessage;
		};

		struct Table
		{
			// Amount of entries in table
			unsigned int size;
			// List of messages in table
			Format const * messages;
			// Format of entity messages
			Format const entityMessage;
		};

		typedef std::shared_ptr<Message> Ptr;
		static const uint32_t MAX_HEADER_SIZE = 5;
		
		Message(uint32_t buf_size);
		Message(uint8_t * buf, uint32_t buf_size);
		~Message();

		void setMessageId(uint16_t messageId);
		void setRequestId(uint32_t requestId);

		void readString(std::string & s);
		void readString(std::wstring & s);

		template <typename _T>
		void pyRead (_T & ref)
		{
			static_assert(std::is_trivially_copyable<_T>::value, "Cannot read a non-POD type");
			if (size_ - offset_ < sizeof(_T))
				throw  Mercury::packet_processing_exception("Failed to read message: reached end of buffer");

			if (sizeof(_T) > 16)
				memcpy(&ref, &buffer_[offset_], sizeof(_T));
			else
				ref = *reinterpret_cast<const _T *>(&buffer_[offset_]);

			offset_ += sizeof(_T);
		}

		inline uint16_t messageId() const
		{
			return messageId_;
		}

		inline uint32_t requestId() const
		{
			return requestId_;
		}

		void debugDump();

	protected:
		uint32_t references_;

	private:
		uint16_t messageId_;
		uint32_t requestId_;
};

}
