$(BUILD_PLATFORM_DIR)/libcortexm3.rlib: $(call rwildcard,$(SRC_DIR)arch/cortex-m3/src,*.rs) $(BUILD_PLATFORM_DIR)/libcore.rlib $(BUILD_PLATFORM_DIR)/libcommon.rlib $(BUILD_PLATFORM_DIR)/libmain.rlib | $(BUILD_PLATFORM_DIR)
	@echo "Building $@"
	@$(RUSTC) $(RUSTC_FLAGS) --out-dir $(BUILD_PLATFORM_DIR) $(SRC_DIR)arch/cortex-m3/src/lib.rs

