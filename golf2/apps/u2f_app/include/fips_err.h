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

#ifndef __FIPS_ERR_H__
#define __FIPS_ERR_H__

/* Signals start on the left, errors on the right. */
enum fips_err {
  FIPS_INITIALIZED = 1 << 31,
  FIPS_SUCCESS = 0,
  FIPS_UNINITIALIZED = 1,
  FIPS_FATAL_TRNG = 1 << 1,
  FIPS_FATAL_HMAC_SHA256 = 1 << 2,
  FIPS_FATAL_HMAC_DRBG = 1 << 3,
  FIPS_FATAL_ECDSA = 1 << 4,
  FIPS_FATAL_TOO_MANY_KEYS = 1 << 5,
  FIPS_FATAL_AES128 = 1 << 6,
  FIPS_FATAL_CMAC_AES128 = 1 << 7,
  FIPS_ERROR_MASK = 0xfe,
  FIPS_RFU_MASK = 0x7fffff00,
};

extern int fips_fatal;
extern int fips_fatal_lineno;

//#ifdef CR50_DEV || 1
#if 1
#define throw_fips_err(x)                                        \
  {                                                              \
    printf("%s:%d fips err 0x%08x\n", __FILE__, __LINE__, (x)); \
    if (!fips_fatal_lineno) fips_fatal_lineno = __LINE__;        \
    _throw_fips_err(x);                                          \
  }
#else
#define throw_fips_err(x)                                 \
  {                                                       \
    if (!fips_fatal_lineno) fips_fatal_lineno = __LINE__; \
    _throw_fips_err(x);                                   \
  }
#endif

void _throw_fips_err(enum fips_err);

#endif
