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

int tock_digest_hash_update(size_t n) {
  int rval = command(H1B_DRIVER_DIGEST, TOCK_DIGEST_CMD_UPDATE, n, 0);

  return rval;
}

int tock_digest_hash_finalize(void) {
  return command(H1B_DRIVER_DIGEST, TOCK_DIGEST_CMD_FINALIZE, 0, 0);
}

int tock_digest_hash_easy(void* input_buf, size_t input_len,
                          void* output_buf, size_t output_len, TockDigestMode mode) {
  int ret = -1;
  ret = tock_digest_set_input(input_buf, input_len);
  if (ret < 0) {
    printf("Digest: error %i on set_input\n", ret);
    return ret;
  }
  ret = tock_digest_set_output(output_buf, output_len);
  if (ret < 0) {
    printf("Digest: error %i on set_output\n", ret);
    return ret;
  }
  ret = tock_digest_hash_initialize(mode);
  if (ret < 0) {
    printf("Digest: error %i on initialize\n", ret);
    return ret;
  }
  ret = tock_digest_hash_update(input_len);
  if (ret < 0) {
    printf("Digest: error %i on update\n", ret);
    return ret;
  }
  return tock_digest_hash_finalize();
}
