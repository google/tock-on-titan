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

BUILD_SUBDIRS := $(addprefix userspace/,dcrypto_test gpio_test h1b_tests \
		   personality_clear personality_test sha_test spin \
		   u2f_app u2f_test )

.PHONY: userspace/build
userspace/build: $(addsuffix /build,$(BUILD_SUBDIRS))

.PHONY: userspace/check
userspace/check:
	cd userspace && cargo check --release

.PHONY: userspace/clean
userspace/clean:
	rm -f userspace/Cargo.lock

.PHONY: userspace/devicetests
userspace/devicetests: $(addsuffix /devicetests,$(BUILD_SUBDIRS))

.PHONY: userspace/doc
userspace/doc:
	cd userspace && cargo doc --release

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
