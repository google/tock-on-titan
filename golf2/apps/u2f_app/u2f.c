/*
 * Main apdu dispatcher and u2f command handlers.
 */

#include "asn1.h"
#include "storage.h"
#include "u2f_corp.h" /* asn.1 der */
#include "aes.h"
#include "fips.h"
#include "fips_err.h"
#include "sha256.h"
#include "p256_ecdsa.h"
#include "trng.h"
#include "kl.h"

#define POP_TOUCH_YES 1

int pop_check_presence(int consume, int bpm);

int pop_check_presence(int consume __attribute__((unused)),
                       int bpm __attribute__((unused))) {
  return POP_TOUCH_YES;
}

/**
 * Encode fw version number into byte array.
 * First byte is flags byte.
 * Signal fw version is present.
 */
static void encodeVersion(uint8_t *dst __attribute__((unused))) {
  //  const struct SignedHeader *hdr =
  //    (const struct SignedHeader *)get_program_memory_addr(
  //        system_get_image_copy());

  //  dst[0] = G2F_KH_VERSION;
  // dst[1] = hdr->epoch_;
  // dst[2] = hdr->major_;
  // dst[3] = hdr->minor_;
}

/**
 * Return !0 if both arrays are equal.
 * Constant time.
 */
static int equal_arrays(const void *va, const void *vb, size_t n) {
  const uint8_t *a = (const uint8_t *)va;
  const uint8_t *b = (const uint8_t *)vb;
  uint8_t accu = 0;

  while (n--) accu |= (*a++) ^ (*b++);
  return accu == 0;
}

/**
 * (re)generate origin-specific ECDSA keypair via a DRBG seeded w/
 * factory-derived entropy.
 *
 * @param seed ptr to store 32 byte seed to regenate this key on this chip
 * @param d p256_int pointer to ECDSA private key
 * @param pk_x p256_int pointer to public key point
 * @param pk_y p256_int pointer to public key point
 *
 * @return EC_SUCCESS
 */
static int origin_keypair(uint8_t *seed, p256_int *d, p256_int *pk_x,
                          p256_int *pk_y) {
  uint32_t tmp[8];
  DRBG ctx;

  if (pk_x != NULL && pk_y != NULL) {
    /* Initial generation: pick origin additional data */
    if (kl_random(seed)) return EC_ERROR_UNKNOWN;
    /* zeroize last 8 bytes of seed
     * to keep room for covert channel */
    memset(seed + 24, 0, 8);
  }
  memcpy(tmp, seed, sizeof(tmp));
  if (kl_derive_origin(tmp, tmp)) return EC_ERROR_UNKNOWN;

  make_drbg1(&ctx);
  fips_keygen(&ctx, d, pk_x, pk_y, tmp, sizeof(tmp));
  DRBG_exit(&ctx);

  return EC_SUCCESS;
}

/* Interleave bytes of two 32 byte arrays. */
/* Only take 24 first bytes of each input and zeroize rest of out. */
static void interleave32(const uint8_t *a, const uint8_t *b, uint8_t *out) {
  size_t i;

  for (i = 0; i < 24; ++i) {
    out[2 * i + 0] = a[i];
    out[2 * i + 1] = b[i];
  }
  for (; i < 32; ++i) {
    out[2 * i + 0] = 0;
    out[2 * i + 1] = 0;
  }
}

/* De-interleave 64 bytes into two 32 arrays. */
/* Only write 24 bytes to each output and zeroize the other 8. */
static void deinterleave64(const uint8_t *in, uint8_t *a, uint8_t *b) {
  size_t i;

  for (i = 0; i < 24; ++i) {
    a[i] = in[2 * i + 0];
    b[i] = in[2 * i + 1];
  }
  for (; i < 32; ++i) {
    a[i] = 0;
    b[i] = 0;
  }
}

/***
 * Generate a KL-derived private key, as bytes. Should be 32 bytes
 * (256b key).
 */
static int gen_scramblek(const uint8_t *origin __attribute((unused)),
                         uint8_t *scramblek,
                         size_t key_len) {
  uint32_t buf[8];
  DRBG ctx;

  if (key_len != sizeof(buf)) return EC_ERROR_UNKNOWN;
  if (kl_derive_obfs(get_personality()->salt, buf)) return EC_ERROR_UNKNOWN;

  make_drbg1(&ctx);
  DRBG_generate(&ctx, scramblek, key_len, buf, sizeof(buf));

  return EC_SUCCESS;
}

