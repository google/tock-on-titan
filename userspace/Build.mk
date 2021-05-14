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
                                         otpilot           \
                                         personality_clear \
                                         personality_test  \
                                         rng               \
                                         sha_test          \
                                         spin              \
                                         u2f_app           \
                                         u2f_test )

# All boards that we should build for
BOARDS += golf2
BOARDS += papa

# The board we should run tests on
TANGO_BOARD_FOR_TEST ?= golf2

.PHONY: userspace/build
userspace/build: $(addsuffix /build,$(BUILD_SUBDIRS))

.PHONY: userspace/build-signed
userspace/build-signed: userspace/build
userspace/build-signed: $(addsuffix /build-signed,$(BUILD_SUBDIRS))

.PHONY: userspace/check
userspace/check: build/gitlongtag sandbox_setup
	cd userspace && TOCK_KERNEL_VERSION=h1_tests $(BWRAP) cargo check --release

.PHONY: userspace/clean
userspace/clean:
	rm -f userspace/Cargo.lock

.PHONY: userspace/devicetests
userspace/devicetests: $(addsuffix /devicetests,$(BUILD_SUBDIRS))

.PHONY: userspace/doc
userspace/doc: sandbox_setup
	cd userspace && TOCK_KERNEL_VERSION=h1 $(BWRAP) cargo doc --release

.PHONY: userspace/localtests
userspace/localtests: $(addsuffix /localtests,$(BUILD_SUBDIRS))

# .PHONY so it happens every time, even though we do write a file at the target
.PHONY: build/gitlongtag
build/gitlongtag:
	mkdir -p build
	# Remove the trailing newline character
	printf '%s' $$(git describe --always --dirty --long) >$@

include $(addsuffix /Build.mk,$(BUILD_SUBDIRS))

# ------------------------------------------------------------------------------
# Build rules shared between the C apps.
# ------------------------------------------------------------------------------

# Each of these build rules exists once for each C app. These rules are static
# pattern rules, documented here:
# https://www.gnu.org/software/make/manual/html_node/Static-Pattern.html.
#
# Example:
# if BOARDS is "board1 board2"
# and if C_APPS is "userspace/app1 userspace/app2" and
# and if C_APPS_board2 is "userspace/app3"
# then the /build targets will expand as follows:
#   .PHONY: userspace/app1/build userspace/app2/build userspace/app3/build
#   userspace/app1/build: build/userspace/app1/board1/full_image
#   userspace/app1/build: build/userspace/app1/board2/full_image
#   userspace/app2/build: build/userspace/app2/board1/full_image
#   userspace/app2/build: build/userspace/app2/board2/full_image
#   userspace/app3/build: build/userspace/app3/board2/full_image


# ------------------------------------------------------------------------------
# Macro to define unsigned_image and full_image targets for a specific
# board-and-app-and-tbf-file combination.
# Arguments:
# - $(BOARD)
# - $(APP)
# - $(TBF_FILE)
define IMAGE_TARGETS

build/userspace/$(APP)/$(BOARD)/unsigned_image: \
		build/userspace/$(APP)/$(TBF_FILE) \
		kernel/build
	mkdir -p build/userspace/$(APP)/$(BOARD)/
	cp build/kernel/cargo/thumbv7m-none-eabi/release/$(BOARD) \
		build/userspace/$(APP)/$(BOARD)/unsigned_image
	arm-none-eabi-objcopy --set-section-flags .apps=alloc,code,contents \
		build/userspace/$(APP)/$(BOARD)/unsigned_image
	arm-none-eabi-objcopy --update-section \
		.apps=build/userspace/$(APP)/$(TBF_FILE) \
		build/userspace/$(APP)/$(BOARD)/unsigned_image

build/userspace/$(APP)/$(BOARD)/full_image: \
		build/userspace/$(APP)/$(BOARD)/unsigned_image
	$(TANGO_CODESIGNER) --b --input build/userspace/$(APP)/$(BOARD)/unsigned_image \
		--key=$(TANGO_CODESIGNER_KEY) \
		--output=build/userspace/$(APP)/$(BOARD)/signed_image;
	cat $(TANGO_BOOTLOADER) build/userspace/$(APP)/$(BOARD)/signed_image \
		> build/userspace/$(APP)/$(BOARD)/full_image;

endef


# ------------------------------------------------------------------------------
# Macro to define targets for a specific board-and-app combination.
# Arguments:
# - $(BOARD)
# - $(APP)
define C_APP_BOARD_TARGETS

