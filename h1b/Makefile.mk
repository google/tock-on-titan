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

ARCH = cortex-m3

RUST_TARGET ?= $(SRC_DIR)chips/h1b/target.json

RUSTC_FLAGS += -C opt-level=3 -Z no-landing-pads
RUSTC_FLAGS += --target $(RUST_TARGET)
RUSTC_FLAGS += -Ctarget-cpu=$(ARCH) -C relocation_model=static
RUSTC_FLAGS += -C no-stack-check -C soft-float -C target-feature="+soft-float"

CFLAGS_BASE = -mcpu=$(ARCH) -mthumb -mfloat-abi=soft
CFLAGS += $(CFLAGS_BASE) -O3 -nostartfiles
LOADER = $(SRC_DIR)chips/h1b/layout.ld
LDFLAGS += -T$(LOADER) -lm
OBJDUMP_FLAGS := --disassemble --source --disassembler-options=force-thumb
OBJDUMP_FLAGS += -C --section-headers

$(BUILD_PLATFORM_DIR)/libh1b.rlib: $(call rwildcard,$(SRC_DIR)chips/h1b/src,*.rs) $(BUILD_PLATFORM_DIR)/libcortexm3.rlib $(BUILD_PLATFORM_DIR)/libcore.rlib $(BUILD_PLATFORM_DIR)/libhil.rlib $(BUILD_PLATFORM_DIR)/libcommon.rlib $(BUILD_PLATFORM_DIR)/libmain.rlib | $(BUILD_PLATFORM_DIR)
	@echo "Building $@"
	@$(RUSTC) $(RUSTC_FLAGS) --out-dir $(BUILD_PLATFORM_DIR) $(SRC_DIR)chips/h1b/src/lib.rs

