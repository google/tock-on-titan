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
third_party/build: build/cargo-host/release/elf2tab

.PHONY: third_party/check
third_party/check: cargo_version_check
	cd third_party/elf2tab && \
		CARGO_BUILD_TARGET_DIR="../../build/cargo-host" \
		cargo check --frozen --release
	cd third_party/rustc-demangle && \
		CARGO_BUILD_TARGET_DIR="../../build/cargo-host" \
		cargo check --offline --release

.PHONY: third_party/devicetests
third_party/devicetests:

.PHONY: third_party/doc
third_party/doc: cargo_version_check
	cd third_party/elf2tab && \
		CARGO_BUILD_TARGET_DIR="../../build/cargo-host" \
		cargo doc --frozen --release
	cd third_party/rustc-demangle && \
		CARGO_BUILD_TARGET_DIR="../../build/cargo-host" \
		cargo doc --offline --release

.PHONY: third_party/localtests
third_party/localtests: cargo_version_check
	cd third_party/elf2tab && \
		CARGO_BUILD_TARGET_DIR="../../build/cargo-host" \
		cargo test --frozen --release
	cd third_party/rustc-demangle && \
		CARGO_BUILD_TARGET_DIR="../../build/cargo-host" \
		cargo test --offline --release


.PHONY: build/cargo-host/release/elf2tab
build/cargo-host/release/elf2tab: cargo_version_check
	cd third_party/elf2tab && \
		CARGO_BUILD_TARGET_DIR="../../build/cargo-host" \
		cargo build --frozen --release
