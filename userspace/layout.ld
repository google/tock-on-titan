/* Copyright 2018 Google LLC
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

/* Note: modifications should be reflected in chip_layout.ld or Rust
   userspace (and tests) will fail. */

MEMORY {
  FLASH (rx) : ORIGIN = 0x00076040, LENGTH = 0x0000FFC0
  SRAM (rwx) : ORIGIN = 0x00014000, LENGTH = 0x0000c000
}

MPU_MIN_ALIGN = 8K;

INCLUDE ../third_party/libtock-rs/layout_generic.ld
