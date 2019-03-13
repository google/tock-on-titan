# Copyright 2018 Google LLC
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

# Makefile for the userspace libgolf2 library. This includes the C code 
# for making system calls to H1B peripherals that the golf2 kernel
# supports.

TOCK_USERLAND_BASE_DIR ?= $(THIRD_PARTY)/libtock-c/libtock
LIBNAME := libgolf2
$(LIBNAME)_DIR := ../$(LIBNAME)

$(LIBNAME)_SRCS := $($(LIBNAME)_DIR)/h1b_aes_syscalls.c      \
                   $($(LIBNAME)_DIR)/dcrypto_syscalls.c  \
                   $($(LIBNAME)_DIR)/digest_syscalls.c   \
                   $($(LIBNAME)_DIR)/u2f_syscalls.c

include $(TOCK_USERLAND_BASE_DIR)/TockLibrary.mk
