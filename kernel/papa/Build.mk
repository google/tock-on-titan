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

.PHONY: papa/build
papa/build: papa/target/thumbv7m-none-eabi/release/papa

.PHONY: papa/check
papa/check:
	$(MAKE) -C papa -f TockMakefile check

.PHONY: papa/clean
papa/clean:
	$(MAKE) -C papa -f TockMakefile clean
	rm -f papa/Cargo.lock

.PHONY: papa/devicetests
papa/devicetests:

.PHONY: papa/doc
papa/doc:
	$(MAKE) -C papa -f TockMakefile doc

.PHONY: papa/localtests
papa/localtests:


.PHONY: papa/target/thumbv7m-none-eabi/release/papa
papa/target/thumbv7m-none-eabi/release/papa:
	$(MAKE) -C papa -f TockMakefile target/thumbv7m-none-eabi/release/papa
