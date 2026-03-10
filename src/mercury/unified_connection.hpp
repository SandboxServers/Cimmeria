#pragma once

namespace Mercury
{

/*
 * The circular buffer is a fixed capacity container with a continuously moving start element.
 * When the buffer is filled, new data is written starting at the beginning of the buffer and overwriting the old.
 */
template <typename _T>
class CircularBuffer
{
public:
	CircularBuffer(CircularBuffer const &) = delete;
	CircularBuffer & operator=(CircularBuffer const &) = delete;

	CircularBuffer(std::size_t size)
		: buffer_(new _T[size]), capacity_(size), begin_(0), end_(0)
	{
	}

	~CircularBuffer()
	{
		delete [] buffer_;
	}

	/*
	 * Returns the total amount of items the buffer can hold.
	 */
	std::size_t capacity() const
	{
		return capacity_;
	}

	/*
	 * Returns the amount of items currently in the buffer
	 */
	std::size_t size() const
	{
		if (end_ >= begin_)
		{
			return end_ - begin_;
		}
		else
		{
			return capacity_ - begin_ + end_;
		}
	}

	/*
	 * Returns the amount of free item slots
	 */
	std::size_t free() const
	{
		if (end_ >= begin_)
		{
			return capacity_ - end_ + begin_;
		}
		else
		{
			return begin_ - end_;
		}
	}

	/*
	 * Returns a pointer to the first element of the buffer.
	 * If the buffer is empty, the returned pointer will point to an empty slot.
	 * 
	 * NOTE: The storage area is not guaranteed to be sequential, ie. begin() + 1
	 * may or may not point to the second element.
	 */
	_T * begin()
	{
		return buffer_ + begin_;
	}
	
	/*
	 * Returns a pointer to the first element of the buffer.
	 * If the buffer is empty, the returned pointer will point to an empty slot.
	 * 
	 * NOTE: The storage area is not guaranteed to be sequential, ie. begin() + 1
	 * may or may not point to the second element.
	 */
	_T const * begin() const
	{
		return buffer_ + begin_;
	}
	
	/*
	 * Returns a pointer to the element after the last element of the buffer.
	 */
	_T * end()
	{
		return buffer_ + end_;
	}
	
	/*
	 * Returns a pointer to the element after the last element of the buffer.
	 */
	_T const * end() const
	{
		return buffer_ + end_;
	}

	/*
	 * Resets the contents of the buffer.
	 */
	void clear()
	{
		begin_ = end_ = 0;
	}

	/*
	 * Erases [size] items from the beginning of the buffer.
	 *
	 * @param size Amount of items to erase
	 */
	void eraseBegin(std::size_t size)
	{
		assert(size <= this->size());
		begin_ += size;
		if (begin_ >= capacity_)
		{
			begin_ -= capacity_;
		}
	}

	/*
	 * Insert [size] items at the beginning of the buffer.
	 *
	 * @param size Amount of items to restores
	 */
	void insertBegin(std::size_t size)
	{
		assert(size <= this->free());
		if (size > begin_)
		{
			begin_ += capacity_ - size;
		}
		else
		{
			begin_ -= size;
		}
	}
	
	/*
	 * Erases [size] items from the end of the buffer.
	 *
	 * @param size Amount of items to erase
	 */
	void eraseEnd(std::size_t size)
	{
		assert(size <= this->size());
		if (size > end_)
		{
			end_ = capacity_ - size + end_;
		}
		else
		{
			end_ -= size;
		}
	}
	
	/*
	 * Insert [size] items at the end of the buffer.
	 *
	 * @param size Amount of items to restores
	 */
	void insertEnd(std::size_t size)
	{
		assert(size <= free());
		end_ += size;
		if (end_ >= capacity_)
		{
			end_ -= capacity_;
		}
	}

	/*
	 * Returns the amount of items at begin() that are stored contiguously in the memory
	 */
	std::size_t contiguousBeginItems() const
	{
		if (end_ >= begin_)
		{
			return end_ - begin_;
		}
		else
		{
			return capacity_ - begin_;
		}
	}
	

	/*
	 * Returns the amount of items at end() that are stored contiguously in the memory
	 */
	std::size_t contiguousEndItems() const
	{
		if (end_ >= begin_)
		{
			return capacity_ - end_;
		}
		else
		{
			return begin_ - end_;
		}
	}

private:
	_T * buffer_;
	std::size_t capacity_;
	std::size_t begin_, end_;
};

/*
 * Helper for writing bytestreams into circular buffers
 */
template <typename _Container>
class BufferWriter
{
public:
	BufferWriter(_Container & buffer)
		: buffer_(buffer)
	{
	}

