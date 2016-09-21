CHIP=hotel

TANGO_CODESIGNER ?= codesigner
TANGO_CODESIGNER_KEY ?= loader-testkey-A.pem
TANGO_BOOTLOADER ?= bootloader.hex
TANGO_SPIFLASH ?= spiflash

PLATFORM_DEPS := $(BUILD_PLATFORM_DIR)/libcore.rlib $(BUILD_PLATFORM_DIR)/libsupport.rlib
PLATFORM_DEPS += $(BUILD_PLATFORM_DIR)/libhil.rlib $(BUILD_PLATFORM_DIR)/libdrivers.rlib
PLATFORM_DEPS += $(BUILD_PLATFORM_DIR)/libmain.rlib


$(BUILD_PLATFORM_DIR)/kernel.o: $(call rwildcard,$(SRC_DIR)platform/golf/src,*.rs) $(BUILD_PLATFORM_DIR)/libhotel.rlib $(PLATFORM_DEPS) | $(BUILD_PLATFORM_DIR)
	@echo "Building $@"
	@$(RUSTC) $(RUSTC_FLAGS) -C lto --emit obj -o $@ $(SRC_DIR)platform/golf/src/main.rs
	@$(OBJDUMP) $(OBJDUMP_FLAGS) $@ > $(BUILD_PLATFORM_DIR)/kernel.lst

$(BUILD_PLATFORM_DIR)/kernel.elf: $(BUILD_PLATFORM_DIR)/kernel.o | $(BUILD_PLATFORM_DIR)
	@tput bold ; echo "Linking $@" ; tput sgr0
	@$(CC) $(CFLAGS) -Wl,-gc-sections $^ $(LDFLAGS) -Wl,-Map=$(BUILD_PLATFORM_DIR)/kernel.Map -o $@
	@$(OBJDUMP) $(OBJDUMP_FLAGS) $@ > $(BUILD_PLATFORM_DIR)/kernel_post-link.lst
	@$(SIZE) $@

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

