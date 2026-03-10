#pragma once

#include <mercury/packet.hpp>
#include <mercury/channel.hpp>
#include <openssl/evp.h>

namespace Mercury {

/*
 * Implements an encryption scheme using AES-256-CBC encryption,
 * PKCS #7 padding and an HMAC-MD5 authentication code.
 *
 * Uses the OpenSSL 3.x EVP API (EVP_MAC for HMAC, EVP_CIPHER_CTX for AES).
 */
class EncryptionFilter : public MessageFilter
{
public:
	const static uint32_t MAX_PACKET_LENGTH = Packet::MAX_LENGTH + 32;
	static const int BLOCK_SIZE = 16;

	EncryptionFilter(uint8_t * key);
	~EncryptionFilter();

	void setKey(uint8_t * key);

	virtual bool sendMessage(Packet & packet, Mercury::MemoryStream & buffer);
	virtual bool receiveMessage(ReceivedPacket & packet);
	virtual uint32_t addedSize();

private:
	EVP_MAC * mac_;
	EVP_MAC_CTX * hmac_;
	uint8_t key_[32];
};

}
