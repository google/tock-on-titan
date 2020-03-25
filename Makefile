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

# Subdirectories containing Build.mk files.
BUILD_SUBDIRS := golf2 papa runner third_party tools userspace

.PHONY: all
all: build

.PHONY: build
build: $(addsuffix /build,$(BUILD_SUBDIRS))

.PHONY: check
check: $(addsuffix /check,$(BUILD_SUBDIRS))

# No need to recurse into most directories, as rm does that for us.
.PHONY: clean
clean: golf2/clean papa/clean userspace/clean
	rm -rf build/

.PHONY: devicetests
devicetests: $(addsuffix /devicetests,$(BUILD_SUBDIRS))

.PHONY: doc
doc: $(addsuffix /doc,$(BUILD_SUBDIRS))

.PHONY: localtests
localtests: $(addsuffix /localtests,$(BUILD_SUBDIRS))

.PHONY: prtest
prtest: build devicetests localtests
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
