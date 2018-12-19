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

APP ?= u2f_app
TARGET=thumbv7m-none-eabi
PLATFORM=golf2
TOCK_ARCH=cortex-m3

include ../third_party/tock/boards/Makefile.common

TOCKLOADER = $(TANGO_SPIFLASH)

all: target/$(TARGET)/release/golf2-$(APP)-full

.PHONY: program
program: target/$(TARGET)/release/golf2-$(APP)-full
	$(TANGO_SPIFLASH) --input=$^ --verbose

.PHONY: apps/$(APP)/build/$(TOCK_ARCH)/$(TOCK_ARCH)/$(TOCK_ARCH).bin
apps/$(APP)/build/$(TOCK_ARCH)/$(TOCK_ARCH)/$(TOCK_ARCH).bin:
	make -C apps/$(APP) TOCK_ARCH=$(TOCK_ARCH)

# Note that the .text section should NOT be marked as code, because
# codesigner assumes that the section containing the RW image after
# the bootloader is not marked as code. If you mark .text as code, then
# codesigner looks for the RW image (the Tock kernel) after the kernel,
# can't find information such as entry points, and aborts.
target/$(TARGET)/release/golf2-$(APP): target/$(TARGET)/release/golf2.bin apps/$(APP)/build/$(TOCK_ARCH)/$(TOCK_ARCH)/$(TOCK_ARCH).tbf
	cp target/$(TARGET)/release/golf2.elf target/$(TARGET)/release/golf2-$(APP)
	arm-none-eabi-objcopy --set-section-flags .apps=alloc,code,contents target/$(TARGET)/release/golf2-$(APP)
	arm-none-eabi-objcopy --update-section .apps=apps/$(APP)/build/$(TOCK_ARCH)/$(TOCK_ARCH)/$(TOCK_ARCH).tbf \
	  target/$(TARGET)/release/golf2-$(APP)

target/$(TARGET)/release/golf2-$(APP)-self-signed: target/$(TARGET)/release/golf2-$(APP).elf
	$(TANGO_CODESIGNER) --b --input $^ --key=$(TANGO_CODESIGNER_KEY) --output=$@

target/$(TARGET)/release/golf2-$(APP)-full: target/$(TARGET)/release/golf2-$(APP)-self-signed
	cat $(TANGO_BOOTLOADER) $^ > $@

#flash: target/target/release/golf2-$(APP)-full
#	$(TANGO_SPIFLASH) --input=$^ --verbose
