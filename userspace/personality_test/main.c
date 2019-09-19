// Copyright 2019 Google LLC
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

// This tests the personality driver, which persistently stores some
// U2F application state in flash.


#include "common.h"
#include "storage.h"
#include "kl.h"
#include "fips.h"
#include "x509.h"

#include <stdio.h>
#include <string.h>
#include <timer.h>
#include <rng.h>


static void setup_personality(void);

int check_personality(const perso_st* id) {
  uint32_t chksum[8];
  uint32_t err = 0;
  size_t i;
  if (id == NULL) {
    return EC_ERROR_UNKNOWN;
  }
  err = kl_derive_attest(id->cert_hash, chksum);

  for (i = 0; i < 8; ++i) {
    err |= (chksum[i] ^ id->chksum[i]);
  }
  return err == 0 ? EC_SUCCESS : EC_ERROR_UNKNOWN;
}

int new_personality(perso_st* id) {
  p256_int priv;
  LITE_SHA256_CTX ctx;  // SHA256 output container
  int err = 0;

  if (id == NULL) {
    return EC_ERROR_UNKNOWN;
  }

  memset(id, 0xff, sizeof(perso_st));

  err |= kl_random(id->salt);

  err |= individual_keypair(&priv, &id->pub_x, &id->pub_y, id->salt);
  id->cert_len = generate_cert(&priv, &id->pub_x, &id->pub_y, 1, id->cert,
                               sizeof(id->cert));
  SHA256_INIT(&ctx);
  SHA256_UPDATE(&ctx, id->cert, id->cert_len);
  memcpy(id->cert_hash, SHA256_FINAL(&ctx), SHA256_DIGEST_SIZE);

  err |= kl_derive_attest(id->cert_hash, id->chksum);
  //  printf("Setting personality\n");
  //..set_personality(id);
  // printf("Personality set\n");

  return err == 0 ? EC_SUCCESS : EC_ERROR_UNKNOWN;
}

static void setup_personality(void) {
  perso_st* person = get_personality();

  if (check_personality(person) == EC_SUCCESS) return;
  printf("    - invalid, generating new personality\n");
  if (new_personality(person) == EC_SUCCESS) set_personality(person);
  printf("    - personality set\n");
}


static void check_device_setup(void) {
  printf("  - Checking setup\n");
  ensure_factory_entropy();
  printf("  - Setting up personality.\n");
  setup_personality();
  printf("  - Setup complete.\n");
}

static void print_personality(void) {
  perso_st* person = get_personality();
  printf(" === PERSONALITY === \n");
  printf("Checksum: ");
  for (unsigned int i = 0; i < 8; i++) {
    printf("%08x ", (unsigned int)person->chksum[i]);
  }
  printf("\n");

  printf("Salt:     ");
  for (unsigned int i = 0; i < 8; i++) {
    printf("%08x ", (unsigned int)person->salt[i]);
  }
  printf("\n");

  printf("X:        ");
  for (unsigned int i = 0; i < 8; i++) {
    printf("%08x ", (unsigned int)P256_DIGIT(&person->pub_x, i));
  }
  printf("\n");

  printf("Y:        ");
  for (unsigned int i = 0; i < 8; i++) {
    printf("%08x ", (unsigned int)P256_DIGIT(&person->pub_y, i));
  }
  printf("\n");

  printf("Hash:     ");
  for (unsigned int i = 0; i < 8; i++) {
    printf("%08x ", (unsigned int)person->cert_hash[i]);
  }
  printf("\n");

  printf("Len: %i\n", person->cert_len);

  printf("Cert:\n");
  for (unsigned int i = 0; i < person->cert_len; i += 16) {
    for (unsigned int j = i; j < person->cert_len && j < i + 16; j++) {
      printf("%02x ", person->cert[j]);
    }
    printf("\n");
  }
}

int main(void) {
  init_fips();
  if (kl_init()) {
    printf("kl_init() FAIL\n");
  }
  printf("= Testing Personality Driver =\n");
  check_device_setup();

  print_personality();
  return 0;
}
