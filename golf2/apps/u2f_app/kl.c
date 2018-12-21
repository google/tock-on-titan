// Fill in KL random functions with calls to random
#include "kl.h"

int kl_init(void) {
  return 0;
}

int kl_random(void* output __attribute__((unused)) ) {
  return 0;
}

int kl_derive(const uint32_t salt[8] __attribute__((unused)),
              const uint32_t input[8] __attribute__((unused)),
              uint32_t output[8]__attribute__((unused))  ) {
  return 0;
}

int kl_derive_attest(const uint32_t input[8] __attribute__((unused)),
                     uint32_t output[8] __attribute__((unused)) ) {
  return 0;
}

int kl_derive_obfs(const uint32_t input[8]__attribute__((unused)),
                   uint32_t output[8] __attribute__((unused)) ) {
  return 0;
}

int kl_derive_origin(const uint32_t input[8]__attribute__((unused)),
                     uint32_t output[8]__attribute__((unused))  ) {
  return 0;
}

int kl_derive_ssh(const uint32_t input[8] __attribute__((unused)),
                  uint32_t output[8] __attribute__((unused))) {
  return 0;
}
