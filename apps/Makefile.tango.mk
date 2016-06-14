CHIP := hotel
ARCH := cortex-m3
TOCK_PLATFORM_LINKER_SCRIPT = $(TOCK_DIR)/chips/hotel/linker.ld

TANGO_CODESIGNER ?= codesigner
TANGO_CODESIGNER_KEY ?= loader-testkey-A.pem
TANGO_BOOTLOADER ?= bootloader.hex
TANGO_SPIFLASH ?= spiflash


include $(TOCK_APPS_DIR)/Makefile.Arm-M.mk


# Apps to link may grow over time so defer expanding that
.SECONDEXPANSION:
$(TOCK_APP_BUILD_DIR)/kernel_and_app.elf: $(TOCK_BUILD_DIR)/ctx_switch.o $(TOCK_BUILD_DIR)/kernel.o $$(APPS_TO_LINK_TO_KERNEL) | $(TOCK_BUILD_DIR)
	@tput bold ; echo "Linking $@" ; tput sgr0
	$(Q)$(CC) $(CFLAGS) $(CPPFLAGS) $^ $(LDFLAGS) -Wl,-Map=$(TOCK_APP_BUILD_DIR)/kernel_and_app.Map -o $@
	$(Q)$(GENLST) $@ > $(TOCK_APP_BUILD_DIR)/kernel_and_app.lst
	$(Q)$(SIZE) $@

$(TOCK_APP_BUILD_DIR)/self_signed_kernel.hex: $(TOCK_APP_BUILD_DIR)/kernel_and_app.elf
	@echo "Self signing $<"
	@$(TANGO_CODESIGNER) -k $(TANGO_CODESIGNER_KEY) -i $< -o $@

$(TOCK_APP_BUILD_DIR)/kernel_and_app.hex: $(TOCK_APP_BUILD_DIR)/self_signed_kernel.hex $(TANGO_BOOTLOADER)
	@echo "Concatenating bootloader"
	@cat $^ > $@

.PHONE: flash
flash: $(TOCK_APP_BUILD_DIR)/kernel_and_app.hex
	@echo "Flashing $<"
	@$(TANGO_SPIFLASH) -i $<

all: $(TOCK_APP_BUILD_DIR)/kernel_and_app.elf
	@tput bold ; echo "Finished building $(APP) for $(TOCK_PLATFORM)" ; tput sgr0
