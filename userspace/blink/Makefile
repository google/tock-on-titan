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
TOCK_USERLAND_BASE_DIR = $(CURRENT_DIR)../../../third_party/libtock-c
BUILDDIR ?= $(CURRENT_DIR)/build/$(TOCK_ARCH)

C_SRCS   := $(wildcard *.c)

OBJS += $(patsubst %.c,$(BUILDDIR)/%.o,$(C_SRCS))

CPPFLAGS += -DSTACK_SIZE=2048

include $(TOCK_USERLAND_BASE_DIR)/AppMakefile.mk

$(BUILDDIR)/%.o: %.c | $(BUILDDIR)
	$(CC) $(CFLAGS) $(CPPFLAGS) -c -o $@ $<


