#ifndef __FIPS_CRYPTO_TESTS_H__
#define __FIPS_CRYPTO_TESTS_H__

#include "p256.h"

int fips_aes128_kat(void);
int fips_sha256_kat(void);
int fips_hmac_sha256_kat(void);

// Function not here because poor encapsulation. Function uses too many tightly
// coupled bignum engine fcns. :/
// int fips_ecdsa_sign_kat(void);
// Ugh, also means these can't be static (file scope).
extern p256_int fixed_d;
extern p256_int test_msg;
extern p256_int fixed_k;
extern p256_int fixed_r;
extern p256_int fixed_s;

int fips_hmac_drbg_instantiate_kat(DRBG* ctx);
int fips_hmac_drbg_reseed_kat(DRBG* ctx);
int fips_hmac_drbg_generate_kat(DRBG* ctx);
int fips_cmac_aes128_kat(void);
int fips_ecdsa_sign_kat(void);

int fips_ecdsa_consistency_test(const p256_int* x, const p256_int* y,
                                const p256_int* d);

#endif  // __FIPS_CRYPTO_TESTS_H__
