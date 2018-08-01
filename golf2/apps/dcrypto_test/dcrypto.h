#ifndef TOCK_DCRYPTO_H
#define TOCK_DCRYPTO_H

#include <stdlib.h>

#define HOTEL_DRIVER_DCRYPTO 0x40004

#define TOCK_DCRYPTO_CMD_CHECK 0
#define TOCK_DCRYPTO_CMD_RUN   1

#define TOCK_DCRYPTO_ALLOW_DATA 0
#define TOCK_DCRYPTO_ALLOW_PROG 1

#define TOCK_DCRYPTO_RUN_DONE 0


// Run the program pointed to by program with the data pointed to by
// data as data memory. The lengths are in bytes, but only whole
// 4-byte words are used: partial words are not used. For example,
// calling tock_dcrypto_run with a datalen of 11 will result in only 8
// bytes of data being copied in and out from dcrypto memory, while
// calling it with a datalen of 12 will result in 12 bytes being
// copied in/out.
//
// While the function does not accept partial words, it does not assume
// alignment: data and program do not have to be word-aligned.
int tock_dcrypto_run(void* data, size_t datalen,
		     void* program, size_t programlen);

#endif
