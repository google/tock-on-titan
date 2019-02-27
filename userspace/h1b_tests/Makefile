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

# Supported targets (these correspond to cargo commands)
#   all (default) -- alias for build
#   build
#   check
#   clean
#   doc

.PHONY: all
all: build

.PHONY: build
build: build/cortex-m3/cortex-m3/cortex-m3.tbf

.PHONY: check
check:
	cargo check --release

.PHONY: clean
clean:
	cargo clean
	rm -rf build/

.PHONY: doc
doc:
	cargo doc --release

# Builds the Tock Binary Format (TBF) executable from the cargo-generated ELF
# file. We use elf2tab -- it will create the tbf file as a side-effect. It
# creates it in the same directory as the elf files, while the golf2 Makefile
# expects it at build/cortex-m3/cortex-m3/cortex-m3.tbf, so we manually move the
# file over afterwards.
build/cortex-m3/cortex-m3/cortex-m3.tbf: target/thumbv7m-none-eabi/release/h1b_tests
	mkdir -p build/cortex-m3/cortex-m3
	cd ../../third_party/elf2tab && cargo run --release -- -n "h1b_tests" \
		-o ../../userspace/h1b_tests/build/cortex-m3/cortex-m3/cortex-m3.tab \
		../../userspace/h1b_tests/$^ --stack=2048 --app-heap=1024 \
		--kernel-heap=1024 --protected-region-size=64
	mv target/thumbv7m-none-eabi/release/h1b_tests.tbf \
		build/cortex-m3/cortex-m3/cortex-m3.tbf

# Builds the h1b_tests Elf file using cargo. Marked as phony because we rely on
# Cargo to determine when to rebuild. We build using `cargo test` so that rustc
# builds the test code and tests list. We can't just pass --test to rustc
# through rustflags because that causes cargo to fail to parse rustc's output
# for some invocations. Cargo names the test h1b_test-<hex string>, and outputs
# another file with the same name plus ".d", so we need to use find with a
# regexp to locate the test executable. Note that we first clear out existing
# test binaries because Cargo does not do so; the `find` call relies on there
# only being a single test binary.
.PHONY: target/thumbv7m-none-eabi/release/h1b_tests
target/thumbv7m-none-eabi/release/h1b_tests:
	rm -f target/thumbv7m-none-eabi/release/h1b_tests-*
	cargo test --no-run --release
	find target/thumbv7m-none-eabi/release/ -maxdepth 1 \
		-regex 'target/thumbv7m-none-eabi/release/h1b_tests-[^.]+' \
		-exec cp '{}' target/thumbv7m-none-eabi/release/h1b_tests ';'