/* en/dis-entangle kh w/ the origin dependent scramble key. */
static int obfuscate_kh(const uint8_t *origin, const uint8_t *in, uint8_t *out,
                        enum AES_encrypt_mode mode) {
  uint8_t scramblek[SHA256_DIGEST_SIZE];
  uint8_t iv[AES_BLOCK_LEN] = {0};
  int i;

  /* KEK derivation */
  if (gen_scramblek(origin, scramblek, sizeof(scramblek)))
    return EC_ERROR_UNKNOWN;

  fips_aes_init(scramblek, 256, iv, AES_CIPHER_MODE_CBC, mode);

  for (i = 0; i < 3; i++) {
    fips_aes_block(in + i * AES_BLOCK_LEN, out + i * AES_BLOCK_LEN);
  }
  /* block[3] ^= sha256(block[0..2]) */
  SHA256(out, 3 * AES_BLOCK_LEN, scramblek);
  for (i = 0; i < AES_BLOCK_LEN; ++i) {
    out[3 * AES_BLOCK_LEN + i] = in[3 * AES_BLOCK_LEN + i] ^ scramblek[i];
  }

  return EC_SUCCESS;
}

static uint16_t u2f_version(APDU apdu, uint8_t *obuf, uint16_t *obuf_len) {
  /*  "U2F_V2" */
  static const uint8_t version[] = {0x55, 0x32, 0x46, 0x5f, 0x56, 0x32};

  if (apdu.len) return U2F_SW_WRONG_LENGTH;

  memcpy(obuf, version, sizeof(version));

  *obuf_len = sizeof(version);
  return U2F_SW_NO_ERROR;
}

/* U2F REGISTER command  */
static uint16_t u2f_register(APDU apdu, uint8_t *obuf, uint16_t *obuf_len) {
  const U2F_REGISTER_REQ *req = (const U2F_REGISTER_REQ *)apdu.data;
  U2F_REGISTER_RESP *resp = (U2F_REGISTER_RESP *)obuf;
  int l, m_off; /* msg length and interior offset */

  p256_int r, s; /* ecdsa signature */
  /* Origin keypair */
  uint8_t od_seed[SHA256_DIGEST_SIZE];
  p256_int od, opk_x, opk_y;
  /* KDF, Key handle */
  uint8_t kh[U2F_APPID_SIZE + sizeof(p256_int)];
  uint8_t buf[U2F_APPID_SIZE + sizeof(p256_int)];
  /* sha256({RFU, app ID, nonce, keyhandle, public key}) */
  p256_int h;
  const uint8_t rfu = U2F_REGISTER_HASH_ID;
  const uint8_t pk_start = U2F_POINT_UNCOMPRESSED;
  p256_int att_d;
  uint32_t cert_len;
  LITE_SHA256_CTX ctx;  // SHA256 output container
  DRBG drbg_ctx;

  printf("REGISTER CMD\n");

  if (apdu.len != sizeof(U2F_REGISTER_REQ)) {
    printf(
        "ERR: U2F REGISTER INS error wrong "
        "length\n");
    return U2F_SW_WRONG_LENGTH;
  }

  /* Check user presence, w/ optional consume */
  if (pop_check_presence(apdu.p1 & G2F_CONSUME, 250) != POP_TOUCH_YES &&
      (apdu.p1 & G2F_TUP) != 0) {
    return U2F_SW_CONDITIONS_NOT_SATISFIED;
  }

  /* Check crypto state */
  if (fips_fatal != FIPS_INITIALIZED) {
    return U2F_SW_WTF + 6;
  }

  /* Generate origin-specific keypair */
  if (origin_keypair(od_seed, &od, &opk_x, &opk_y) != EC_SUCCESS) {
    printf("ERR: Origin-specific keypair generation failed!");
    return U2F_SW_WTF + 1;
  }

  /* Generate key handle */
  /* Interleave origin ID, origin priv key, obfuscate and export. */
  interleave32(req->appId, od_seed, buf);

  if (apdu.p1 & G2F_ATTEST) {
    /* encode fw version # in buf[48..63] */
    encodeVersion(buf + 48);
  } else {
    /* kill covert channel */
    rand_bytes(buf + 48, 16);
  }

  if (obfuscate_kh(req->appId, buf, kh, AES_ENCRYPT_MODE) != EC_SUCCESS)
    return U2F_SW_WTF + 2;

  /* Response message hash for signing */
  SHA256_INIT(&ctx);
  SHA256_UPDATE(&ctx, &rfu, sizeof(rfu));
  SHA256_UPDATE(&ctx, req->appId, U2F_APPID_SIZE);
  SHA256_UPDATE(&ctx, req->chal, U2F_CHAL_SIZE);
  SHA256_UPDATE(&ctx, kh, sizeof(kh));
  SHA256_UPDATE(&ctx, &pk_start, sizeof(pk_start));

  /* Insert origin-specific public keys into the response */
  // TODO: Separate building the data structure from the hashing ops
  PT_TO_BIN(&opk_x, resp->pubKey.x); /* endianness */
  PT_TO_BIN(&opk_y, resp->pubKey.y); /* endianness */
  SHA256_UPDATE(&ctx, resp->pubKey.x, sizeof(p256_int));
  SHA256_UPDATE(&ctx, resp->pubKey.y, sizeof(p256_int));
  PT_FROM_BIN(SHA256_FINAL(&ctx), &h);

  /* Construct remainder of the response */
  resp->registerId = U2F_REGISTER_ID;
  l = sizeof(resp->registerId);
  resp->pubKey.pointFormat = U2F_POINT_UNCOMPRESSED;
  l += sizeof(resp->pubKey);
  resp->keyHandleLen = sizeof(kh);
  l += sizeof(resp->keyHandleLen);
  memcpy(resp->keyHandleCertSig, kh, sizeof(kh));
  l += sizeof(kh);
  m_off = sizeof(kh);

  if (apdu.p1 & G2F_ATTEST) {
    /* Use a keyladder-derived keypair for Individual attestation */
    if (individual_keypair(&att_d, NULL, NULL, NULL) != EC_SUCCESS) {
      printf("ERR: Attestation key generation failed!");
      return U2F_SW_WTF + 3;
    }
    printf("indiv attest\n");
    cert_len =
        individual_cert(resp->keyHandleCertSig + m_off, U2F_MAX_ATT_CERT_SIZE);
  } else {
    /* Anon attestation keypair; use origin key to self-sign */
    printf("anon attest\n");
    cert_len =
        anonymous_cert(&od, &opk_x, &opk_y, resp->keyHandleCertSig + m_off,
                       U2F_MAX_ATT_CERT_SIZE);
    att_d = od;
  }
  if (cert_len == 0) return U2F_SW_WTF + 4;

  printf("copied n = %d byte cert into response\n", cert_len);
  l += cert_len;
  m_off += cert_len;

  /* Sign over the response w/ the attestation key */
  make_drbg2(&drbg_ctx);
  if (!ECDSA_SIGN(&drbg_ctx, &att_d, &h, &r, &s)) {
    PT_CLEAR(&att_d);
    printf("ERR: signing error 1\n");
    return U2F_SW_WTF + 5;
  }
  PT_CLEAR(&att_d);

  /* Signature -> ASN.1 DER encoded bytes */
  l += asn1_sigp(resp->keyHandleCertSig + m_off, &r, &s);

  printf("cmd REGISTER completed\n");
  *obuf_len = l;

  return U2F_SW_NO_ERROR; /* APDU success */
}