	BufferWriter(BufferWriter<_Container> const & writer)
		: buffer_(writer.buffer_)
	{
	}

	BufferWriter & operator = (BufferWriter<_Container> const & writer)
	{
		buffer_ = writer.buffer_;
	}

	/*
	 * Returns the position of the current write pointer relative to the beginning of the buffer.
	 */
	std::size_t position() const
	{
		return buffer_.size();
	}

	/*
	 * Seeks to the location specified by [position], writes the specified value
	 * to the buffer and returns the end pointer to the original location.
	 *
	 * @param position Write position
	 * @param value    Value to write
	 */
	template <typename _T>
	void writeAt(std::size_t position, _T const & value)
	{
		std::size_t size = buffer_.size();
		buffer_.eraseEnd(size - position);
		*this << value;
		buffer_.insertEnd(size - position - sizeof(_T));
	}

	/*
	 * Writes the specified value to the buffer.
	 * Throws a range_error, if there isn't enough space in the buffer.
	 *
	 * @param value Value to write
	 */
	template <typename _T>
	BufferWriter & operator << (_T const & value)
	{
		static_assert(std::is_pod<_T>::value, "Cannot serialize non-POD types to a buffer");

		if (buffer_.free() <= sizeof(_T))
		{
			throw std::range_error("Buffer is full");
		}

		if (buffer_.contiguousEndItems() >= sizeof(_T))
		{
			*reinterpret_cast<_T *>(buffer_.end()) = value;
			buffer_.insertEnd(sizeof(_T));
		}
		else
		{
			std::size_t endSize = buffer_.contiguousEndItems();
			uint8_t const * valueBuf = reinterpret_cast<uint8_t const *>(&value);
			std::copy(valueBuf, valueBuf + endSize, buffer_.end());
			buffer_.insertEnd(endSize);
			std::copy(valueBuf + endSize, valueBuf + sizeof(_T), buffer_.end());
			buffer_.insertEnd(sizeof(_T) - endSize);
		}
		return *this;
	}
	

	/*
	 * Writes the specified value to the buffer.
	 * Throws a range_error, if there isn't enough space in the buffer.
	 *
	 * @param ptr  Pointer to data
	 * @param size Amount of bytes to write
	 */
	void write(void const * ptr, std::size_t size)
	{
		if (buffer_.free() <= size)
		{
			throw std::range_error("Buffer is full");
		}

		if (buffer_.contiguousEndItems() >= size)
		{
			memcpy(buffer_.end(), ptr, size);
			buffer_.insertEnd(size);
		}
		else
		{
			uint8_t const * bptr = (uint8_t const *)ptr;
			std::size_t endSize = buffer_.contiguousEndItems();
			std::copy(bptr, bptr + endSize, buffer_.end());
			buffer_.insertEnd(endSize);
			std::copy(bptr + endSize, bptr + size, buffer_.end());
			buffer_.insertEnd(size - endSize);
		}
	}

protected:
	_Container & buffer_;
};

template <typename _Container>
class BufferReader
{
public:
	/*
	 * Helper class for storing a contiguous chunk of data for use by readContiguous().
	 */
	class ContiguousBuffer
	{
	public:
		inline ContiguousBuffer(uint8_t * ptr, bool owned)
			: ptr_(ptr), owned_(owned)
		{
		}

		inline ContiguousBuffer(ContiguousBuffer const & src)
		{
			ptr_ = src.ptr_;
			owned_ = src.owned_;
			src.owned_ = false;
		}

		inline ~ContiguousBuffer()
		{
			if (owned_)
				delete [] ptr_;
		}

		inline uint8_t * data() const
		{
			return ptr_;
		}

	private:
		uint8_t * ptr_;
		mutable bool owned_;
		
		inline ContiguousBuffer & operator = (ContiguousBuffer & src) 
		{
			return *this;
		}
	};

	BufferReader(_Container & buffer)
		: buffer_(buffer)
	{
	}

	BufferReader(BufferReader<_Container> const & reader)
		: buffer_(reader.buffer_)
	{
	}

	BufferReader & operator = (BufferReader<_Container> const & reader)
	{
		buffer_ = reader.buffer_;
	}

	/*
	 * Returns the amount of readable items from this buffer.
	 */
	std::size_t remaining() const
	{
		return buffer_.size();
	}
	
