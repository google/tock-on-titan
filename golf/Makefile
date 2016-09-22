APP ?= blink
TOCK_ARCH = cortex-m3

all: target/target/release/golf-$(APP)-full

.PHONY: target/target/release/golf
target/target/release/golf:
	@cargo build --release --target=../hotel/target.json

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
	@cargo doc --target=../hotel/target.json

.PHONY: clean
clean:
	@cargo clean
	@make -C apps/$(APP) clean