static uint16_t u2f_authenticate(APDU apdu, uint8_t *obuf, uint16_t *obuf_len) {
  const U2F_AUTHENTICATE_REQ *req = (const U2F_AUTHENTICATE_REQ *)apdu.data;
  U2F_AUTHENTICATE_RESP *resp = (U2F_AUTHENTICATE_RESP *)obuf;
  uint8_t kh[KH_LEN];
  uint8_t od_seed[SHA256_DIGEST_SIZE];

  p256_int origin_d;
  uint8_t origin[U2F_APPID_SIZE];

  LITE_SHA256_CTX ctx;  // SHA output container
  p256_int h, r, s;
  uint8_t sig_len;

  uint32_t count = 0;

  DRBG drbg_ctx;

  if (apdu.len != U2F_APPID_SIZE + U2F_CHAL_SIZE + 1 + KH_LEN) {
    printf(
        "ERR: U2F AUTHENTICATE INS error wrong "
        "length\n");
    return U2F_SW_WRONG_LENGTH;
  }

  /* Check crypto state */
  if (fips_fatal != FIPS_INITIALIZED) {
    return U2F_SW_WTF + 6;
  }

  /* Disentangle key handle */
  if (obfuscate_kh(req->appId, req->keyHandle, kh, AES_DECRYPT_MODE) !=
      EC_SUCCESS)
    return U2F_SW_WTF + 1;
  deinterleave64(kh, origin, od_seed);

  /* Check whether appId (i.e. origin) matches. Constant time. */
  if (!equal_arrays(origin, req->appId, 24)) return U2F_SW_WRONG_DATA;

  /* Origin check only? */
  if (apdu.p1 & G2F_CHECK) return U2F_SW_CONDITIONS_NOT_SATISFIED;

  /* Sense user presence, with optional consume */
  resp->flags = pop_check_presence(apdu.p1 & G2F_CONSUME, 500) == POP_TOUCH_YES;

  /* Mandatory user presence? */
  if ((apdu.p1 & G2F_TUP) != 0 && resp->flags == 0)
    return U2F_SW_CONDITIONS_NOT_SATISFIED;

  /* Increment-only counter in flash. OK to share between origins. */
  count = flash_ctr_incr();
  resp->ctr[0] = (count >> 24) & 0xFF;
  resp->ctr[1] = (count >> 16) & 0xFF;
  resp->ctr[2] = (count >> 8) & 0xFF;
  resp->ctr[3] = count & 0xFF;

  /* Message signature */
  SHA256_INIT(&ctx);
  SHA256_UPDATE(&ctx, req->appId, U2F_APPID_SIZE);
  SHA256_UPDATE(&ctx, &resp->flags, sizeof(uint8_t));
  SHA256_UPDATE(&ctx, resp->ctr, U2F_CTR_SIZE);
  SHA256_UPDATE(&ctx, req->chal, U2F_CHAL_SIZE);
  PT_FROM_BIN(SHA256_FINAL(&ctx), &h);

  if (origin_keypair(od_seed, &origin_d, NULL, NULL)) return U2F_SW_WTF + 2;

  make_drbg2(&drbg_ctx);
  if (!ECDSA_SIGN(&drbg_ctx, &origin_d, &h, &r, &s)) {
    PT_CLEAR(&origin_d);
    return U2F_SW_WTF + 3;
  }
  PT_CLEAR(&origin_d);

  sig_len = asn1_sigp(resp->sig, &r, &s);

  *obuf_len = sizeof(resp->flags) + U2F_CTR_SIZE + sig_len;
  return U2F_SW_NO_ERROR;
}

