APP ?= dcrypto_test
TOCK_ARCH = cortex-m3
TARGET = thumbv7m-none-eabi
PLATFORM = golf2

include ../tock/boards/Makefile.common

TOCKLOADER = $(TANGO_SPIFLASH)

all: target/$(TARGET)/release/golf2-$(APP)-full

.PHONY: program
program: target/$(TARGET)/release/golf2-$(APP)-full
	$(TANGO_SPIFLASH) --input=$^ --verbose

.PHONY: apps/$(APP)/build/$(TOCK_ARCH)/$(TOCK_ARCH)/$(TOCK_ARCH).bin
apps/$(APP)/build/$(TOCK_ARCH)/$(TOCK_ARCH)/$(TOCK_ARCH).bin:
	make -C apps/$(APP) TOCK_ARCH=$(TOCK_ARCH)


target/$(TARGET)/release/golf2-$(APP): target/$(TARGET)/release/golf2.bin apps/$(APP)/build/$(TOCK_ARCH)/$(TOCK_ARCH)/$(TOCK_ARCH).bin
	arm-none-eabi-objcopy --update-section .apps=apps/$(APP)/build/$(TOCK_ARCH)/$(TOCK_ARCH)/$(TOCK_ARCH).bin \
	  --set-section-flags .apps=alloc,code \
	  target/$(TARGET)/release/golf2 $@

target/$(TARGET)/release/golf2-$(APP)-self-signed: target/$(TARGET)/release/golf2-$(APP)
	$(TANGO_CODESIGNER) --b --input $^ --key=$(TANGO_CODESIGNER_KEY) --output=$@

target/$(TARGET)/release/golf2-$(APP)-full: target/$(TARGET)/release/golf2-$(APP)-self-signed
	cat $(TANGO_BOOTLOADER) $^ > $@

#flash: target/target/release/golf2-$(APP)-full
#	$(TANGO_SPIFLASH) --input=$^ --verbose