# `foreach TBF_FILE` ensures that TBF_FILE has the expected value when
# IMAGE_TARGETS is expanded.
$(foreach TBF_FILE,cortex-m3/cortex-m3.tbf,$(eval $(IMAGE_TARGETS)))

.PHONY: userspace/$(APP)/$(BOARD)/build
userspace/$(APP)/$(BOARD)/build: \
		build/userspace/$(APP)/$(BOARD)/unsigned_image

.PHONY: userspace/$(APP)/$(BOARD)/build-signed
userspace/$(APP)/$(BOARD)/build-signed: \
		build/userspace/$(APP)/$(BOARD)/full_image

.PHONY: userspace/$(APP)/$(BOARD)/check
userspace/$(APP)/$(BOARD)/check:

.PHONY: userspace/$(APP)/$(BOARD)/devicetests
userspace/$(APP)/$(BOARD)/devicetests:

.PHONY: userspace/$(APP)/$(BOARD)/doc
userspace/$(APP)/$(BOARD)/doc:

.PHONY: userspace/$(APP)/$(BOARD)/localtests
userspace/$(APP)/$(BOARD)/localtests:

.PHONY: userspace/$(APP)/$(BOARD)/program
userspace/$(APP)/$(BOARD)/program:  \
		build/userspace/$(APP)/$(BOARD)/full_image
	flock build/device_lock $(TANGO_SPIFLASH) --verbose \
		--input=build/userspace/$(APP)/$(BOARD)/full_image

.PHONY: userspace/$(APP)/$(BOARD)/run
userspace/$(APP)/$(BOARD)/run: \
		build/cargo-host/release/runner build/userspace/$(APP)/$(BOARD)/full_image
	flock build/device_lock -c ' \
		$(TANGO_SPIFLASH) --verbose \
			--input=build/userspace/$(APP)/$(BOARD)/full_image ; \
		stty -F /dev/ttyUltraConsole3 115200 -echo ; \
		stty -F /dev/ttyUltraTarget2 115200 -icrnl ; \
		build/cargo-host/release/runner'

endef

# C_APP_TARGET contains build rules that are specific to a C app but not an
# (APP, BOARD) combination.
define C_APP_TARGETS

# Because $(MAKE) is expanded inside an outer variable, `make` isn't sure it
# refers to `make`. Because of that, it won't pass through jobserver arguments.
# The leading '+' tells it to do so anyway.
.PHONY: build/userspace/$(APP)/cortex-m3/cortex-m3.tbf
build/userspace/$(APP)/cortex-m3/cortex-m3.tbf: \
		build/cargo-host/release/elf2tab
	+flock build/libtock_c_lock $(BWRAP) $(MAKE) -C userspace/$(APP) -f TockMakefile

endef

# ------------------------------------------------------------------------------
# Generate the targets for all board-and-app combinations.
$(foreach BOARD,$(BOARDS),$(foreach APP,$(C_APPS) $(C_APPS_$(BOARD)),$(eval $(C_APP_BOARD_TARGETS))))
$(foreach APP,$(C_APPS),$(eval $(C_APP_TARGETS)))


# ------------------------------------------------------------------------------
# Macro to define target for a specific app.
# The target depends on all board-and-app combinations that are valid for the specific app.
# A combination with a board is valid if the app is in $(C_APPS) or in $(C_APPS_$(BOARD)).
# Arguments:
# - $(APP)
define C_APP_COMBINED_TARGET

# Entry target for an individual app to build unsigned app for all boards
.PHONY: userspace/$(APP)/build
userspace/$(APP)/build: \
		$(if $(filter $(APP),$(C_APPS)),$(foreach BOARD,$(BOARDS),userspace/$(APP)/$(BOARD)/build)) \
		$(foreach BOARD,$(BOARDS),$(if $(filter $(APP),$(C_APPS_$(BOARD))),userspace/$(APP)/$(BOARD)/build))

# Entry target for an individual app to build signed app for all boards
.PHONY: userspace/$(APP)/build-signed
userspace/$(APP)/build-signed: \
		$(if $(filter $(APP),$(C_APPS)),$(foreach BOARD,$(BOARDS),userspace/$(APP)/$(BOARD)/build-signed)) \
		$(foreach BOARD,$(BOARDS),$(if $(filter $(APP),$(C_APPS_$(BOARD))),userspace/$(APP)/$(BOARD)/build-signed))

