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

APP ?= blink
TOCK_ARCH = cortex-m3

all: target/target/release/golf-$(APP)

.PHONY: target/target/release/golf
target/target/release/golf:
	@cargo build --release --target=../h1b/target.json

.PHONY: apps/$(APP)/build/$(TOCK_ARCH)/app.bin
apps/$(APP)/build/$(TOCK_ARCH)/app.bin:
	@make -C apps/$(APP) TOCK_ARCH=$(TOCK_ARCH)

target/target/release/golf-$(APP): target/target/release/golf apps/$(APP)/build/$(TOCK_ARCH)/app.bin
	@arm-none-eabi-objcopy --update-section .apps=apps/$(APP)/build/$(TOCK_ARCH)/app.bin \
	  --set-section-flags .apps=alloc,code \
	  target/target/release/golf $@

target/target/release/golf-$(APP)-self-signed: target/target/release/golf-$(APP)
	$(TANGO_CODESIGNER) --input $^ --key=$(TANGO_CODESIGNER_KEY) --output=$@

target/target/release/golf-$(APP)-full: target/target/release/golf-$(APP)-self-signed
	@cat $(TANGO_BOOTLOADER) $^ > $@

flash: target/target/release/golf-$(APP)-full
	@$(TANGO_SPIFLASH) --input=$^ --verbose

.PHONY: doc
doc:
	@cargo doc --target=../h1b/target.json

.PHONY: clean
clean:
	@cargo clean
	@make -C apps/$(APP) clean

