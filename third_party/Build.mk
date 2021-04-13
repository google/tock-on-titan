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

# Note: the build for third_party/ is special, because we do not always control
# the makefiles in the directories below this. As a result, this makefile must
# interface with the build systems of third_party/*.

# We skip chromiumos-ec and libtock-c as their build systems are only designed
# to work as part of a Tock application build. We skip libtock-rs and tock as
# their target configurations are in different directories.

.PHONY: third_party/build
third_party/build: build/cargo-host/release/elf2tab sandbox_setup
	cd third_party/libtock-rs && \
		CARGO_TARGET_DIR="../../build/userspace/cargo" \
		$(BWRAP) cargo build --offline --release --target=thumbv7m-none-eabi --examples

.PHONY: third_party/build-signed
third_party/build-signed: third_party/build

.PHONY: third_party/check
third_party/check: cargo_version_check sandbox_setup build/elf2tab
	cd build/elf2tab && \
		CARGO_TARGET_DIR="../../build/cargo-host" $(BWRAP) cargo check --release
	cd third_party/libtock-rs && \
		CARGO_TARGET_DIR="../../build/userspace/cargo" \
		$(BWRAP) cargo check --offline --release --target=thumbv7m-none-eabi --examples
	cd third_party/rustc-demangle && \
		CARGO_TARGET_DIR="../../build/cargo-host" \
		$(BWRAP) cargo check --offline --release

.PHONY: third_party/clean
third_party/clean:
	rm -f third_party/libtock-rs/Cargo.lock
	rm -f third_party/rustc-demangle/Cargo.lock

.PHONY: third_party/devicetests
third_party/devicetests:

.PHONY: third_party/doc
third_party/doc: cargo_version_check sandbox_setup build/elf2tab
	cd build/elf2tab && \
		CARGO_TARGET_DIR="../../build/cargo-host" $(BWRAP) cargo doc --release
	cd third_party/rustc-demangle && \
		CARGO_TARGET_DIR="../../build/cargo-host" \
		$(BWRAP) cargo doc --offline --release

.PHONY: third_party/localtests
third_party/localtests: cargo_version_check sandbox_setup build/elf2tab
	cd build/elf2tab && \
		CARGO_TARGET_DIR="../../build/cargo-host" $(BWRAP) cargo test --release
	cd third_party/libtock-rs && \
		CARGO_TARGET_DIR="../../build/cargo-host" \
		$(BWRAP) cargo test --lib --offline --release
	cd third_party/rustc-demangle && \
		CARGO_TARGET_DIR="../../build/cargo-host" \
		$(BWRAP) cargo test --offline --release


.PHONY: build/cargo-host/release/elf2tab
build/cargo-host/release/elf2tab: build/elf2tab cargo_version_check sandbox_setup
	cd build/elf2tab && \
		CARGO_TARGET_DIR="../../build/cargo-host" $(BWRAP) cargo build --release

.PHONY: build/elf2tab
build/elf2tab:
	mkdir -p build && \
	rm -rf build/elf2tab && \
	cp -rp -t build third_party/elf2tab && \
	rm -f build/elf2tab/Cargo.lock
