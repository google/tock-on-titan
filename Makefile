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
BUILD_SUBDIRS := golf2 third_party userspace

.PHONY: all
all: build

.PHONY: build
build: $(addsuffix /build,$(BUILD_SUBDIRS))

.PHONY: check
check: $(addsuffix /check,$(BUILD_SUBDIRS))

# No need to recurse into most directories, as rm does that for us.
.PHONY: clean
clean: golf2/clean userspace/clean
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


include $(addsuffix /Build.mk,$(BUILD_SUBDIRS))
