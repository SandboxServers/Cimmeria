#include <stdafx.hpp>
#include <mercury/encryption_filter.hpp>
#include <openssl/params.h>

namespace Mercury {

EncryptionFilter::EncryptionFilter(uint8_t * key)
	: mac_(nullptr), hmac_(nullptr)
{
	mac_ = EVP_MAC_fetch(NULL, "HMAC", NULL);
	hmac_ = EVP_MAC_CTX_new(mac_);
	setKey(key);
}

EncryptionFilter::~EncryptionFilter()
{
	EVP_MAC_CTX_free(hmac_);
	EVP_MAC_free(mac_);
}

void EncryptionFilter::setKey(uint8_t * key)
{
	memcpy(key_, key, 32);

	OSSL_PARAM params[] = {
		OSSL_PARAM_construct_utf8_string("digest", const_cast<char *>("MD5"), 0),
		OSSL_PARAM_construct_END()
	};
	EVP_MAC_init(hmac_, key_, 32, params);
}

bool EncryptionFilter::sendMessage(Packet & packet, Mercury::MemoryStream & buffer)
{
	/*
	 * This is a hacky way to avoid memory copies
	 * The unpadded part is encrypted directly to the buffer.
	 * Since the packet cannot be changed, the padding & the last part is copied to the buffer,
	 * an in-place encryption is done and the HMAC is appended.
	 */
	unsigned int unpaddedLen = packet.offset() & ~(BLOCK_SIZE - 1);
	unsigned int paddedLen = unpaddedLen + BLOCK_SIZE;

	SGW_ASSERT(buffer.size() >= paddedLen + 16);

	// Build padded last block with PKCS #7 padding
	uint8_t padblock[BLOCK_SIZE];
	uint32_t copylen = packet.offset() & (BLOCK_SIZE - 1);
	memcpy(padblock, &packet.buffer()[unpaddedLen], copylen);

	uint8_t pad = 16 - copylen;
	for (unsigned int i = 0; i < pad; i++)
		padblock[copylen + i] = pad;

	// Encrypt using EVP AES-256-CBC with zero IV, padding disabled (manual PKCS#7)
	uint8_t iv[16] = {};
	EVP_CIPHER_CTX * ctx = EVP_CIPHER_CTX_new();
	EVP_EncryptInit_ex(ctx, EVP_aes_256_cbc(), NULL, key_, iv);
	EVP_CIPHER_CTX_set_padding(ctx, 0);

	int outl = 0;
	EVP_EncryptUpdate(ctx, reinterpret_cast<uint8_t *>(buffer.buffer()), &outl,
		reinterpret_cast<uint8_t *>(packet.buffer()), unpaddedLen);

	int outl2 = 0;
	EVP_EncryptUpdate(ctx, reinterpret_cast<uint8_t *>(&buffer.buffer()[unpaddedLen]), &outl2,
		padblock, BLOCK_SIZE);

	int finallen = 0;
	EVP_EncryptFinal_ex(ctx, reinterpret_cast<uint8_t *>(&buffer.buffer()[unpaddedLen + outl2]), &finallen);
	EVP_CIPHER_CTX_free(ctx);

	// Append HMAC-MD5
	uint8_t hmac[16];
	size_t hmac_len = sizeof(hmac);
	EVP_MAC_init(hmac_, NULL, 0, NULL);
	EVP_MAC_update(hmac_, reinterpret_cast<uint8_t *>(buffer.buffer()), paddedLen);
	EVP_MAC_final(hmac_, hmac, &hmac_len, sizeof(hmac));
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

	// Verify HMAC-MD5 at the end of the packet
	uint8_t hmac[16];
	size_t hmac_len = sizeof(hmac);
	EVP_MAC_init(hmac_, NULL, 0, NULL);
	EVP_MAC_update(hmac_, reinterpret_cast<uint8_t *>(packet.buffer()), packet.size() - 16);
	EVP_MAC_final(hmac_, hmac, &hmac_len, sizeof(hmac));

	if (memcmp(hmac, &packet.buffer()[packet.size() - 16], 16) != 0)
	{
		WARNC(CATEGORY_MERCURY, "Decryption error: HMAC verification failed");
		return false;
	}

	packet.chopFromTail(16);

	// Decrypt using EVP AES-256-CBC with zero IV, padding disabled (manual PKCS#7)
	uint8_t iv[16] = {};
	EVP_CIPHER_CTX * ctx = EVP_CIPHER_CTX_new();
	EVP_DecryptInit_ex(ctx, EVP_aes_256_cbc(), NULL, key_, iv);
	EVP_CIPHER_CTX_set_padding(ctx, 0);

	int outl = 0;
	EVP_DecryptUpdate(ctx, reinterpret_cast<uint8_t *>(packet.buffer()), &outl,
		reinterpret_cast<uint8_t *>(packet.buffer()), packet.size());

	int finallen = 0;
	EVP_DecryptFinal_ex(ctx, reinterpret_cast<uint8_t *>(&packet.buffer()[outl]), &finallen);
	EVP_CIPHER_CTX_free(ctx);

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
