# Makefile for chromiumos-ec.a library. This includes code under third
# party licenses.

TOCK_USERLAND_BASE_DIR ?= ../libtock-c/libtock
LIBNAME := chromiumos-ec
$(LIBNAME)_DIR := $(THIRD_PARTY)/$(LIBNAME)

$(LIBNAME)_SRCS := $($(LIBNAME)_DIR)/sha256.c

include $(TOCK_USERLAND_BASE_DIR)/TockLibrary.mk