	/*
	 * Reads the specified value from the buffer.
	 * Throws a range_error, if there isn't enough data in the buffer.
	 *
	 * @param value Value to read
	 */
	template <typename _T>
	BufferReader & operator >> (_T & value)
	{
		static_assert(std::is_pod<_T>::value, "Cannot unserialize non-POD types from a buffer");

		if (sizeof(_T) > buffer_.size())
		{
			throw std::range_error("Buffer doesn't hold enough data");
		}
		
		if (buffer_.contiguousBeginItems() >= sizeof(_T))
		{
			value = *reinterpret_cast<_T *>(buffer_.begin());
			buffer_.eraseBegin(sizeof(_T));
		}
		else
		{
			std::size_t begSize = buffer_.contiguousBeginItems();
			std::copy(
				buffer_.begin(),
				buffer_.begin() + begSize,
				reinterpret_cast<uint8_t *>(&value)
			);
			buffer_.eraseBegin(begSize);
			std::copy(
				buffer_.begin(),
				buffer_.begin() + sizeof(_T) - begSize,
				reinterpret_cast<uint8_t *>(&value) + begSize
			);
			buffer_.eraseBegin(sizeof(_T) - begSize);
		}
		return *this;
	}
	
	/*
	 * Reads the specified value from the buffer.
	 * Throws a range_error, if there isn't enough data in the buffer.
	 *
	 * @param ptr  Pointer to data
	 * @param size Amount of bytes to read
	 */
	void read(void * ptr, std::size_t size)
	{
		if (size > buffer_.size())
		{
			throw std::range_error("Buffer doesn't hold enough data");
		}

		if (buffer_.contiguousBeginItems() >= size)
		{
			memcpy(ptr, buffer_.begin(), size);
			buffer_.eraseBegin(size);
		}
		else
		{
			uint8_t * bptr = (uint8_t *)ptr;
			std::size_t begSize = buffer_.contiguousBeginItems();
			std::copy(
				buffer_.begin(),
				buffer_.begin() + begSize,
				bptr
			);
			buffer_.eraseBegin(begSize);
			std::copy(
				buffer_.begin(),
				buffer_.begin() + size - begSize,
				bptr + begSize
			);
			buffer_.eraseBegin(size - begSize);
		}
	}
	
	/*
	 * Reads and returns a pointer to the specified area from the buffer and ensures
	 * that the pointer points to a contiguous buffer (copied/reallocated, if required).
	 * Throws a range_error, if there isn't enough data in the buffer.
	 *
	 * @param size Amount of bytes to read
	 */
	ContiguousBuffer readContiguous(std::size_t size)
	{
		if (buffer_.contiguousBeginItems() >= size)
		{
			ContiguousBuffer buf = ContiguousBuffer(buffer_.begin(), false); 
			buffer_.eraseBegin(size);
			return buf;
		}
		else
		{
			ContiguousBuffer buf = ContiguousBuffer(new uint8_t[size], true);
			read(buf.data(), size);
			return buf;
		}
	}

protected:
	_Container & buffer_;
};

class UnifiedConnection : public std::enable_shared_from_this<UnifiedConnection>
{
	public:
		typedef std::shared_ptr<UnifiedConnection> Ptr;
		typedef CircularBuffer<uint8_t> Buffer;
		typedef uint8_t MessageId;

		class Writer : public BufferWriter<Buffer>
		{
		public:
			inline Writer(UnifiedConnection & connection)
				: BufferWriter(connection.sending_), connection_(connection), writing_(false)
			{
			}

			inline ~Writer()
			{
				if (writing_)
				{
					connection_.rollbackMessage();
				}
			}

			void beginMessage(MessageId messageId)
			{
				connection_.beginMessage(messageId);
				writing_ = true;
			}

			void endMessage()
			{
				connection_.endMessage();
				writing_ = false;
			}

			void writeString(std::string const & s)
			{
				*this << (uint32_t)s.length();
				write(s.data(), s.length());
			}

		private:
			UnifiedConnection & connection_;
			bool writing_;
		};

		class Reader : public BufferReader<Buffer>
		{
		public:
			static const unsigned int MaxStringLength = 0x100000;

			inline Reader(UnifiedConnection & connection, MessageId messageId, std::size_t size)
				: BufferReader(connection.received_), messageSize_(size), messageId_(messageId)
			{
			}

			/*
			 * Returns the amount of readable items from this buffer.
			 */
			std::size_t remaining() const
			{
				return messageSize_;
			}