# Entry target for an individual app to run check for all boards
.PHONY: userspace/$(APP)/check
userspace/$(APP)/check: \
		$(if $(filter $(APP),$(C_APPS)),$(foreach BOARD,$(BOARDS),userspace/$(APP)/$(BOARD)/check)) \
		$(foreach BOARD,$(BOARDS),$(if $(filter $(APP),$(C_APPS_$(BOARD))),userspace/$(APP)/$(BOARD)/check))

# Entry target for an individual app to run devicetest on the test board
.PHONY: userspace/$(APP)/devicetests
userspace/$(APP)/devicetests: \
		$(if $(filter $(APP),$(C_APPS)),userspace/$(APP)/$(TANGO_BOARD_FOR_TEST)/devicetests) \
		$(if $(filter $(APP),$(C_APPS_$(TANGO_BOARD_FOR_TEST))),userspace/$(APP)/$(TANGO_BOARD_FOR_TEST)/devicetests)

# Entry target for an individual app to run localtests for all boards
.PHONY: userspace/$(APP)/localtests
userspace/$(APP)/localtests: \
		$(if $(filter $(APP),$(C_APPS)),$(foreach BOARD,$(BOARDS),userspace/$(APP)/$(BOARD)/localtests)) \
		$(foreach BOARD,$(BOARDS),$(if $(filter $(APP),$(C_APPS_$(BOARD))),userspace/$(APP)/$(BOARD)/localtests))

endef

# ------------------------------------------------------------------------------
# Generate the targets for all apps combinations.
$(foreach APP,$(C_APPS) $(foreach BOARD,$(BOARDS),$(C_APPS_$(BOARD))),$(eval $(C_APP_COMBINED_TARGET)))

# ------------------------------------------------------------------------------
# Build rules shared between Rust app targets. These are not used for Rust test
# apps, which are handled below.
# ------------------------------------------------------------------------------

# These are static pattern rules, see above (the C apps rules) section for an
# explanation.

# ------------------------------------------------------------------------------
# Macro to define targets for a specific board-and-app combination.
# Arguments:
# - $(BOARD)
# - $(APP)
define RUST_APP_BOARD_TARGETS

# `foreach TBF_FILE` ensures that TBF_FILE has the expected value when
# IMAGE_TARGETS is expanded.
$(foreach TBF_FILE,$(BOARD)/app.tbf,$(eval $(IMAGE_TARGETS)))

.PHONY: userspace/$(APP)/$(BOARD)/build
userspace/$(APP)/$(BOARD)/build: \
		build/userspace/$(APP)/$(BOARD)/unsigned_image

.PHONY: userspace/$(APP)/$(BOARD)/build-signed
userspace/$(APP)/$(BOARD)/build-signed: \
		build/userspace/$(APP)/$(BOARD)/full_image

.PHONY: userspace/$(APP)/$(BOARD)/check
userspace/$(APP)/$(BOARD)/check: sandbox_setup build/gitlongtag
	cd userspace/$(APP) && TOCK_KERNEL_VERSION=$(APP) $(BWRAP) cargo check \
		--offline --release

.PHONY: userspace/$(APP)/$(BOARD)/devicetests
userspace/$(APP)/$(BOARD)/devicetests:

.PHONY: userspace/$(APP)/$(BOARD)/doc
userspace/$(APP)/$(BOARD)/doc: sandbox_setup build/gitlongtag
	cd userspace/$(APP) && TOCK_KERNEL_VERSION=$(APP) $(BWRAP) cargo doc \
		--offline --release

.PHONY: userspace/$(APP)/$(BOARD)/localtests
userspace/$(APP)/$(BOARD)/localtests:

.PHONY: userspace/$(APP)/$(BOARD)/program
userspace/$(APP)/$(BOARD)/program: \
		build/userspace/$(APP)/$(BOARD)/full_image
	flock build/device_lock -c '$(TANGO_SPIFLASH) --verbose \
		--input=build/userspace/$(APP)/$(BOARD)/full_image'

.PHONY: userspace/$(APP)/$(BOARD)/run
userspace/$(APP)/$(BOARD)/run: \
		build/cargo-host/release/runner build/userspace/$(APP)/$(BOARD)/full_image
	flock build/device_lock -c ' \
		$(TANGO_SPIFLASH) --verbose \
				  --input=build/userspace/$(APP)/$(BOARD)/full_image ; \
		stty -F /dev/ttyUltraConsole3 115200 -echo ; \
		stty -F /dev/ttyUltraTarget2 115200 -icrnl ; \
		build/cargo-host/release/runner'

