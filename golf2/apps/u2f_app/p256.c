#include "include/p256.h"
#include "include/drbg.h"
#include "include/trng.h"

const p256_int FIPS_SECP256r1_n =  // curve order
    {{0xfc632551, 0xf3b9cac2, 0xa7179e84, 0xbce6faad, -1, -1, 0, -1}};

const p256_int FIPS_SECP256r1_nMin2 =  // curve order - 2
    {{0xfc632551 - 2, 0xf3b9cac2, 0xa7179e84, 0xbce6faad, -1, -1, 0, -1}};

// w/ cryptoc and dcrypto.
// Return -1, 0, 1 for a < b, a == b or a > b respectively.
int fips_p256_cmp(const p256_int* a, const p256_int* b) {
  size_t i;
  int notzero = 0;
  p256_sddigit borrow = 0;

  for (i = 0; i < P256_NDIGITS; ++i) {
    borrow += (p256_sddigit)P256_DIGIT(a, i) - P256_DIGIT(b, i);
    // Track whether any result digit is ever not zero.
    // Relies on !!(non-zero) evaluating to 1, e.g., !!(-1) evaluating to 1.
    notzero |= !!((p256_digit)borrow);
    borrow >>= P256_BITSPERDIGIT;
  }
  return (int)borrow | notzero;
}

// b = a + d. Returns carry, 0 or 1.
int fips_p256_add_d(const p256_int* a, p256_digit d, p256_int* b) {
  size_t i;
  p256_ddigit carry = d;

  for (i = 0; i < P256_NDIGITS; ++i) {
    carry += (p256_ddigit)P256_DIGIT(a, i);
    P256_DIGIT(b, i) = (p256_digit)carry;
    carry >>= P256_BITSPERDIGIT;
  }
  return (int)carry;
}

// 1 < k < |p256|
// TODO: How do we know the DRBG has been initialized?
// Returns 0 on success
int fips_p256_pick(DRBG* drbg, p256_int* output, const void* data,
                   size_t data_len) {
  int result = 0;
  p256_int tmp;

  do {
    result |= DRBG_generate(drbg, &tmp, sizeof(tmp), data, data_len);
  } while (result == 0 && fips_p256_cmp(&tmp, &FIPS_SECP256r1_nMin2) > 0);

  fips_p256_add_d(&tmp, 1, output);
  fips_p256_clear(&tmp);

  return result;
}

// !!! NOT OK for crypto use; only for "zeroization"
static void fips_p256_rnd(p256_int* output) {
  size_t i;

  for (i = 0; i < sizeof(output->a) / sizeof(output->a[0]); ++i) {
    output->a[i] = fips_rand();
  }
}

// Randomize (side-channels) then zeroize
void fips_p256_clear(p256_int* output) {
  fips_p256_rnd(output);
  fips_p256_init(output);
}

void fips_p256_from_bin(const uint8_t src[P256_NBYTES], p256_int* dst) {
  int i;
  const uint8_t* p = &src[0];

  for (i = P256_NDIGITS - 1; i >= 0; --i) {
    P256_DIGIT(dst, i) = (p[0] << 24) | (p[1] << 16) | (p[2] << 8) | p[3];
    p += 4;
  }
}

void fips_p256_to_bin(const p256_int* src, uint8_t dst[P256_NBYTES]) {
  int i;
  uint8_t* p = &dst[0];

  for (i = P256_NDIGITS - 1; i >= 0; --i) {
    p256_digit digit = P256_DIGIT(src, i);
    p[0] = (uint8_t)(digit >> 24);
    p[1] = (uint8_t)(digit >> 16);
    p[2] = (uint8_t)(digit >> 8);
    p[3] = (uint8_t)(digit);
    p += 4;
  }
}

void fips_p256_init(p256_int* a) {
  int i;
  for (i = 0; i < P256_NDIGITS; ++i) P256_DIGIT(a, i) = 0;
}

int fips_p256_is_zero(const p256_int* a) {
  int i, result = 0;
  for (i = 0; i < P256_NDIGITS; ++i) result |= P256_DIGIT(a, i);
  return !result;
}
