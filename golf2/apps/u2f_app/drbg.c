#include "include/drbg.h"
#include "include/sha256.h"

void set_8words(uint32_t* dst, uint32_t v) {
  dst[0] = v;
  dst[1] = v;
  dst[2] = v;
  dst[3] = v;
  dst[4] = v;
  dst[5] = v;
  dst[6] = v;
  dst[7] = v;
}

static void _updateV(DRBG* ctx) {
  fips_hwHMAC256_init(ctx->K);
  fips_hwSHA256_update(ctx->V, sizeof(ctx->V));
  fips_hwSHA256_final(ctx->V);
}

static void _updateKV(DRBG* ctx, uint8_t tag, const void* p0, size_t p0_len,
                      const void* p1, size_t p1_len, const void* p2,
                      size_t p2_len) {
  fips_hwHMAC256_init(ctx->K);
  fips_hwSHA256_update(ctx->V, sizeof(ctx->V));
  fips_hwSHA256_update(&tag, sizeof(tag));
  fips_hwSHA256_update(p0, p0_len);
  fips_hwSHA256_update(p1, p1_len);
  fips_hwSHA256_update(p2, p2_len);
  fips_hwSHA256_final(ctx->K);

  _updateV(ctx);
}

void DRBG_update(DRBG* ctx, const void* p0, size_t p0_len, const void* p1,
                 size_t p1_len, const void* p2, size_t p2_len) {
  _updateKV(ctx, 0, p0, p0_len, p1, p1_len, p2, p2_len);
  if (p0_len + p1_len + p2_len == 0) return;
  _updateKV(ctx, 1, p0, p0_len, p1, p1_len, p2, p2_len);
}

void DRBG_reseed(DRBG* ctx, const void* p0, size_t p0_len, const void* p1,
                 size_t p1_len, const void* p2, size_t p2_len) {
  DRBG_update(ctx, p0, p0_len, p1, p1_len, p2, p2_len);
  ctx->reseed_counter = 1;
}

void DRBG_init(DRBG* ctx, const void* p0, size_t p0_len, const void* p1,
               size_t p1_len, const void* p2, size_t p2_len) {
  // TOOD(mschilder): checks provided entropy is >= 1.5 * 128 bits
  set_8words(ctx->K, 0x00000000);
  set_8words(ctx->V, 0x01010101);
  DRBG_reseed(ctx, p0, p0_len, p1, p1_len, p2, p2_len);
}

void DRBG_exit(DRBG* ctx) {
  set_8words(ctx->K, 0);
  set_8words(ctx->V, 0);
}

int DRBG_generate(DRBG* ctx, void* output, size_t output_len,
                  const void* input, size_t input_len) {
  if (output_len > 7500 / 8) return 1;
  if (ctx->reseed_counter >= 10000) return 2;

  if (input_len) {
    DRBG_update(ctx, input, input_len, NULL, 0, NULL, 0);
  }

  while (output_len) {
    size_t n = output_len > sizeof(ctx->V) ? sizeof(ctx->V) : output_len;

    _updateV(ctx);

    memcpy(output, &ctx->V, n);
    output += n;
    output_len -= n;
  }

  DRBG_update(ctx, input, input_len, NULL, 0, NULL, 0);
  ctx->reseed_counter++;

  return 0;
}
