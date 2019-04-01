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

# Common Makefile code for all C Tock-on-Titan apps. Included by TockMakefile in
# each app directory. Only implements the "all" target, which builds the
# firmware image -- Build.mk should implement program and run.

APP_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))$(APP)

ELF2TAB := ../../build/cargo-host/release/elf2tab
TOCK_ARCHS := cortex-m3
TOCK_USERLAND_BASE_DIR = $(APP_DIR)/../../third_party/libtock-c
BUILDDIR ?= $(APP_DIR)/../../build/userspace/$(APP)

C_SRCS   := $(wildcard *.c)

OBJS += $(patsubst %.c,$(BUILDDIR)/%.o,$(C_SRCS))

TOCK_APP_CONFIG = -Xlinker --defsym=STACK_SIZE=$$(STACK_SIZE)\
                  -Xlinker --defsym=APP_HEAP_SIZE=$$(APP_HEAP_SIZE)\
                  -Xlinker --defsym=KERNEL_HEAP_SIZE=$$(KERNEL_HEAP_SIZE)

include $(TOCK_USERLAND_BASE_DIR)/AppMakefile.mk

$(BUILDDIR)/%.o: %.c | $(BUILDDIR)
	$(CC) $(CFLAGS) $(CPPFLAGS) -c -o $@ $<