			template <typename _T>
			Reader & operator >> (_T & value)
			{
				if (sizeof(_T) > messageSize_)
				{
					throw std::range_error("Message doesn't hold enough data");
				}

				BufferReader::operator>> (value);
				messageSize_ -= sizeof(_T);
				return *this;
			}

			void readString(std::string & s)
			{
				uint32_t length;
				*this >> length;
				if (length > MaxStringLength)
				{
					throw std::runtime_error("String too long");
				}

				s.resize(length);
				read(const_cast<char *>(s.data()), s.length());
			}

			ContiguousBuffer readContiguous(std::size_t size)
			{
				if (size > messageSize_)
				{
					throw std::range_error("Message doesn't hold enough data");
				}
				
				messageSize_ -= size;
				return BufferReader<Buffer>::readContiguous(size);
			}

			inline MessageId messageId() const
			{
				return messageId_;
			}

		private:
			std::size_t messageSize_;
			MessageId messageId_;
		};

		UnifiedConnection(uint32_t connectionId);
		virtual ~UnifiedConnection();

		boost::asio::ip::tcp::socket & socket();
		uint32_t connectionId() const;
		
		void connect(const std::string & address, uint16_t port);
		void connected();

		void close();
		bool isConnected() const;

	protected:

		virtual void unregisterConnection() = 0;

		/**
		 * Called when a message was received from the peer. The stream contains
		 * message contents excluding the header and message ID.
		 *
		 * @param msg Message stream reader
		 */
		virtual void onMessageReceived(Reader & msg) = 0;

		/**
		 * Callback function called when the connection request was completed.
		 *
		 * @param errcode Error code; connection was successful if code == errc::success
		 */
		virtual void onConnected(const boost::system::error_code & errcode) = 0;

		/**
		 * Callback function called when the connection to the server was closed.
		 *
		 * @param errcode Reason why the connection was lost
		 */
		virtual void onDisconnected(const boost::system::error_code & errcode) = 0;
		
		/**
		 * Begin writing a message to the internal send buffer.
		 * 
		 * @param messageId Message ID (or command code)
		 */
		void beginMessage(MessageId messageId);

		/**
		 * Finish writing a message to the internal send buffer.
		 * If no message is currently being transmitted, a send operation is started on the socket.
		 */
		void endMessage();

		/*
		 * Rolls back the message being written (it won't be sent to the peer).
		 * Can only be called for messages where endMessage() wasn't called yet!
		 */
		void rollbackMessage();

		void close(const boost::system::error_code & errcode);

	private:
		// When a message larger than this is received, the connection is closed
		static const unsigned int MaxMessageLength = 0x100000;

#if defined(_MSC_VER)
#pragma pack(push, 1)
#endif
		struct MessageHeader
		{
			uint32_t length;
			MessageId messageId;
#if defined(__GNUC__)
		} __attribute__((packed));
#elif defined(_MSC_VER)
		};
#pragma pack(pop)
#else
		};
#endif

		// Connection handle
		boost::asio::ip::tcp::socket socket_;
		bool registered_;
		bool connected_;

		// Are we writing a message?
		bool writingMessage_;
		// Offset of the message header we're writing
		std::size_t headerOffset_;

		// Buffer holding the data we've received so far
		// When a complete message is received, it is removed from the buffer
		CircularBuffer<uint8_t> received_;
		// Buffer holding the data we're about to send
		CircularBuffer<uint8_t> sending_;
		// How many bytes can we send from the send buffer?
		// (non-writable bytes are part of an unfinished message)
		std::size_t writableBytes_;

		uint32_t connectionId_;
		
		void connectionCallback(const boost::system::error_code & errcode);
		
		/*
		 * Called when the asynchronous socket receive call has completed and we have received some data.
		 *
		 * @param errcode Status code; receive is successful if == errc::success
		 * @param receivedLength Amount of bytes received on socket
		 */
		void onDataReceived(const boost::system::error_code & errcode, size_t rcvdLength);

		/*
		 * Called when the asynchronous socket send call has completed.
		 *
		 * @param errcode Status code; sending is successful if == errc::success
		 * @param receivedLength Amount of bytes sent on socket
		 */
		void onDataSent(const boost::system::error_code & errcode, size_t sentLength);

		/*
		 * Starts an asynchronous receive operation.
		 * This is automatically called on connection and by onDataReceived(), don't call it manually!
		 */
		void receive();
		/*
		 * Starts an asynchronous send operation.
		 * This is automatically called by onDataSent() and endMessage(), don't call it manually!
		 */
		void send();
};

}

