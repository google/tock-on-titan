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

#include "digest_syscalls.h"
#include "tock.h"

// TODO These need to be standardized
#define H1B_DRIVER_DIGEST 0x40003

// command() type ids
#define TOCK_DIGEST_CMD_INITIALIZE 0
#define TOCK_DIGEST_CMD_UPDATE     1
#define TOCK_DIGEST_CMD_FINALIZE   2
#define TOCK_DIGEST_CMD_BUSY       3
#define TOCK_DIGEST_CMD_CERT_INIT  4

// allow() type ids
#define TOCK_DIGEST_ALLOW_INPUT    0
#define TOCK_DIGEST_ALLOW_OUTPUT   1

int tock_digest_set_input(void* buf, size_t len) {
  int rval = allow(H1B_DRIVER_DIGEST, TOCK_DIGEST_ALLOW_INPUT, buf, len);
  if (rval != TOCK_SUCCESS){
    printf("Digest set input returned error %i\n", rval);
  }
  return rval;
}

int tock_digest_set_output(void* buf, size_t len) {
  int rval = allow(H1B_DRIVER_DIGEST, TOCK_DIGEST_ALLOW_OUTPUT, buf, len);
  if (rval != TOCK_SUCCESS){
    printf("Digest set output returned error %i\n", rval);
  }
  return rval;
}

int tock_digest_hash_initialize(TockDigestMode mode) {
  int rval = command(H1B_DRIVER_DIGEST, TOCK_DIGEST_CMD_INITIALIZE, mode, 0);
  return rval;
}

int tock_digest_cert_initialize(uint32_t cert) {
  int rval = command(H1B_DRIVER_DIGEST, TOCK_DIGEST_CMD_CERT_INIT, cert, 0);
  return rval;
}

int tock_digest_hash_update(size_t n) {
  int rval = command(H1B_DRIVER_DIGEST, TOCK_DIGEST_CMD_UPDATE, n, 0);
  return rval;
}

int tock_digest_hash_finalize(void) {
  return command(H1B_DRIVER_DIGEST, TOCK_DIGEST_CMD_FINALIZE, 0, 0);
}

int tock_digest_busy(void) {
  return (command(H1B_DRIVER_DIGEST, TOCK_DIGEST_CMD_BUSY, 0, 0) == TOCK_EBUSY);
}

int tock_digest_hash_easy(void* input_buf, size_t input_len,
                          void* output_buf, size_t output_len, TockDigestMode mode) {
  int err = -1;
  err = tock_digest_set_input(input_buf, input_len);
  if (err < 0) {
    printf("Digest: error %i on hash_easy set_input\n", err);
    return err;
  }
  err = tock_digest_set_output(output_buf, output_len);
  if (err < 0) {
    printf("Digest: error %i on hash_easy set_output\n", err);
    return err;
  }
  err = tock_digest_hash_initialize(mode);
  if (err < 0) {
    printf("Digest: error %i on hash_easy initialize\n", err);
    return err;
  }
  err = tock_digest_hash_update(input_len);
  if (err < 0) {
    printf("Digest: error %i on hash_easy update\n", err);
    return err;
  }
  return tock_digest_hash_finalize();
}

int tock_digest_with_cert(uint32_t cert,
                          void* input_buf, size_t input_len,
                          void* output_buf, size_t output_len) {
  int err = -1;
  err = tock_digest_set_input(input_buf, input_len);
  if (err < 0) {
    printf("Digest with cert: error %i on set_input\n", err);
    return err;
  }
  err = tock_digest_set_output(output_buf, output_len);
  if (err < 0) {
    printf("Digest with cert: error %i on set_output\n", err);
    return err;
  }

  err = tock_digest_cert_initialize(cert);
  if (err < 0) {
    printf("Digest with cert: error %i on initialize\n", err);
    return err;
  }
  if (input_buf != NULL) {
    err = tock_digest_hash_update(input_len);
    if (err < 0) {
      printf("Digest with cert: error %i on update\n", err);
      return err;
    }
    err = tock_digest_hash_finalize();
    if (err < 0) {
      printf("Digest with cert: error %i on finalize\n", err);
    }
  }

  return err;
}
