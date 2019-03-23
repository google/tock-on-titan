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

APP := u2f_app
STACK_SIZE := 4096

THIRD_PARTY    = ../../third_party
CHROMIUMOS_DIR = $(THIRD_PARTY)/chromiumos-ec
LIBGOLF2_DIR   = ../libgolf2

EXTERN_LIBS += $(CHROMIUMOS_DIR) $(LIBGOLF2_DIR)

include ../CAppMakefile.mk
include $(CHROMIUMOS_DIR)/Makefile
include $(LIBGOLF2_DIR)/Makefile

override CPPFLAGS += -Wno-shadow -Wno-nested-externs -Wno-unused-parameter
override CPPFLAGS += -I./include
