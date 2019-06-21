# Copyright 2019 Google LLC
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

FROM launcher.gcr.io/google/debian9

ENV CARGO_HOME=/root/cargo \
    PATH=$PATH:/root/cargo/bin \
    RUSTUP_HOME=/root/rustup

RUN apt-get update && apt-get install --no-install-recommends -y curl make && \
    curl -sSf 'https://sh.rustup.rs/' | sh -s -- --default-toolchain none -y \
    && apt-get purge -y curl && apt-get -y autoremove

# ------------------------------------------------------------------------------
# Configure additional toolchains here in the next two commands. Note that we
# only need to add the thumbv7m target for toolchains used by embedded code --
# elf2tab only needs the host toolchain.
# ------------------------------------------------------------------------------
RUN rustup toolchain add nightly-2018-08-16 \
                         nightly-2018-11-30 \
                         stable

RUN rustup target add --toolchain nightly-2018-08-16 thumbv7m-none-eabi
RUN rustup target add --toolchain nightly-2018-11-30 thumbv7m-none-eabi

# Prevent rustup from trying to download new toolchains if the toolchains in
# rust-toolchain are updated and this image is not updated.
ENV RUSTUP_DIST_SERVER="https://rustup.invalid/"
