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

# Note that `all` does _not_ depend on `build-signed`.  Many consumers
# of this project (especially new ones) won't be happy if they have to
# struggle with signing from day one.
.PHONY: all
all: build

# Subdirectories containing Build.mk files.
BUILD_SUBDIRS := kernel runner third_party tools userspace

# Both cargo and Tock's build system like connecting to the internet and
# installing things during builds. We don't like that. This is a sandbox we can
# run commands in that denies network access as well as write access outside the
# build/ directory.
#
# The mount order of /tmp and CURDIR is important. It is reasonable for someone
# to check this repository out under /tmp and try to build it. If this only has
# `--ro-bind / / --tmpfs /tmp` without `--ro-bind "$(CURDIR)" "$(CURDIR)"`,
# the sandbox will not contain any source code and the build will fail. Mounting
# in this order leaves the source code available and /tmp writable.
BWRAP := bwrap                                                               \
         --ro-bind / /                                                       \
         --tmpfs /tmp                                                        \
         --ro-bind "$(CURDIR)" "$(CURDIR)"                                   \
         --bind "$(CURDIR)/build" "$(CURDIR)/build"                          \
         --bind "$(CURDIR)/kernel/Cargo.lock" "$(CURDIR)/kernel/Cargo.lock"  \
         --bind "$(CURDIR)/runner/Cargo.lock" "$(CURDIR)/runner/Cargo.lock"  \
         --bind "$(CURDIR)/third_party/libtock-rs/Cargo.lock"                \
                "$(CURDIR)/third_party/libtock-rs/Cargo.lock"                \
         --bind "$(CURDIR)/third_party/rustc-demangle/Cargo.lock"            \
                "$(CURDIR)/third_party/rustc-demangle/Cargo.lock"            \
         --bind "$(CURDIR)/tools/Cargo.lock" "$(CURDIR)/tools/Cargo.lock"    \
         --bind "$(CURDIR)/userspace/Cargo.lock"                             \
                "$(CURDIR)/userspace/Cargo.lock"                             \
         --dev /dev                                                          \
         --unshare-all

# A target that sets up directories the bwrap sandbox needs.
.PHONY: sandbox_setup
sandbox_setup:
	mkdir -p build
	>kernel/Cargo.lock
	>runner/Cargo.lock
	>third_party/libtock-rs/Cargo.lock
	>third_party/rustc-demangle/Cargo.lock
	>tools/Cargo.lock
	>userspace/Cargo.lock

.PHONY: build
build: $(addsuffix /build,$(BUILD_SUBDIRS))

.PHONY: build-signed
build-signed: $(addsuffix /build-signed,$(BUILD_SUBDIRS))

.PHONY: check
check: $(addsuffix /check,$(BUILD_SUBDIRS))

# No need to recurse into most directories, as rm does that for us.
.PHONY: clean
clean: kernel/clean runner/clean third_party/clean tools/clean userspace/clean
	rm -rf build/

.PHONY: devicetests
devicetests: $(addsuffix /devicetests,$(BUILD_SUBDIRS))

.PHONY: doc
doc: $(addsuffix /doc,$(BUILD_SUBDIRS))

.PHONY: localtests
localtests: $(addsuffix /localtests,$(BUILD_SUBDIRS))

.PHONY: prtest
prtest: build check devicetests localtests
	@echo '------------------------------------------------------'
	@echo 'prtest successful. When you open a PR, paste the below'
	@echo 'block (not the output above) into the PR description:'
	@echo '------------------------------------------------------'
	@echo '```'
	@echo '----------------------'
	@echo '`make prtest` summary:'
	@echo '----------------------'
	git rev-parse HEAD
	git status
	@echo '```'

# Installs the necessary Rust toolchains
.PHONY: setup
setup:
	rustup toolchain add --profile minimal \
		"$$(cat third_party/libtock-rs/rust-toolchain)"
	rustup toolchain add --profile minimal \
		"$$(cat third_party/tock/rust-toolchain)"
	rustup target add --toolchain \
		"$$(cat third_party/libtock-rs/rust-toolchain)" thumbv7m-none-eabi
	rustup target add --toolchain \
		"$$(cat third_party/tock/rust-toolchain)" thumbv7m-none-eabi


# A target that prints an error message and fails the build if the cargo version
# is not sufficiently up-to-date.
.PHONY: cargo_version_check
cargo_version_check:
	min_version="1.37.0" ; \
	cargo_version="$$(cargo -V | awk '{ print $$2 }')" ; \
	if [ "$$(third_party/tock/tools/semver.sh $${cargo_version} \< $${min_version})" != "false" ] ; \
		then echo "#######################################################################"; \
		     echo "# Please update your stable toolchain. Minimum version: $${min_version}"; \
		     echo "#######################################################################"; \
		     exit 1; \
		fi

include $(addsuffix /Build.mk,$(BUILD_SUBDIRS))
