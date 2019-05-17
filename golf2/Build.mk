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

.PHONY: golf2/build
golf2/build: golf2/target/thumbv7m-none-eabi/release/golf2

.PHONY: golf2/check
golf2/check:
	$(MAKE) -C golf2 -f TockMakefile check

.PHONY: golf2/clean
golf2/clean:
	$(MAKE) -C golf2 -f TockMakefile clean
	rm -f golf2/Cargo.lock

.PHONY: golf2/devicetests
golf2/devicetests:

.PHONY: golf2/doc
golf2/doc:
	$(MAKE) -C golf2 -f TockMakefile doc

.PHONY: golf2/localtests
golf2/localtests:


.PHONY: golf2/target/thumbv7m-none-eabi/release/golf2
golf2/target/thumbv7m-none-eabi/release/golf2:
	$(MAKE) -C golf2 -f TockMakefile target/thumbv7m-none-eabi/release/golf2
