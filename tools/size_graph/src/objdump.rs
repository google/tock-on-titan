// Copyright 2019 Google LLC
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

use std::process::ChildStdout;

pub fn parse<H: FnMut(&[u8]) -> (), R: FnMut(&[u8]) -> ()>(
    stdout: ChildStdout, mut header: H, mut reference: R) -> std::io::Result<()>
{
    use std::io::Read;
    let mut bufreader = std::io::BufReader::new(stdout);
    let mut symbol = Vec::new();
    loop {
        let mut byte = 0;
        // Scan for a '<'; ignore all bytes up to and including the '<'.
        loop {
            if bufreader.read(std::slice::from_mut(&mut byte))? == 0 {
                return Ok(());
            }
            if byte == '<' as u8 { break; }
        }
        // Read out the symbol name. Look for a terminating '>' but do not
        // include it.
        loop {
            if bufreader.read(std::slice::from_mut(&mut byte))? == 0 {
                return Ok(());
            }
            if byte == '>' as u8 { break; }
            symbol.push(byte);
        }
        // Check for a trailing colon, indicating this is a symbol header. If
        // we've reached EOF, then assume it is a symbol reference.
        if bufreader.read(std::slice::from_mut(&mut byte))? == 0 {
            reference(&symbol);
            return Ok(());
        }
        if byte == ':' as u8 {
            header(&symbol);
        } else {
            reference(&symbol);
        }
        symbol.clear();
        // Ignore everything through the next newline.
        loop {
            if bufreader.read(std::slice::from_mut(&mut byte))? == 0 {
                return Ok(());
            }
            if byte == '\n' as u8 { break; }
        }
    }
}
