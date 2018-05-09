#include <stdio.h>

#include "gaes.h"
#include "tock.h"

int bytes_encrypted = 0;

static void tock_aes_encrypt_done_cb(int count,
				    int unused2 __attribute__((unused)),
				    int unused3 __attribute__((unused)),
				    void *callback_args) {
  bytes_encrypted = count;
  *(bool *)callback_args = true;  
}

static void tock_aes_mark_true_cb(int unused __attribute__((unused)),
                                  int unused2 __attribute__((unused)),
                                  int unused3 __attribute__((unused)),
                                  void *callback_args) {
  *(bool *)callback_args = true;
}

int tock_aes_setup(void *key, size_t len, int aes_size, int encrypt) {
  int ret = -1;
  ret = allow(HOTEL_DRIVER_AES, TOCK_AES_ALLOW_KEY, key, len);

  bool key_expansion_done = false;
  ret = subscribe(HOTEL_DRIVER_AES, TOCK_AES_DONE_KEY_EXPANSION_INT,
                  tock_aes_mark_true_cb, &key_expansion_done);
  if (ret < 0) return ret;
  ret = command(HOTEL_DRIVER_AES, TOCK_AES_CMD_SETUP, aes_size, 0);
  if (ret < 0) return ret;

  yield_for(&key_expansion_done);

  ret = command(HOTEL_DRIVER_AES, TOCK_AES_CMD_SET_ENCRYPT_MODE, encrypt, 0);
  
  if (ret < 0) return ret;
  return 0;
}

int tock_aes_crypt(void *data, size_t len, void *out, size_t outlen) {
  int ret = -1;
  size_t written_bytes = 0;
  size_t read_bytes = 0;
  bytes_encrypted = 0;
  while (written_bytes < len) {
    ret = allow(HOTEL_DRIVER_AES, TOCK_AES_ALLOW_INPUT, data + written_bytes,
                len - written_bytes);
    if (ret < 0) return ret;
    
    ret = allow(HOTEL_DRIVER_AES, TOCK_AES_ALLOW_OUTPUT, out + written_bytes,
                outlen - written_bytes);
    if (ret < 0) return ret;
	
    bool cipher_done = false;
    ret = subscribe(HOTEL_DRIVER_AES, TOCK_AES_DONE_CIPHER_INT,
                    tock_aes_encrypt_done_cb, &cipher_done);
    if (ret < 0) return ret;

    ret = command(HOTEL_DRIVER_AES, TOCK_AES_CMD_CRYPT, 0, 0);
    if (ret < 0) {
      printf("operation failed: %d\n", ret);
      return ret;
    }
    yield_for(&cipher_done);
    written_bytes += bytes_encrypted;
    read_bytes += bytes_encrypted;
    if (read_bytes == outlen) {
      printf("Output buffer too small to handle encryption\n");
      return -1;
    }
  }
  return read_bytes;
}

int tock_aes_finish() {
  return command(HOTEL_DRIVER_AES, TOCK_AES_CMD_FINISH, 0, 0);
}
