#include <stdafx.hpp>
#include <mercury/message.hpp>
#include <mercury/packet.hpp>
#include <iostream>
#include <iomanip>

namespace Mercury
{

Message::Message(uint32_t buf_size)
	: Mercury::MemoryStream(buf_size), references_(0), messageId_(0xffff), requestId_(0)
{
}

Message::Message(uint8_t * buf, uint32_t buf_size)
	: Mercury::MemoryStream(buf, buf_size), references_(0), messageId_(0xffff), requestId_(0)
{
}

Message::~Message()
{
}

void Message::setMessageId(uint16_t messageId)
{
	messageId_ = messageId;
}

void Message::setRequestId(uint32_t requestId)
{
	requestId_ = requestId;
}

void Message::readString(std::string & s)
{
	uint8_t length;
	*this >> length;
	s.resize(length);
	read(const_cast<char *>(s.data()), length);
}

void Message::readString(std::wstring & s)
{
	uint32_t length;
	*this >> length;
	
	if ((size() - offset_ < length * 2) || length > 0x000FFFFF)
		throw Mercury::packet_processing_exception("Failed to read widestring: reached end of buffer");
	
	s.resize(length);
	read(const_cast<wchar_t *>(s.data()), length * 2);
}

void Message::debugDump()
{
	std::cout << std::hex << std::setw(2);
	std::cout << "Message messageId: " << messageId_ << " length: %d" << std::dec << size_ << std::endl;
	std::cout << "Body: ";
	for (unsigned int i = 0; i < size_; i++)
	{
		if ((i % 16) == 0)
			std::cout << std::endl << "\t";
		std::cout << std::hex << std::setw(2) << std::setfill('0') << (unsigned int)((uint8_t)buffer_[i]) << " ";
	}

	std::cout << std::dec << std::endl;
}

}
