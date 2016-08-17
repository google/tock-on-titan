$(BUILD_PLATFORM_DIR)/ctx_switch.o: $(SRC_DIR)arch/$(ARCH)/ctx_switch.S | $(BUILD_PLATFORM_DIR)
	@$(TOOLCHAIN)as -mcpu=cortex-m3 -mthumb $^ -o $@

$(BUILD_PLATFORM_DIR)/libcortexm3.rlib: $(call rwildcard,$(SRC_DIR)arch/cortex-m3,*.rs) $(BUILD_PLATFORM_DIR)/libcore.rlib $(BUILD_PLATFORM_DIR)/libcommon.rlib $(BUILD_PLATFORM_DIR)/libmain.rlib | $(BUILD_PLATFORM_DIR)
	@echo "Building $@"
	@$(RUSTC) $(RUSTC_FLAGS) --out-dir $(BUILD_PLATFORM_DIR) $(SRC_DIR)arch/cortex-m3/lib.rs

