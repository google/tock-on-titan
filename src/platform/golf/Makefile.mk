CHIP=hotel

TANGO_CODESIGNER ?= codesigner
TANGO_CODESIGNER_KEY ?= loader-testkey-A.pem
TANGO_BOOTLOADER ?= bootloader.hex
TANGO_SPIFLASH ?= spiflash

$(BUILD_PLATFORM_DIR)/libplatform.rlib: $(call rwildcard,$(SRC_DIR)platform/$(TOCK_PLATFORM),*.rs) $(BUILD_PLATFORM_DIR)/libcore.rlib $(BUILD_PLATFORM_DIR)/libhil.rlib $(BUILD_PLATFORM_DIR)/lib$(CHIP).rlib $(BUILD_PLATFORM_DIR)/libdrivers.rlib | $(BUILD_PLATFORM_DIR)
	@echo "Building $@"
	@$(RUSTC) $(RUSTC_FLAGS) --out-dir $(BUILD_PLATFORM_DIR) $(SRC_DIR)platform/$(TOCK_PLATFORM)/lib.rs

$(BUILD_PLATFORM_DIR)/self_signed_kernel.hex: $(BUILD_PLATFORM_DIR)/kernel.elf
	@echo "Self signing $<"
	@$(TANGO_CODESIGNER) -k $(TANGO_CODESIGNER_KEY) -i $< -o $@

$(BUILD_PLATFORM_DIR)/kernel.hex: $(BUILD_PLATFORM_DIR)/self_signed_kernel.hex $(TANGO_BOOTLOADER)
	@echo "Concatenating bootloader"
	@cat $^ > $@

.PHONE: flash
flash: $(BUILD_PLATFORM_DIR)/kernel.hex
	@echo "Flashing $<"
	@$(TANGO_SPIFLASH) -i $<

all: $(BUILD_PLATFORM_DIR)/kernel.hex

