#pragma once

#include <mercury/packet.hpp>
#include <mercury/channel.hpp>
#include <openssl/aes.h>
#include <openssl/hmac.h>

namespace Mercury {

/*
 * Implements an encryption scheme using AES-256 encryption,
 * PKCS #7 padding and an HMAC-MD5 authentication code.
 */
class EncryptionFilter : public MessageFilter
{
public:
	const static uint32_t MAX_PACKET_LENGTH = Packet::MAX_LENGTH + 32;

	EncryptionFilter(uint8_t * key);
	~EncryptionFilter();

	void setKey(uint8_t * key);

	virtual bool sendMessage(Packet & packet, Mercury::MemoryStream & buffer);
	virtual bool receiveMessage(ReceivedPacket & packet);
	virtual uint32_t addedSize();

private:
	HMAC_CTX hmac_;
	AES_KEY  encryptor_;
	AES_KEY  decryptor_;
};

}
