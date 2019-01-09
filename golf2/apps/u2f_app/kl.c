// Copyright 2018 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Fill in KL random functions with calls to random
#include "include/kl.h"

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