.PHONY: build/userspace/$(APP)/$(BOARD)/app
build/userspace/$(APP)/$(BOARD)/app: sandbox_setup build/gitlongtag
	rm -f build/userspace/cargo/thumbv7m-none-eabi/release/$(APP)-*
	cd userspace/$(APP) && TOCK_KERNEL_VERSION=$(APP) $(BWRAP) cargo build \
		--offline --release
	mkdir -p build/userspace/$(APP)/$(BOARD)/
	cp "build/userspace/cargo/thumbv7m-none-eabi/release/$(APP)" \
		"build/userspace/$(APP)/$(BOARD)/app"

# We want to detect when an application's size is larger than 64 KiB.
build/userspace/$(APP)/$(BOARD)/app.tbf: \
		build/cargo-host/release/elf2tab build/userspace/$(APP)/$(BOARD)/app
	build/cargo-host/release/elf2tab -n $(APP) \
		-o build/userspace/$(APP)/$(BOARD)/app_tab \
		build/userspace/$(APP)/$(BOARD)/app --stack=2048 --app-heap=4096 \
		--kernel-heap=1024 --protected-region-size=64
	if [ "$$$$(wc -c < build/userspace/$(APP)/$(BOARD)/app.tbf)" -gt 65536 ]; \
		then echo "#########################################################"; \
		     echo "# Application $(notdir $(APP)) for board $(BOARD) is too large."; \
		     echo "# Check size of build/userspace/$(APP)/$(BOARD)/app.tbf"; \
		     echo "#########################################################"; \
		     false ; \
		fi

endef

# ------------------------------------------------------------------------------
# Generate the targets for all board-and-app combinations.
$(foreach BOARD,$(BOARDS),$(foreach APP,$(RUST_APPS) $(RUST_APPS_$(BOARD)),$(eval $(RUST_APP_BOARD_TARGETS))))


# ------------------------------------------------------------------------------
# Macro to define target for a specific app.
# The target depends on all board-and-app combinations that are valid for the specific app.
# A combination with a board is valid if the app is in $(RUST_APPS) or in $(RUST_APPS_$(BOARD)).
# Arguments:
# - $(APP)
define RUST_APP_TARGET

# Entry target for an individual app to build unsigned app for all boards
.PHONY: userspace/$(APP)/build
userspace/$(APP)/build: \
		$(if $(filter $(APP),$(RUST_APPS)),$(foreach BOARD,$(BOARDS),userspace/$(APP)/$(BOARD)/build)) \
		$(foreach BOARD,$(BOARDS),$(if $(filter $(APP),$(RUST_APPS_$(BOARD))),userspace/$(APP)/$(BOARD)/build))

# Entry target for an individual app to build signed app for all boards
.PHONY: userspace/$(APP)/build-signed
userspace/$(APP)/build-signed: \
		$(if $(filter $(APP),$(RUST_APPS)),$(foreach BOARD,$(BOARDS),userspace/$(APP)/$(BOARD)/build-signed)) \
		$(foreach BOARD,$(BOARDS),$(if $(filter $(APP),$(RUST_APPS_$(BOARD))),userspace/$(APP)/$(BOARD)/build-signed))

# Entry target for an individual app to run check for all boards
.PHONY: userspace/$(APP)/check
userspace/$(APP)/check: \
		$(if $(filter $(APP),$(RUST_APPS)),$(foreach BOARD,$(BOARDS),userspace/$(APP)/$(BOARD)/check)) \
		$(foreach BOARD,$(BOARDS),$(if $(filter $(APP),$(RUST_APPS_$(BOARD))),userspace/$(APP)/$(BOARD)/check))

# Entry target for an individual app to run devicetests on the test board
.PHONY: userspace/$(APP)/devicetests
userspace/$(APP)/devicetests: \
		$(if $(filter $(APP),$(RUST_APPS)),userspace/$(APP)/$(TANGO_BOARD_FOR_TEST)/devicetests) \
		$(if $(filter $(APP),$(RUST_APPS_$(TANGO_BOARD_FOR_TEST))),userspace/$(APP)/$(TANGO_BOARD_FOR_TEST)/devicetests)

