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
	cd ../../../third_party/elf2tab && cargo run --release -- -n "h1b_tests" \
		-o ../../golf2/apps/h1b_tests/build/cortex-m3/cortex-m3/cortex-m3.tab \
		../../golf2/apps/h1b_tests/$^ --stack=2048 --app-heap=1024 \
		--kernel-heap=1024 --protected-region-size=64
	mv target/thumbv7m-none-eabi/release/h1b_tests.tbf \
		build/cortex-m3/cortex-m3/cortex-m3.tbf

# Builds the h1b_tests Elf file using cargo. Marked as phony because we rely on
# Cargo to determine when to rebuild.
.PHONY: target/thumbv7m-none-eabi/release/h1b_tests
target/thumbv7m-none-eabi/release/h1b_tests:
	cargo build --release
