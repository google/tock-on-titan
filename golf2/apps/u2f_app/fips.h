#ifndef __FIPS_H__
#define __FIPS_H__

#define FLASH_INFO_MANUFACTURE_STATE_SIZE   0x200
#define FLASH_INFO_MANUFACTURE_STATE_OFFSET 0x0

//#include "common.h" /* uint*_t */
#include "stddef.h" /* size_t */
#include "fips_err.h"
#include "drbg.h"
#include "p256.h"
#include "digest_syscalls.h"
#include "sha256.h"

#define PT_TO_BIN fips_p256_to_bin
#define PT_FROM_BIN fips_p256_from_bin
#define PT_CLEAR fips_p256_clear
#define PT_CMP fips_p256_cmp
#define PT_IS_VALID fips_p256_is_valid_point
#define PT_MUL fips_p256_point_mul
#define KEY_FROM_BYTES fips_p256_key_from_bytes
#define ECDSA_SIGN fips_p256_ecdsa_sign

#define SHA256_INIT SHA256_init
#define SHA256_UPDATE SHA256_update
#define SHA256_FINAL SHA256_final
#define SHA256 tock_compat_sha256

/* FIPS startup health tests */
/* Sub-initializers are in their respective crypto drivers */
int init_fips(void);

/* Factory-derived entropy */
#ifndef FACTORY_ENTROPY_OFFSET
#define FACTORY_ENTROPY_OFFSET FLASH_INFO_MANUFACTURE_STATE_OFFSET
#endif
#ifndef FACTORY_ENTROPY_SIZE
#define FACTORY_ENTROPY_SIZE FLASH_INFO_MANUFACTURE_STATE_SIZE
#endif

#define ENTROPY_128SEC 16                  // 128-bit security level
#define NONCE_128SEC (ENTROPY_128SEC / 2)  // 64b

#define ORIGIN_ENTROPY_NBYTES ENTROPY_128SEC  // 128b
#define ORIGIN_NONCE_NBYTES NONCE_128SEC      // 64b

#define POOL_SIZE_NBYTES ORIGIN_ENTROPY_NBYTES + ORIGIN_NONCE_NBYTES
// TODO: BUILD_ASSERT(POOL_SIZE_NBYTES % sizeof(uint32_t) == 0);
#define POOL_SIZE POOL_SIZE_NBYTES / sizeof(uint32_t);

void ensure_factory_entropy(void);
uint32_t flash_physical_info_read_word(uint32_t addr, uint32_t* output);
int flash_info_read_enable(uint32_t addr, uint32_t len);
int flash_info_read_disable(void);

/* DRBG flavors */
void make_drbg1(DRBG*);
void make_drbg2(DRBG*);
int fips_keygen(DRBG*, p256_int*, p256_int*, p256_int*, const void*, int);

#endif  // __FIPS_H__
