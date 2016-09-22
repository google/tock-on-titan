#ifndef TOCK_DIGEST_H
#define TOCK_DIGEST_H

#include <stdlib.h>

// TODO These need to be standardized
#define HOTEL_DRIVER_DIGEST 2

// command() type ids
#define TOCK_DIGEST_CMD_INITIALIZE 0
#define TOCK_DIGEST_CMD_UPDATE 1
#define TOCK_DIGEST_CMD_FINALIZE 2

// allow() type ids
#define TOCK_DIGEST_ALLOW_INPUT 0
#define TOCK_DIGEST_ALLOW_OUTPUT 1

typedef enum TockDigestMode {
  DIGEST_MODE_SHA1 = 0,
  DIGEST_MODE_SHA256 = 1,
} TockDigestMode;

// TODO: Should be const, but currently not allowed by kernel
int tock_digest_set_input(void* buf, size_t len);
int tock_digest_set_output(void* buf, size_t len);

int tock_digest_hash_initialize(TockDigestMode mode);
int tock_digest_hash_update(size_t n);
int tock_digest_hash_finalize();

int tock_digest_hash_easy(void* input_buf, size_t input_len,
                          void* output_buf, size_t output_len, TockDigestMode mode);

#endif // TOCK_DIGEST_H
