# Copyright 2020 Google LLC
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

IMAGES=_a _b

# ------------------------------------------------------------------------------
# Macro to define targets for a specific board-app-and-image combination.
# Arguments:
# - $(IMAGE) [optional suffix to build A/B images]
define KERNEL_IMAGE_TARGETS

.PHONY: kernel/build$(IMAGE)
kernel/build$(IMAGE): sandbox_setup
	cd kernel && \
		CARGO_TARGET_DIR="../build/kernel/cargo$(IMAGE)" \
		RUSTFLAGS="-C link-arg=-T./layout$(IMAGE).ld" \
		$(BWRAP) cargo build --release

.PHONY: kernel/build-signed$(IMAGE)
kernel/build-signed$(IMAGE): kernel/build$(IMAGE)

endef # KERNEL_IMAGE_TARGETS

# Instantiate for IMAGE=-a and IMAGE=-b
$(foreach IMAGE,$(IMAGES),$(eval $(KERNEL_IMAGE_TARGETS)))

# Instantiate for IMAGE=""
$(eval $(KERNEL_IMAGE_TARGETS))

.PHONY: kernel/check
kernel/check: sandbox_setup
	cd kernel && $(BWRAP) cargo check --release

.PHONY: kernel/clean
kernel/clean:
	rm -f kernel/Cargo.lock

.PHONY: kernel/devicetests
kernel/devicetests:

.PHONY: kernel/doc
kernel/doc: sandbox_setup
	cd kernel && $(BWRAP) cargo doc --release

.PHONY: kernel/localtests
kernel/localtests:
