// Copyright 2020 lowRISC contributors.
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

use clap::App;
use clap::AppSettings;
use clap::Arg;
use clap::SubCommand;

use core::convert::TryFrom;

use spiutils::io::StdWrite;
use spiutils::io::Write;
use spiutils::protocol::payload;
use spiutils::protocol::wire::FromWire;
use spiutils::protocol::wire::ToWire;

use std::fs::OpenOptions;
use std::io::Read as _;

fn wrap(input_file: &str, output_file: &str) {
    let mut input = OpenOptions::new()
        .read(true)
        .open(&input_file)
        .expect("failed to open input file");
    let mut output = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(&output_file)
        .expect("failed to open output file");

    let mut read_buf = Vec::new();
    input
        .read_to_end(&mut read_buf)
        .expect("couldn't read from file");

    let header = payload::Header {
        content: payload::ContentType::Manticore,
        content_len: u16::try_from(read_buf.len()).unwrap(),
    };

    let mut stdwrite = StdWrite(&mut output);
    header
        .to_wire(&mut stdwrite)
        .expect("failed to write header");
    stdwrite
        .write_bytes(&read_buf.as_slice())
        .expect("failed to write payload");
}

fn unwrap(input_file: &str, output_file: &str) {
    let mut input = OpenOptions::new()
        .read(true)
        .open(&input_file)
        .expect("failed to open input file");
    let mut output = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(&output_file)
        .expect("failed to open output file");

    let mut read_buf = Vec::new();
    input
        .read_to_end(&mut read_buf)
        .expect("couldn't read from file");

    let mut read_buf_slice = read_buf.as_slice();
    println!("read_buf_slice.len={}", read_buf_slice.len());
    let header = payload::Header::from_wire(&mut read_buf_slice).expect("failed to read header");

    match header.content {
        payload::ContentType::Manticore => {
            let mut stdwrite = StdWrite(&mut output);
            stdwrite
                .write_bytes(&mut &read_buf_slice[..header.content_len as usize])
                .expect("failed to write payload");
        }
        _ => {
            panic!("Unsupported content type {:?}", header.content);
        }
    }
}

fn main() {
    let app = App::new("SPI Transport Tool")
        .version("0.1")
        .author("lowRISC contributors")
        .about("Command line tool for SPI Transport library")
        .setting(AppSettings::ArgRequiredElseHelp)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .subcommand(
            SubCommand::with_name("wrap")
                .about("Wrap a message")
                .arg(
                    Arg::with_name("input")
                        .short("i")
                        .long("input")
                        .help("input file containing unwrapped message")
                        .required(true)
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("output")
                        .short("o")
                        .long("output")
                        .help("output file for wrapped message")
                        .required(true)
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("unwrap")
                .about("Unwrap a message")
                .arg(
                    Arg::with_name("input")
                        .short("i")
                        .long("input")
                        .help("input file containing wrapped message")
                        .required(true)
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("output")
                        .short("o")
                        .long("output")
                        .help("output file for unwrapped message")
                        .required(true)
                        .takes_value(true),
                ),
        );
    let matches = app.get_matches();

    if let Some(matches) = matches.subcommand_matches("wrap") {
        wrap(
            matches.value_of("input").unwrap(),
            matches.value_of("output").unwrap(),
        );
    } else if let Some(matches) = matches.subcommand_matches("unwrap") {
        unwrap(
            matches.value_of("input").unwrap(),
            matches.value_of("output").unwrap(),
        );
    }
}
