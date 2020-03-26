# README for userspace/libh1

This directory contains the libh1 library for the Tock operating
system, which provides system calls for h1 specific device
drivers. These system calls were initially designed to be the minimal
set required to support a U2F application, so do not implement the
full capabilities of the drivers.

The userspace library functions can be found in the associated header files.
This document documents the underlying system calls.

## DCRYPTO (0x40004)

dcrypto is the bignum accelerator on H1. It has its own assembly
language. It implements two allows:
  * 0: data, a buffer containing data input/output
  * 1: program: a buffer containing assembly instructions to execute

It implements two commands:
  * 0: check(_, _)
  * 1: run(address, _), where address is the instruction in the code block at which to start execution.

It implements one callback:
  * 0: run_done(error, fault, _), where `error` is the return code; if it is not `TOCK_SUCCESS`, then `fault` contains a dcrypto-specific error code.

## DIGEST (0x40003)

The digest (SHA) engine on H1 has some additional functionality for
computing HMAC as well computing entries in its hidden keyladder. It
implements two allows:
  * 0: input, a buffer containing input for the hash operation
  * 1: output: a buffer for the resulting hash

It implements 6 commands:
  * 0: check(?, ?), check if driver present
  * 1: initialize(mode, ?), initialize the hash engine into a hash mode (SHA1=0, SHA256=1, SHA256_HMAC=2)
  * 2: update(len, ?), update the hash with n bytes from input buffer
  * 3: finalize(?, ?), finalize the hash into the output buffer
  * 4: busy(?, ?), check if the hash engine is busy
  * 5: certificate_initialize(cert, ?): initialize hash with certificate `cert`

## H1_AES (0x40010)

The AES engine implements a different syscall API than standard Tock
because its hardware is quite different. It offers the same library
calls, plus a few additional ones (e.g., for ECB mode, required for
FIPS). It supports both AES128 and AES256 and encrypts/decrypts a
single block at a time.  Which one to perform is specified by the size
of the input and output buffers. It implements three allows:
  * 0: key
  * 1: input
  * 3: IV or CTR, the initialization vector (for CBC mode) or counter (for CTR mode)

It implements 7 commands. The commands take no parameters: length is defined
by the input and output buffers.
  * 0: check
  * 1: ecb_encrypt: encrypt in ECB mode
  * 2: ecb_decrypt: decrypt in ECB mode
  * 3: ctr_encrypt: encrypt in CTR mode
  * 4: ctr_decrypt: decrypt in CTR mode
  * 5: cbc_encrypt: encrypt in CBC mode
  * 6: cbc_decrypt: decrypt in CBC mode

It provides a single callback:
  * 0: crypt_done(type), where type=1 for encryption and type=2 for decryption

## U2F (0x20008)

The U2F driver implements data transport over USB endpoint 1 (EP1). It
implements two allows:
  * 1: transmit
  * 2: receive

It implements three commands:
  * 0: check
  * 1: transmit(len, ?)
  * 2: receive(len, &)

It provides three callbacks:
  * 1: transmit_done: the buffer passed via allow was transmitted
  * 2: receive: the buffer passed via receive was received into
  * 3: reconnect: the device reconnected
