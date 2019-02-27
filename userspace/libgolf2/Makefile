# Makefile for chromiumos-ec.a library. This includes code under third
# party licenses.

TOCK_USERLAND_BASE_DIR ?= $(THIRD_PARTY)/libtock-c/libtock
LIBNAME := libgolf2
$(LIBNAME)_DIR := ../$(LIBNAME)

$(LIBNAME)_SRCS := $($(LIBNAME)_DIR)/aes_syscalls.c      \
                   $($(LIBNAME)_DIR)/dcrypto_syscalls.c  \
                   $($(LIBNAME)_DIR)/digest_syscalls.c   \
                   $($(LIBNAME)_DIR)/u2f_syscalls.c

include $(TOCK_USERLAND_BASE_DIR)/TockLibrary.mk
