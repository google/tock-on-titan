// Copyright 2021 lowRISC contributors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

use core::time::Duration;

use manticore::crypto::rsa;
use manticore::hardware;
use manticore::protocol::capabilities::*;
use manticore::protocol::device_id;
use manticore::server::pa_rot::Options;
use manticore::server::pa_rot::PaRot;

const NETWORKING: Networking = Networking {
    max_message_size: 1024,
    max_packet_size: 256,
    mode: RotMode::Platform,
    roles: BusRole::HOST,
};

const TIMEOUTS: Timeouts = Timeouts {
    regular: Duration::from_millis(30),
    crypto: Duration::from_millis(200),
};

const DEVICE_ID: device_id::DeviceIdentifier =
    device_id::DeviceIdentifier {
        vendor_id: 1,
        device_id: 2,
        subsys_vendor_id: 3,
        subsys_id: 4,
    };

pub struct Identity {
    pub version: [u8; 32],
    pub device_id: [u8; 64],
}
impl hardware::Identity for Identity {
    fn firmware_version(&self) -> &[u8; 32] {
        &self.version
    }
    fn unique_device_identity(&self) -> &[u8] {
        &self.device_id
    }
}

pub struct Reset;
impl hardware::Reset for Reset {
    fn resets_since_power_on(&self) -> u32 {
        0
    }
    fn uptime(&self) -> Duration {
        Duration::from_millis(1)
    }
}

pub struct NoRsaPubKey;
impl rsa::PublicKey for NoRsaPubKey {
    fn len(&self) -> rsa::ModulusLength {
        unreachable!()
    }
}

pub struct NoRsaEngine;
impl rsa::Engine for NoRsaEngine {
    type Error = ();
    type Key = NoRsaPubKey;

    fn verify_signature(
        &mut self,
        _signature: &[u8],
        _message: &[u8],
    ) -> Result<(), ()> {
        Err(())
    }
}

pub struct NoRsa;
impl rsa::Builder for NoRsa {
    type Engine = NoRsaEngine;

    fn supports_modulus(&self, _: rsa::ModulusLength) -> bool {
        true
    }

    fn new_engine(&self, _key: NoRsaPubKey) -> Result<NoRsaEngine, ()> {
        Err(())
    }
}

pub fn get_pa_rot(identity: &Identity) -> PaRot<Identity, Reset, NoRsa> {
    PaRot::new(Options {
        identity: &identity,
        reset: &Reset,
        rsa: &NoRsa,
        device_id: DEVICE_ID,
        networking: NETWORKING,
        timeouts: TIMEOUTS,
    })
}