# Entry target for an individual app to run localtests for all boards
.PHONY: userspace/$(APP)/localtests
userspace/$(APP)/localtests: \
		$(if $(filter $(APP),$(RUST_APPS)),$(foreach BOARD,$(BOARDS),userspace/$(APP)/$(BOARD)/localtests)) \
		$(foreach BOARD,$(BOARDS),$(if $(filter $(APP),$(RUST_APPS_$(BOARD))),userspace/$(APP)/$(BOARD)/localtests))

endef

# ------------------------------------------------------------------------------
# Generate the targets for all apps combinations.
$(foreach APP,$(RUST_APPS) $(foreach BOARD,$(BOARDS),$(RUST_APPS_$(BOARD))),$(eval $(RUST_APP_TARGET)))

# ------------------------------------------------------------------------------
# Build rules shared between Rust test app targets.
# ------------------------------------------------------------------------------

# These are static pattern rules, see above (the C apps rules) section for an
# explanation.

# ------------------------------------------------------------------------------
# Macro to define targets for a specific board-and-app combination.
# Arguments:
# - $(BOARD)
# - $(APP)
define RUST_TEST_BOARD_TARGETS

# `foreach TBF_FILE` ensures that TBF_FILE has the expected value when
# IMAGE_TARGETS is expanded.
$(foreach TBF_FILE,$(BOARD)/app.tbf,$(eval $(IMAGE_TARGETS)))

.PHONY: userspace/$(APP)/$(BOARD)/build
userspace/$(APP)/$(BOARD)/build: \
		build/userspace/$(APP)/$(BOARD)/unsigned_image

.PHONY: userspace/$(APP)/$(BOARD)/build-signed
userspace/$(APP)/$(BOARD)/build-signed: \
		build/userspace/$(APP)/$(BOARD)/full_image

.PHONY: userspace/$(APP)/$(BOARD)/check
userspace/$(APP)/$(BOARD)/check: sandbox_setup
	cd userspace/$(APP) && TOCK_KERNEL_VERSION=$(APP) $(BWRAP) cargo check \
		--offline --release

.PHONY: userspace/$(APP)/$(BOARD)/devicetests
userspace/$(APP)/$(BOARD)/devicetests: \
		build/cargo-host/release/runner build/userspace/$(APP)/$(BOARD)/full_image
	flock build/device_lock -c ' \
		$(TANGO_SPIFLASH) --verbose \
				  --input=build/userspace/$(APP)/$(BOARD)/full_image ; \
		stty -F /dev/ttyUltraConsole3 115200 -echo ; \
		stty -F /dev/ttyUltraTarget2 115200 -icrnl ; \
		build/cargo-host/release/runner --test'

.PHONY: userspace/$(APP)/$(BOARD)/doc
userspace/$(APP)/$(BOARD)/doc: sandbox_setup
	cd userspace/$(APP) && TOCK_KERNEL_VERSION=$(APP) $(BWRAP) cargo doc \
		--offline --release

.PHONY: userspace/$(APP)/$(BOARD)/localtests
userspace/$(APP)/$(BOARD)/localtests:

.PHONY: userspace/$(APP)/$(BOARD)/program
userspace/$(APP)/$(BOARD)/program: \
		build/userspace/$(APP)/$(BOARD)/full_image
	flock build/device_lock -c '$(TANGO_SPIFLASH) --verbose \
		--input=build/userspace/$(APP)/$(BOARD)/full_image'

.PHONY: userspace/$(APP)/$(BOARD)/run
userspace/$(APP)/$(BOARD)/run: \
		build/cargo-host/release/runner build/userspace/$(APP)/$(BOARD)/full_image
	flock build/device_lock -c ' \
		$(TANGO_SPIFLASH) --verbose \
				  --input=build/userspace/$(APP)/$(BOARD)/full_image ; \
		stty -F /dev/ttyUltraConsole3 115200 -echo ; \
		stty -F /dev/ttyUltraTarget2 115200 -icrnl ; \
		build/cargo-host/release/runner'

.PHONY: build/userspace/$(APP)/$(BOARD)/app
build/userspace/$(APP)/$(BOARD)/app: sandbox_setup
	rm -f build/userspace/cargo/thumbv7m-none-eabi/release//$(APP)-*
	cd userspace/$(APP) && TOCK_KERNEL_VERSION=$(APP) \
		$(BWRAP) cargo test --no-run --offline --release
	mkdir -p build/userspace/$(APP)/$(BOARD)
	find build/userspace/cargo/thumbv7m-none-eabi/release/ -maxdepth 1 -regex \
		'build/userspace/cargo/thumbv7m-none-eabi/release/$(APP)-[^.]+' \
		-exec cp '{}' build/userspace/$(APP)/$(BOARD)/app ';'

