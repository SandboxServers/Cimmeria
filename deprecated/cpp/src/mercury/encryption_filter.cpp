#include <stdafx.hpp>
#include <mercury/encryption_filter.hpp>
#include <openssl/md5.h>

namespace Mercury {
	

EncryptionFilter::EncryptionFilter(uint8_t * key)
{
	HMAC_CTX_init(&hmac_);
	setKey(key);
}

EncryptionFilter::~EncryptionFilter()
{
	HMAC_CTX_cleanup(&hmac_);
}

void EncryptionFilter::setKey(uint8_t * key)
{
	HMAC_Init(&hmac_, key, 32, EVP_md5());

	AES_set_encrypt_key(key, 256, &encryptor_);
	AES_set_decrypt_key(key, 256, &decryptor_);
}

bool EncryptionFilter::sendMessage(Packet & packet, Mercury::MemoryStream & buffer)
{
	/*
	 * This is a hacky way to avoid memory copies
	 * The unpadded part is encrypted directly to the buffer.
	 * Since the packet cannot be changed, the padding & the last part is copied to the buffer,
	 * an in-place encryption is done and the HMAC is appended.
	 */
	unsigned int unpaddedLen = packet.offset() & ~(AES_BLOCK_SIZE - 1);
	unsigned int paddedLen = unpaddedLen + AES_BLOCK_SIZE;

	SGW_ASSERT(buffer.size() >= paddedLen + 16);

	// Encrypt all unpadded blocks
	uint8_t iv[16] = {0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00};
	AES_cbc_encrypt(reinterpret_cast<uint8_t *>(packet.buffer()), reinterpret_cast<uint8_t *>(buffer.buffer()), 
		unpaddedLen, &encryptor_, iv, AES_ENCRYPT);

	uint8_t padblock[AES_BLOCK_SIZE];
	// Copy padded block and append pad
	uint32_t copylen = packet.offset() & (AES_BLOCK_SIZE - 1);
	memcpy(padblock, &packet.buffer()[unpaddedLen], copylen);

	uint8_t pad = 16 - copylen;
	for (unsigned int i = 0; i < pad; i++)
		padblock[copylen + i] = pad;

	// Encrypt padded block
	AES_cbc_encrypt(padblock, reinterpret_cast<uint8_t *>(&buffer.buffer()[unpaddedLen]), 
		AES_BLOCK_SIZE, &encryptor_, iv, AES_ENCRYPT);

	// Append HMAC to destination buffer
	uint8_t hmac[16];
	HMAC_Init_ex(&hmac_, NULL, 0, NULL, NULL);
	HMAC_Update(&hmac_, reinterpret_cast<uint8_t *>(buffer.buffer()), paddedLen);
	unsigned int hmac_len = 16;
	HMAC_Final(&hmac_, hmac, &hmac_len);
	SGW_ASSERT(hmac_len == sizeof(hmac));
	buffer.seek(paddedLen);
	buffer.write(hmac, sizeof(hmac));

	return true;
}

bool EncryptionFilter::receiveMessage(ReceivedPacket & packet)
{
	if (packet.size() < 32 || packet.size() & 0x0f)
	{
		TRACEC(CATEGORY_MERCURY, "Received packet with invalid length: %d", packet.size());
		return false;
	}

	// Verify MAC at the end of the packet
	uint8_t hmac[16];
	HMAC_Init_ex(&hmac_, NULL, 0, NULL, NULL);
	HMAC_Update(&hmac_, reinterpret_cast<uint8_t *>(packet.buffer()), packet.size() - 16);
	unsigned int len;
	HMAC_Final(&hmac_, hmac, &len);

	if (memcmp(hmac, &packet.buffer()[packet.size() - 16], 16) != 0)
	{
		WARNC(CATEGORY_MERCURY, "Decryption error: HMAC verification failed");
		return false;
	}

	packet.chopFromTail(16);

	// Decrypt whole packet
	uint8_t iv[16] = {0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00};
	AES_cbc_encrypt(reinterpret_cast<uint8_t *>(packet.buffer()), reinterpret_cast<uint8_t *>(packet.buffer()), packet.size(), &decryptor_, iv, AES_DECRYPT);

	// Check & remove PKCS #7 padding
	uint8_t pad, pad_continuation;
	packet.pop(pad);
	if (pad > 0x10 || pad == 0x00)
	{
		WARNC(CATEGORY_MERCURY, "Corrupted PKCS #7 padding in packet");
		return false;
	}

	for (unsigned int i = 0; i < (uint32_t)pad - 1; i++)
	{
		packet.pop(pad_continuation);
		if (pad != pad_continuation)
		{
			WARNC(CATEGORY_MERCURY, "Inconsistent PKCS #7 padding in packet");
			return false;
		}
	}

	return true;
}

uint32_t EncryptionFilter::addedSize()
{
	// Max PKCS #7 padding size (16) + HMAC-MD5 size (16)
	return 32;
}


}
