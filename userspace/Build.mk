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

BUILD_SUBDIRS := $(addprefix userspace/,                   \
					 aes_test          \
					 blink             \
					 dcrypto_test      \
					 flash_test        \
					 gpio_test         \
					 low_level_debug   \
					 nvcounter_ctest   \
					 nvcounter_test    \
					 personality_clear \
					 personality_test  \
					 rng               \
					 sha_test          \
					 spin              \
					 u2f_app           \
					 u2f_test )

.PHONY: userspace/build
userspace/build: $(addsuffix /build,$(BUILD_SUBDIRS))

.PHONY: userspace/check
userspace/check:
	cd userspace && TOCK_KERNEL_VERSION=h1b_tests cargo check --release

.PHONY: userspace/clean
userspace/clean:
	rm -f userspace/Cargo.lock

.PHONY: userspace/devicetests
userspace/devicetests: $(addsuffix /devicetests,$(BUILD_SUBDIRS))

.PHONY: userspace/doc
userspace/doc:
	cd userspace && TOCK_KERNEL_VERSION=h1b cargo doc --release

.PHONY: userspace/localtests
userspace/localtests: $(addsuffix /localtests,$(BUILD_SUBDIRS))

include $(addsuffix /Build.mk,$(BUILD_SUBDIRS))

# ------------------------------------------------------------------------------
# Build rules shared between the C apps.
# ------------------------------------------------------------------------------

# Each of these build rules exists once for each C app. These rules are static
# pattern rules, documented here:
# https://www.gnu.org/software/make/manual/html_node/Static-Pattern.html.
#
# E.g. if C_APPS is "userspace/app1 userspace/app2 userspace/app3" then the
# /build targets will expand as follows:
#   .PHONY: userspace/app1/build userspace/app2/build userspace/app3/build
#   userspace/app1/build: build/userspace/app1/full_image
#   userspace/app2/build: build/userspace/app2/full_image
#   userspace/app3/build: build/userspace/app3/full_image

.PHONY: $(foreach APP,$(C_APPS),userspace/$(APP)/build)
$(foreach APP,$(C_APPS),userspace/$(APP)/build): userspace/%/build: \
		build/userspace/%/full_image

.PHONY: $(foreach APP,$(C_APPS),userspace/$(APP)/check)
$(foreach APP,$(C_APPS),userspace/$(APP)/check):

.PHONY: $(foreach APP,$(C_APPS),userspace/$(APP)/devicetests)
$(foreach APP,$(C_APPS),userspace/$(APP)/devicetests):

.PHONY: $(foreach APP,$(C_APPS),userspace/$(APP)/doc)
$(foreach APP,$(C_APPS),userspace/$(APP)/doc):

.PHONY: $(foreach APP,$(C_APPS),userspace/$(APP)/localtests)
$(foreach APP,$(C_APPS),userspace/$(APP)/localtests):

.PHONY: $(foreach APP,$(C_APPS),userspace/$(APP)/program)
$(foreach APP,$(C_APPS),userspace/$(APP)/program): userspace/%/program: \
		build/userspace/%/full_image
	flock build/device_lock $(TANGO_SPIFLASH) --verbose \
		--input=build/userspace/$*/full_image

.PHONY: $(foreach APP,$(C_APPS),userspace/$(APP)/run)
$(foreach APP,$(C_APPS),userspace/$(APP)/run): userspace/%/run: \
		build/cargo-host/release/runner build/userspace/%/full_image
	flock build/device_lock -c ' \
		$(TANGO_SPIFLASH) --verbose \
			--input=build/userspace/$*/full_image ; \
		stty -F /dev/ttyUltraConsole3 115200 -echo ; \
		stty -F /dev/ttyUltraTarget2 115200 -icrnl ; \
		build/cargo-host/release/runner'

.PHONY: $(foreach APP,$(C_APPS),build/userspace/$(APP)/cortex-m3/cortex-m3.tbf)
$(foreach APP,$(C_APPS),build/userspace/$(APP)/cortex-m3/cortex-m3.tbf): \
		build/userspace/%/cortex-m3/cortex-m3.tbf: \
		build/cargo-host/release/elf2tab
	$(MAKE) -C userspace/$* -f TockMakefile

$(foreach APP,$(C_APPS),build/userspace/$(APP)/full_image): \
		build/userspace/%/full_image: \
		build/userspace/%/cortex-m3/cortex-m3.tbf \
		golf2/target/thumbv7m-none-eabi/release/golf2
	cp golf2/target/thumbv7m-none-eabi/release/golf2 \
		build/userspace/$*/unsigned_image
	arm-none-eabi-objcopy --set-section-flags .apps=alloc,code,contents \
		build/userspace/$*/unsigned_image
	arm-none-eabi-objcopy --update-section \
		.apps=build/userspace/$*/cortex-m3/cortex-m3.tbf \
		build/userspace/$*/unsigned_image
	$(TANGO_CODESIGNER) --b --input build/userspace/$*/unsigned_image \
		--key=$(TANGO_CODESIGNER_KEY) \
		--output=build/userspace/$*/signed_image
	cat $(TANGO_BOOTLOADER) build/userspace/$*/signed_image \
		> build/userspace/$*/full_image

