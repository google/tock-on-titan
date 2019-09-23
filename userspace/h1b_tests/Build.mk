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

.PHONY: userspace/h1b_tests/build
userspace/h1b_tests/build: build/userspace/h1b_tests/full_image

.PHONY: userspace/h1b_tests/check
userspace/h1b_tests/check:
	cd userspace/h1b_tests && TOCK_KERNEL_VERSION=h1b_tests cargo check \
		--offline --release

.PHONY: userspace/h1b_tests/devicetests
userspace/h1b_tests/devicetests: build/cargo-host/release/runner \
                                 build/userspace/h1b_tests/full_image
	flock build/device_lock -c ' \
		$(TANGO_SPIFLASH) --verbose \
		                  --input=build/userspace/h1b_tests/full_image ; \
		stty -F /dev/ttyUltraConsole3 115200 -echo ; \
		stty -F /dev/ttyUltraTarget2 115200 -icrnl ; \
		build/cargo-host/release/runner --test'

.PHONY: userspace/h1b_tests/doc
userspace/h1b_tests/doc:
	cd userspace/h1b_tests && TOCK_KERNEL_VERSION=h1b_tests cargo doc \
		--offline --release

.PHONY: userspace/h1b_tests/localtests
userspace/h1b_tests/localtests:

.PHONY: userspace/h1b_tests/program
userspace/h1b_tests/program: build/userspace/h1b_tests/full_image
	flock build/device_lock -c '$(TANGO_SPIFLASH) --verbose \
		--input=build/userspace/h1b_tests/full_image'

.PHONY: userspace/h1b_tests/run
userspace/h1b_tests/run: build/cargo-host/release/runner \
                         build/userspace/h1b_tests/full_image
	flock build/device_lock -c ' \
		$(TANGO_SPIFLASH) --verbose \
		                  --input=build/userspace/h1b_tests/full_image ; \
		stty -F /dev/ttyUltraConsole3 115200 -echo ; \
		stty -F /dev/ttyUltraTarget2 115200 -icrnl ; \
		build/cargo-host/release/runner'

.PHONY: build/userspace/h1b_tests/h1b_tests
build/userspace/h1b_tests/h1b_tests:
	rm -f build/userspace/cargo/thumbv7m-none-eabi/release/h1b_tests-*
	cd userspace/h1b_tests && TOCK_KERNEL_VERSION=h1b_tests \
		cargo test --no-run --offline --release
	mkdir -p build/userspace/h1b_tests/
	find build/userspace/cargo/thumbv7m-none-eabi/release/ -maxdepth 1 -regex \
		'build/userspace/cargo/thumbv7m-none-eabi/release/h1b_tests-[^.]+' \
		-exec cp '{}' build/userspace/h1b_tests/h1b_tests ';'

# Due to b/139156455, we want to detect the case where an application's size is
# rounded up to 64 KiB.
build/userspace/h1b_tests/h1b_tests.tbf: \
		build/cargo-host/release/elf2tab build/userspace/h1b_tests/h1b_tests
	build/cargo-host/release/elf2tab -n "h1b_tests" \
		-o build/userspace/h1b_tests/h1b_tests_tab \
		build/userspace/h1b_tests/h1b_tests --stack=2048 --app-heap=1024 \
		--kernel-heap=1024 --protected-region-size=64
	if [ "$$(wc -c <build/userspace/h1b_tests/h1b_tests.tbf)" -ge 65536 ]; \
		then echo "#########################################################"; \
		     echo "# Application h1b_tests is too large.                   #"; \
		     echo "# Check size of build/userspace/h1b_tests/h1b_tests.tbf #"; \
		     echo "#########################################################"; \
		     false ; \
		fi

build/userspace/h1b_tests/full_image: \
		build/userspace/h1b_tests/h1b_tests.tbf \
		golf2/target/thumbv7m-none-eabi/release/golf2
	cp golf2/target/thumbv7m-none-eabi/release/golf2 \
		build/userspace/h1b_tests/unsigned_image
	arm-none-eabi-objcopy --set-section-flags .apps=alloc,code,contents \
		build/userspace/h1b_tests/unsigned_image
	arm-none-eabi-objcopy --update-section \
		.apps=build/userspace/h1b_tests/h1b_tests.tbf \
		build/userspace/h1b_tests/unsigned_image
	$(TANGO_CODESIGNER) --b --input build/userspace/h1b_tests/unsigned_image \
		--key=$(TANGO_CODESIGNER_KEY) \
		--output=build/userspace/h1b_tests/signed_image
	cat $(TANGO_BOOTLOADER) build/userspace/h1b_tests/signed_image \
		> build/userspace/h1b_tests/full_image