/* Receives an apdu-framed message from the U2F HID transport layer
 * Return output buffer's byte count on success. */
uint16_t apdu_rcv(const uint8_t *ibuf, uint16_t in_len, uint8_t *obuf) {
  uint16_t obuf_len = 0;
  uint16_t sw = U2F_SW_CLA_NOT_SUPPORTED;
  uint8_t CLA = ibuf[0];
  uint8_t INS = ibuf[1];
  APDU apdu = {.p1 = ibuf[2], .p2 = ibuf[3], .len = 0, .data = ibuf + 5};

  /* Fill in APDU structure */
  /* [CLA INS P1 P2 [LC1 [LC2 LC3 <request-data>]]] */
  CLA = ibuf[0];
  INS = ibuf[1];

  /* ISO7618 LC decoding */
  if (in_len >= 5) {
    apdu.len = ibuf[4];
  }
  if (apdu.len == 0 && in_len >= 7) {
    apdu.len = (ibuf[5] << 8) | ibuf[6];
    apdu.data += 2;
  }

  printf("\n\nAPDU rcv'd: %.7h\n", ibuf);
  printf("  APDU.len: %x\n", apdu.len);

  if (CLA == 0x00) { /* Always 0x00 */
    sw = U2F_SW_INS_NOT_SUPPORTED;

    switch (INS) {
      case (U2F_REGISTER):
        printf("U2F REGISTER cmd received\n");
        sw = u2f_register(apdu, obuf, &obuf_len);
        if (fips_fatal != FIPS_INITIALIZED) {
          obuf_len = 0;
          sw = U2F_SW_WTF + 6;
        }
        break;

      case (U2F_AUTHENTICATE):
        printf("U2F AUTHENTICATE cmd received\n");
        sw = u2f_authenticate(apdu, obuf, &obuf_len);
        if (fips_fatal != FIPS_INITIALIZED) {
          obuf_len = 0;
          sw = U2F_SW_WTF + 6;
        }
        break;

      case (U2F_VERSION):
        printf("U2F VERSION\n");
        sw = u2f_version(apdu, obuf, &obuf_len);
        break;
    }

#if defined(CONFIG_G2F)
    /* Not a U2F INS. Try internal extensions next. */
    if (sw == U2F_SW_INS_NOT_SUPPORTED) {
      sw = ssh_dispatch(INS, apdu, obuf, &obuf_len);
    }
#endif
  }

  /* append SW status word */
  obuf_len += SW_OFFSET;
  obuf[obuf_len - 2] = sw >> 8;
  obuf[obuf_len - 1] = sw;

  printf(" : %04x\n", sw);

  {
    int i = 0;

    printf("\nAPDU response buffer: %d\n", obuf_len);
    for (i = 0; i < obuf_len; i++) printf("%02X", obuf[i]);
    printf("\n");
  }

  return obuf_len;
}
