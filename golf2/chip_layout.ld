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

/* Note: modifications to prog and appram should be reflected in
   userspace/layout.ld or Rust userspace (and tests) will fail. */
MEMORY
{
  rom (rx)     : ORIGIN = 0x00044400, LENGTH = 0x00031c00
  prog (rx)    : ORIGIN = 0x00076000, LENGTH = 0x00010000
  ram (rwx)    : ORIGIN = 0x00010000, LENGTH = 0x00004000
  appram (rwx) : ORIGIN = 0x00014000, LENGTH = 0x0000c000
}

MPU_MIN_ALIGN = 8K;
