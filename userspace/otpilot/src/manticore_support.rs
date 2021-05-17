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
use manticore::protocol::wire::FromWire;
use manticore::protocol::wire::FromWireError;
use manticore::protocol::wire::ToWire;
use manticore::protocol::wire::ToWireError;
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
    ) -> Result<(), rsa::Error<()>> {
        Err(manticore::crypto::rsa::Error::Custom(()))
    }
}

pub struct NoRsa;
impl rsa::Builder for NoRsa {
    type Engine = NoRsaEngine;

    fn supports_modulus(&self, _: rsa::ModulusLength) -> bool {
        true
    }

    fn new_engine(&self, _key: NoRsaPubKey) -> Result<NoRsaEngine, rsa::Error<()>> {
        Err(manticore::crypto::rsa::Error::Custom(()))
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

const ARENA_SIZE : usize = 64;
static mut ARENA: [u8; ARENA_SIZE] = [0; ARENA_SIZE];

#[derive(Copy, Clone, Debug)]
pub enum HandlerError {
    FromWire(FromWireError),
    ToWire(ToWireError),
    Manticore(manticore::server::Error),
    NoResponse,
}

impl From<FromWireError> for HandlerError {
    fn from(err: FromWireError) -> Self {
        HandlerError::FromWire(err)
    }
}

impl From<ToWireError> for HandlerError {
    fn from(err: ToWireError) -> Self {
        HandlerError::ToWire(err)
    }
}

impl From<manticore::server::Error> for HandlerError {
    fn from(err: manticore::server::Error) -> Self {
        HandlerError::Manticore(err)
    }
}

pub type HandlerResult<T> = Result<T, HandlerError>;

pub struct Handler<'a> {
    // The Handler protocol server.
    server: PaRot<'a, Identity, Reset, NoRsa>,
}

impl<'a> Handler<'a> {

    pub fn new(identity: &'a Identity) -> Self {
        Self {
            server: get_pa_rot(identity),
        }
    }

    pub fn process_request(&mut self, mut input: &[u8], output: &mut[u8]) -> HandlerResult<usize> {
        use manticore::mem::BumpArena;
        use manticore::net::InMemHost;
        use manticore::protocol::Header;
        use manticore::protocol::HEADER_LEN;
        use manticore::io::Cursor;

        let header = {
            unsafe {
                let arena = BumpArena::new(&mut ARENA[..]);
                Header::from_wire(&mut input, &arena)?
            }
        };

        let resp_header: Header;
        let resp_data_len: usize;
        {
            let mut host_port = InMemHost::new(&mut output[HEADER_LEN..]);
            host_port.request(header, input);

            unsafe {
                // TODO(osk): We need the unsafe block since we're accessing ARENA as &mut.
                let arena = BumpArena::new(&mut ARENA[..]);
                self.server.process_request(&mut host_port, &arena)?;
            }

            if let Some((resp, out)) = host_port.response() {
                resp_header = resp;
                resp_data_len = out.len();
            } else {
                return Err(HandlerError::NoResponse);
            }
        }

        let tx_cursor = Cursor::new(output);
        resp_header.to_wire(tx_cursor)?;
        Ok(resp_data_len + HEADER_LEN)
    }

}