# ------------------------------------------------------------------------------
# Build rules shared between Rust app targets. These are not used for Rust test
# apps, which are handled below.
# ------------------------------------------------------------------------------

# These are static pattern rules, see above (the C apps rules) section for an
# explanation.

.PHONY: $(foreach APP,$(RUST_APPS),userspace/$(APP)/build)
$(foreach APP,$(RUST_APPS),userspace/$(APP)/build): userspace/%/build: \
		build/userspace/%/full_image

.PHONY: $(foreach APP,$(RUST_APPS),userspace/$(APP)/check)
$(foreach APP,$(RUST_APPS),userspace/$(APP)/check): userspace/%/check:
	cd userspace/$* && TOCK_KERNEL_VERSION=$* cargo check \
		--offline --release

.PHONY: $(foreach APP,$(RUST_APPS),userspace/$(APP)/devicetests)
$(foreach APP,$(RUST_APPS),userspace/$(APP)/devicetests):

.PHONY: $(foreach APP,$(RUST_APPS),userspace/$(APP)/doc)
$(foreach APP,$(RUST_APPS),userspace/$(APP)/doc): userspace/%/doc:
	cd userspace/$* && TOCK_KERNEL_VERSION=$* cargo doc --offline --release

.PHONY: $(foreach APP,$(RUST_APPS),userspace/$(APP)/localtests)
$(foreach APP,$(RUST_APPS),userspace/$(APP)/localtests):

.PHONY: $(foreach APP,$(RUST_APPS),userspace/$(APP)/program)
$(foreach APP,$(RUST_APPS),userspace/$(APP)/program): userspace/%/program: \
		build/userspace/%/full_image
	flock build/device_lock -c '$(TANGO_SPIFLASH) --verbose \
		--input=build/userspace/$*/full_image'

.PHONY: $(foreach APP,$(RUST_APPS),userspace/$(APP)/run)
$(foreach APP,$(RUST_APPS),userspace/$(APP)/run): userspace/%/run: \
		build/cargo-host/release/runner build/userspace/%/full_image
	flock build/device_lock -c ' \
		$(TANGO_SPIFLASH) --verbose \
				  --input=build/userspace/$*/full_image ; \
		stty -F /dev/ttyUltraConsole3 115200 -echo ; \
		stty -F /dev/ttyUltraTarget2 115200 -icrnl ; \
		build/cargo-host/release/runner'

.PHONY: $(foreach APP,$(RUST_APPS),build/userspace/$(APP)/app)
$(foreach APP,$(RUST_APPS),build/userspace/$(APP)/app): build/userspace/%/app:
	rm -f build/userspace/cargo/thumbv7m-none-eabi/release/$*-*
	cd userspace/$* && TOCK_KERNEL_VERSION=$* cargo build --offline --release
	mkdir -p build/userspace/$*/
	cp "build/userspace/cargo/thumbv7m-none-eabi/release/$*" \
		"build/userspace/$*/app"

# Due to b/139156455, we want to detect the case where an application's size is
# rounded up to 64 KiB.
$(foreach APP,$(RUST_APPS),build/userspace/$(APP)/app.tbf): \
		build/userspace/%/app.tbf: \
		build/cargo-host/release/elf2tab build/userspace/%/app
	build/cargo-host/release/elf2tab -n "$(notdir $*)" \
		-o build/userspace/$*/app_tab \
		build/userspace/$*/app --stack=2048 --app-heap=1024 \
		--kernel-heap=1024 --protected-region-size=64
	if [ "$$(wc -c <build/userspace/$*/app.tbf)" -ge 65536 ]; \
		then echo "#########################################################"; \
		     echo "# Application $(notdir $*) is too large."; \
		     echo "# Check size of build/userspace/$*/app.tbf"; \
		     echo "#########################################################"; \
		     false ; \
		fi

$(foreach APP,$(RUST_APPS),build/userspace/$(APP)/full_image): \
		build/userspace/%/full_image: build/userspace/%/app.tbf \
		golf2/target/thumbv7m-none-eabi/release/golf2 ;
	cp golf2/target/thumbv7m-none-eabi/release/golf2 \
		build/userspace/$*/unsigned_image
	arm-none-eabi-objcopy --set-section-flags .apps=alloc,code,contents \
		build/userspace/$*/unsigned_image
	arm-none-eabi-objcopy --update-section \
		.apps=build/userspace/$*/app.tbf \
		build/userspace/$*/unsigned_image
	$(TANGO_CODESIGNER) --b --input build/userspace/$*/unsigned_image \
		--key=$(TANGO_CODESIGNER_KEY) \
		--output=build/userspace/$*/signed_image
	cat $(TANGO_BOOTLOADER) build/userspace/$*/signed_image \
		> build/userspace/$*/full_image

# ------------------------------------------------------------------------------
# Build rules shared between Rust test app targets.
# ------------------------------------------------------------------------------

# These are static pattern rules, see above (the C apps rules) section for an
# explanation.