# Due to b/139156455, we want to detect the case where an application's size is
# rounded up to 64 KiB.
build/userspace/$(APP)/$(BOARD)/app.tbf: \
		build/cargo-host/release/elf2tab build/userspace/$(APP)/$(BOARD)/app
	build/cargo-host/release/elf2tab -n $(APP) \
		-o build/userspace/$(APP)/$(BOARD)/app_tab \
		build/userspace/$(APP)/$(BOARD)/app --stack=2048 --app-heap=4096 \
		--kernel-heap=1024 --protected-region-size=64
	if [ "$$$$(wc -c build/userspace/$(APP)/$(BOARD)/app.tbf)" -ge 65536 ]; \
		then echo "#########################################################"; \
		     echo "# Application $(APP) for board $(BOARD) is too large."; \
		     echo "# Check size of build/userspace/$(APP)/$(BOARD)/app.tbf"; \
		     echo "#########################################################"; \
		     false ; \
		fi

endef

# ------------------------------------------------------------------------------
# Generate the targets for all board-and-app combinations.
$(foreach BOARD,$(BOARDS),$(foreach APP,$(RUST_TESTS) $(RUST_TESTS_$(BOARD)),$(eval $(RUST_TEST_BOARD_TARGETS))))


# ------------------------------------------------------------------------------
# Macro to define target for a specific app.
# The target depends on all board-and-app combinations that are valid for the specific app.
# A combination with a board is valid if the app is in $(RUST_APPS) or in $(RUST_APPS_$(BOARD)).
# Arguments:
# - $(APP)
define RUST_TEST_TARGET

# Entry target for an individual app to build unsigned app for all boards
.PHONY: userspace/$(APP)/build
userspace/$(APP)/build: \
		$(if $(filter $(APP),$(RUST_TESTS)),$(foreach BOARD,$(BOARDS),userspace/$(APP)/$(BOARD)/build)) \
		$(foreach BOARD,$(BOARDS),$(if $(filter $(APP),$(RUST_TESTS_$(BOARD))),userspace/$(APP)/$(BOARD)/build))

# Entry target for an individual app to build signed app for all boards
.PHONY: userspace/$(APP)/build-signed
userspace/$(APP)/build-signed: \
		$(if $(filter $(APP),$(RUST_TESTS)),$(foreach BOARD,$(BOARDS),userspace/$(APP)/$(BOARD)/build-signed)) \
		$(foreach BOARD,$(BOARDS),$(if $(filter $(APP),$(RUST_TESTS_$(BOARD))),userspace/$(APP)/$(BOARD)/build-signed))

# Entry target for an individual app to run check for all boards
.PHONY: userspace/$(APP)/check
userspace/$(APP)/check: \
		$(if $(filter $(APP),$(RUST_TESTS)),$(foreach BOARD,$(BOARDS),userspace/$(APP)/$(BOARD)/check)) \
		$(foreach BOARD,$(BOARDS),$(if $(filter $(APP),$(RUST_TESTS_$(BOARD))),userspace/$(APP)/$(BOARD)/check))

# Entry target for an individual app to run devicetests on the test board
.PHONY: userspace/$(APP)/devicetests
userspace/$(APP)/devicetests: \
		$(if $(filter $(APP),$(RUST_TESTS)),userspace/$(APP)/$(TANGO_BOARD_FOR_TEST)/devicetests) \
		$(if $(filter $(APP),$(RUST_TESTS_$(TANGO_BOARD_FOR_TEST))),userspace/$(APP)/$(TANGO_BOARD_FOR_TEST)/devicetests)

# Entry target for an individual app to run localtests for all boards
.PHONY: userspace/$(APP)/localtests
userspace/$(APP)/localtests: \
		$(if $(filter $(APP),$(RUST_TESTS)),$(foreach BOARD,$(BOARDS),userspace/$(APP)/$(BOARD)/localtests)) \
		$(foreach BOARD,$(BOARDS),$(if $(filter $(APP),$(RUST_TESTS_$(BOARD))),userspace/$(APP)/$(BOARD)/localtests))

endef

# ------------------------------------------------------------------------------
# Generate the targets for all apps combinations.
$(foreach APP,$(RUST_TESTS) $(foreach BOARD,$(BOARDS),$(RUST_TESTS_$(BOARD))),$(eval $(RUST_TEST_TARGET)))
