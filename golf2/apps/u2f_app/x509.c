#include "include/asn1.h"
#include "include/kl.h"
#include "include/signed_header.h"
#include "include/storage.h"
#include "include/system.h"
#include "include/u2f_corp.h"
#include "include/u2f_hid_corp.h"
#include "include/x509.h"
#include "include/drbg.h"
#include "include/fips.h"
#include "include/p256.h"
#include "include/sha256.h"
#include "include/p256_ecdsa.h"

// Tock
#include "include/u2f_syscalls.h"

static void get_serial(p256_int* n) {

  // const struct SignedHeader* ro_hdr =
  //    (const struct SignedHeader*)get_program_memory_addr(
  //        system_get_ro_image_copy());
  uint32_t tmp[8] = {0};

  //tmp[7] = ro_hdr->keyid;
  tmp[6] = tock_chip_dev_id0();
  tmp[5] = tock_chip_dev_id1();
  tmp[4] = (SN_VERSION << 16) | ((tock_chip_category() & 255) << 24);

  PT_FROM_BIN((const uint8_t*)tmp, n);
}

int individual_keypair(p256_int* d, p256_int* pk_x, p256_int* pk_y,
                       const uint32_t* salt) {
  uint8_t buf[32];

  if (salt == NULL) salt = get_personality()->salt;

  /* Incorporate HIK & diversification constant */
  if (kl_derive_attest(salt, (uint32_t*)buf)) return EC_ERROR_UNKNOWN;

#ifndef CONFIG_FIPS_ATTESTATION_KEYGEN
  /* Generate legacy attestation key */
  while (!KEY_FROM_BYTES(pk_x, pk_y, d, buf)) {
    SHA256(buf, sizeof(buf), buf);
  }
#else
  {
    /* Generate FIPS compliant attestation key */
    DRBG ctx;

    make_drbg1(&ctx);
    fips_keygen(&ctx, d, pk_x, pk_y, buf, sizeof(buf));
    DRBG_exit(&ctx);
  }
#endif

  return EC_SUCCESS;
}

static void add_CN(ASN1* ctx, int unique) {
  SEQ_START(*ctx, t_SEQ, SEQ_SMALL) {
    SEQ_START(*ctx, t_SET, SEQ_SMALL) {
      SEQ_START(*ctx, t_SEQ, SEQ_SMALL) {
        asn1_object(ctx, OID(commonName));
        if (unique) {
          // TODO: add build version info?
          asn1_string(ctx, t_ASCII, BOARD_STRING);
        } else {
          asn1_string(ctx, t_ASCII, "U2F");
        }
      }
      SEQ_END(*ctx);
    }
    SEQ_END(*ctx);
  }
  SEQ_END(*ctx);
}

int generate_cert(const p256_int* d, const p256_int* pk_x, const p256_int* pk_y,
                  int unique, uint8_t* cert, const int n) {
  ASN1 ctx = {cert, 0};
  LITE_SHA256_CTX sha_ctx;  // SHA256 output container
  p256_int h, r, s;
  DRBG drbg_ctx;

  SEQ_START(ctx, t_SEQ, SEQ_LARGE) {  // outer seq
    // Grab current pointer to data to hash later.
    // Note this will fail if cert body + cert sign is less
    // than 256 bytes (SEQ_MEDIUM) -- not likely.
    uint8_t* body = ctx.p + ctx.n;

    // Cert body seq
    SEQ_START(ctx, t_SEQ, SEQ_MEDIUM) {
      // X509 v3
      SEQ_START(ctx, 0xa0, SEQ_SMALL) { asn1_int(&ctx, 2); }
      SEQ_END(ctx);

      // Serial number
      if (unique) {
        get_serial(&h);
        asn1_p256_int(&ctx, &h);
      } else {
        asn1_int(&ctx, 1);
      }

      // Signature algo
      SEQ_START(ctx, t_SEQ, SEQ_SMALL) {
        asn1_object(&ctx, OID(ecdsa_with_SHA256));
      }
      SEQ_END(ctx);

      // Issuer
      add_CN(&ctx, unique);

      // Expiry
      SEQ_START(ctx, t_SEQ, SEQ_SMALL) {
        asn1_string(&ctx, t_TIME, "20000101000000Z");
        asn1_string(&ctx, t_TIME, "20991231235959Z");
      }
      SEQ_END(ctx);

      // Subject
      add_CN(&ctx, unique);

      // Subject pk
      SEQ_START(ctx, t_SEQ, SEQ_SMALL) {
        // pk parameters
        SEQ_START(ctx, t_SEQ, SEQ_SMALL) {
          asn1_object(&ctx, OID(id_ecPublicKey));
          asn1_object(&ctx, OID(prime256v1));
        }
        SEQ_END(ctx);
        // pk bits
        SEQ_START(ctx, t_BITS, SEQ_SMALL) {
          asn1_tag(&ctx, t_NULL);  // ?
          asn1_pub(&ctx, pk_x, pk_y);
        }
        SEQ_END(ctx);
      }
      SEQ_END(ctx);

      // U2F usb transport indicator extension
      SEQ_START(ctx, 0xa3, SEQ_SMALL) {
        SEQ_START(ctx, t_SEQ, SEQ_SMALL) {
          SEQ_START(ctx, t_SEQ, SEQ_SMALL) {
            asn1_object(&ctx, OID(fido_u2f));
            SEQ_START(ctx, t_BYTES, SEQ_SMALL) {
              SEQ_START(ctx, t_BITS, SEQ_SMALL) {
                asn1_tag(&ctx, 5);     // 5 zero bits
                asn1_tag(&ctx, 0x20);  // usb transport
              }
              SEQ_END(ctx);
            }
            SEQ_END(ctx);
          }
          SEQ_END(ctx);
        }
        SEQ_END(ctx);
      }
      SEQ_END(ctx);
    }
    SEQ_END(ctx);  // Cert body

    // Sign all of cert body
    //    SHA256_INIT(&sha_ctx, 0);
    SHA256_INIT(&sha_ctx);
    SHA256_UPDATE(&sha_ctx, body, (ctx.p + ctx.n) - body);
    PT_FROM_BIN(SHA256_FINAL(&sha_ctx), &h);

    make_drbg2(&drbg_ctx);
    if (!ECDSA_SIGN(&drbg_ctx, d, &h, &r, &s)) {
      return 0;
    }

    // Append X509 signature
    SEQ_START(ctx, t_SEQ, SEQ_SMALL);
    asn1_object(&ctx, OID(ecdsa_with_SHA256));
    SEQ_END(ctx);
    SEQ_START(ctx, t_BITS, SEQ_SMALL) {
      asn1_tag(&ctx, t_NULL);  // ?
      asn1_sig(&ctx, &r, &s);
    }
    SEQ_END(ctx);
  }
  SEQ_END(ctx);

  return ctx.n;
}

int anonymous_cert(const p256_int* d, const p256_int* pk_x,
                   const p256_int* pk_y, uint8_t* cert, const int n) {
  return generate_cert(d, pk_x, pk_y, 0, cert, n);
}

int individual_cert(uint8_t* cert, const int n) {
  const perso_st* me = get_personality();

  if (me->cert_len > n) return 0;

  memcpy(cert, me->cert, me->cert_len);
  return me->cert_len;
}
