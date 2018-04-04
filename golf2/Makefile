APP ?= blink
TOCK_ARCH = cortex-m3
TARGET = thumbv7m-none-eabi
PLATFORM = golf2

include ../tock/boards/Makefile.common

TOCKLOADER = $(TANGO_SPIFLASH)

all: target/target/release/golf2-$(APP)-full

.PHONY: program
program: target/target/release/golf2-$(APP)-full
	$(TANGO_SPIFLASH) --input=$^ --verbose

.PHONY: target/target/release/golf2
target/target/release/golf2:
	cargo build --release --target=../hotel/target.json

.PHONY: apps/$(APP)/build/$(TOCK_ARCH)/app.bin
apps/$(APP)/build/$(TOCK_ARCH)/app.bin:
	make -C apps/$(APP) TOCK_ARCH=$(TOCK_ARCH)

target/target/release/golf2-$(APP): target/target/release/golf2 apps/$(APP)/build/$(TOCK_ARCH)/app.bin
	arm-none-eabi-objcopy --update-section .apps=apps/$(APP)/build/$(TOCK_ARCH)/app.bin \
	  --set-section-flags .apps=alloc,code \
	  target/target/release/golf2 $@

target/target/release/golf2-$(APP)-self-signed: target/target/release/golf2-$(APP)
	$(TANGO_CODESIGNER) --b --input $^ --key=$(TANGO_CODESIGNER_KEY) --output=$@

target/target/release/golf2-$(APP)-full: target/target/release/golf2-$(APP)-self-signed
	cat $(TANGO_BOOTLOADER) $^ > $@

#flash: target/target/release/golf2-$(APP)-full
#	$(TANGO_SPIFLASH) --input=$^ --verbose

