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

CURRENT_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))

TOCK_ARCH ?= cortex-m3

THIRD_PARTY = ../../third_party
TOCK_USERLAND_BASE_DIR = $(THIRD_PARTY)/libtock-c
CHROMIUMOS_DIR         = $(THIRD_PARTY)/chromiumos-ec
LIBGOLF2_DIR           = ../libgolf2

BUILDDIR ?= $(CURRENT_DIR)/build/$(TOCK_ARCH)

STACK_SIZE := 4096
C_SRCS   := $(wildcard *.c)

#OBJS += $(patsubst %.c,$(BUILDDIR)/%.o,$(C_SRCS))

TOCK_APP_CONFIG = -Xlinker --defsym=STACK_SIZE=$$(STACK_SIZE)\
		  -Xlinker --defsym=APP_HEAP_SIZE=$$(APP_HEAP_SIZE)\
		  -Xlinker --defsym=KERNEL_HEAP_SIZE=$$(KERNEL_HEAP_SIZE)


EXTERN_LIBS += $(CHROMIUMOS_DIR) $(LIBGOLF2_DIR)

include $(TOCK_USERLAND_BASE_DIR)/AppMakefile.mk
include $(CHROMIUMOS_DIR)/Makefile
include $(LIBGOLF2_DIR)/Makefile

 override CPPFLAGS += -Wno-shadow -Wno-nested-externs -Wno-unused-parameter
 override CPPFLAGS += -I./include
