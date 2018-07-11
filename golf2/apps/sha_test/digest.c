#include "digest.h"
#include "tock.h"

int tock_digest_set_input(void* buf, size_t len) {
  return allow(HOTEL_DRIVER_DIGEST, TOCK_DIGEST_ALLOW_INPUT, buf, len);
}

int tock_digest_set_output(void* buf, size_t len) {
  return allow(HOTEL_DRIVER_DIGEST, TOCK_DIGEST_ALLOW_OUTPUT, buf, len);
}

int tock_digest_hash_initialize(TockDigestMode mode) {
  return command(HOTEL_DRIVER_DIGEST, TOCK_DIGEST_CMD_INITIALIZE, mode, 0);
}

int tock_digest_hash_update(size_t n) {
  return command(HOTEL_DRIVER_DIGEST, TOCK_DIGEST_CMD_UPDATE, n, 0);
}

int tock_digest_hash_finalize() {
  return command(HOTEL_DRIVER_DIGEST, TOCK_DIGEST_CMD_FINALIZE, 0, 0);
}

int tock_digest_hash_easy(void* input_buf, size_t input_len,
                          void* output_buf, size_t output_len, TockDigestMode mode) {
  int ret = -1;
  ret = tock_digest_set_input(input_buf, input_len);
  if (ret < 0) return ret;
  ret = tock_digest_set_output(output_buf, output_len);
  if (ret < 0) return ret;
  ret = tock_digest_hash_initialize(mode);
  if (ret < 0) return ret;
  ret = tock_digest_hash_update(input_len);
  if (ret < 0) return ret;
  return tock_digest_hash_finalize();
}