.PHONY: $(foreach APP,$(RUST_TESTS),userspace/$(APP)/build)
$(foreach APP,$(RUST_TESTS),userspace/$(APP)/build): userspace/%/build: \
		build/userspace/%/full_image

.PHONY: $(foreach APP,$(RUST_TESTS),userspace/$(APP)/check)
$(foreach APP,$(RUST_TESTS),userspace/$(APP)/check): userspace/%/check:
	cd userspace/$* && TOCK_KERNEL_VERSION=$* cargo check \
		--offline --release

.PHONY: $(foreach APP,$(RUST_TESTS),userspace/$(APP)/devicetests)
$(foreach APP,$(RUST_TESTS),userspace/$(APP)/devicetests): \
	userspace/%/devicetests: \
		build/cargo-host/release/runner build/userspace/%/full_image
	flock build/device_lock -c ' \
		$(TANGO_SPIFLASH) --verbose \
				  --input=build/userspace/$*/full_image ; \
		stty -F /dev/ttyUltraConsole3 115200 -echo ; \
		stty -F /dev/ttyUltraTarget2 115200 -icrnl ; \
		build/cargo-host/release/runner --test'

.PHONY: $(foreach APP,$(RUST_TESTS),userspace/$(APP)/doc)
$(foreach APP,$(RUST_TESTS),userspace/$(APP)/doc): userspace/%/doc:
	cd userspace/$* && TOCK_KERNEL_VERSION=$* cargo doc --offline --release

.PHONY: $(foreach APP,$(RUST_TESTS),userspace/$(APP)/localtests)
$(foreach APP,$(RUST_TESTS),userspace/$(APP)/localtests):

.PHONY: $(foreach APP,$(RUST_TESTS),userspace/$(APP)/program)
$(foreach APP,$(RUST_TESTS),userspace/$(APP)/program): userspace/%/program: \
		build/userspace/%/full_image
	flock build/device_lock -c '$(TANGO_SPIFLASH) --verbose \
		--input=build/userspace/$*/full_image'

.PHONY: $(foreach APP,$(RUST_TESTS),userspace/$(APP)/run)
$(foreach APP,$(RUST_TESTS),userspace/$(APP)/run): userspace/%/run: \
		build/cargo-host/release/runner build/userspace/%/full_image
	flock build/device_lock -c ' \
		$(TANGO_SPIFLASH) --verbose \
				  --input=build/userspace/$*/full_image ; \
		stty -F /dev/ttyUltraConsole3 115200 -echo ; \
		stty -F /dev/ttyUltraTarget2 115200 -icrnl ; \
		build/cargo-host/release/runner'

.PHONY: $(foreach APP,$(RUST_TESTS),build/userspace/$(APP)/app)
$(foreach APP,$(RUST_TESTS),build/userspace/$(APP)/app): build/userspace/%/app:
	rm -f build/userspace/cargo/thumbv7m-none-eabi/release/$*-*
	cd userspace/$* && TOCK_KERNEL_VERSION=$* \
		cargo test --no-run --offline --release
	mkdir -p build/userspace/$*/
	find build/userspace/cargo/thumbv7m-none-eabi/release/ -maxdepth 1 -regex \
		'build/userspace/cargo/thumbv7m-none-eabi/release/$*-[^.]+' \
		-exec cp '{}' build/userspace/$*/app ';'

# Due to b/139156455, we want to detect the case where an application's size is
# rounded up to 64 KiB.
$(foreach APP,$(RUST_TESTS),build/userspace/$(APP)/app.tbf): \
		build/userspace/%/app.tbf: \
		build/cargo-host/release/elf2tab build/userspace/%/app
	build/cargo-host/release/elf2tab -n "$(notdir $*)" \
		-o build/userspace/$*/app_tab \
		build/userspace/$*/app --stack=2048 --app-heap=1024 \
		--kernel-heap=1024 --protected-region-size=64
	if [ "$$(wc -c <build/userspace/$*/app.tbf)" -ge 65536 ]; \
		then echo "#########################################################"; \
		     echo "# Application $(notdir $*) is too large."; \
		     echo "# Check size of build/userspace/$*/app.tbf"; \
		     echo "#########################################################"; \
		     false ; \
		fi

$(foreach APP,$(RUST_TESTS),build/userspace/$(APP)/full_image): \
		build/userspace/%/full_image: build/userspace/%/app.tbf \
		golf2/target/thumbv7m-none-eabi/release/golf2 ;
	cp golf2/target/thumbv7m-none-eabi/release/golf2 \
		build/userspace/$*/unsigned_image
	arm-none-eabi-objcopy --set-section-flags .apps=alloc,code,contents \
		build/userspace/$*/unsigned_image
	arm-none-eabi-objcopy --update-section \
		.apps=build/userspace/$*/app.tbf \
		build/userspace/$*/unsigned_image
	$(TANGO_CODESIGNER) --b --input build/userspace/$*/unsigned_image \
		--key=$(TANGO_CODESIGNER_KEY) \
		--output=build/userspace/$*/signed_image
	cat $(TANGO_BOOTLOADER) build/userspace/$*/signed_image \
		> build/userspace/$*/full_image
