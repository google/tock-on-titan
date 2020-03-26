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

// Display the console output of the running h1 firmware. If --test is passed,
// this will reset the h1 (to restart the running tests) and exit when the
// tests are complete. If --test is passed, the return code indicates whether
// the tests were successful. If --test is not passed, this always returns
// success (even when interrupted); this allows it to be killed with an
// interrupt signal without causing `make` to throw an error.
//
// Prior to running this, the /dev/ttyUltraConsole3 and /dev/ttyUltraTarget2
// devices must be properly configured (115200 baud, echo off).

// Because ending executing via Ctrl-C (SIGINT) is the expected behavior for
// `make run`, we want to return 0 on SIGINT to minimize the error message from
// `make` in that case. This signal handler simply terminates the process when
// an interrupt is received, setting the exit code to 0.
extern "C" fn sigint_handler(_: libc::c_int) {
    unsafe { libc::_exit(0); }  // _exit() is signal-safe, exit() is not.
}

fn main() {
    use std::io::{Read,Write};

    let cmdline_matches = clap::App::new("runner")
        .arg(clap::Arg::with_name("delay").help("Reset delay in milliseconds")
             .long("delay").short("d").takes_value(true))
        .arg(clap::Arg::with_name("test").long("test").short("t"))
        .get_matches();

    // Parse the command line arguments early so that we fail fast (with a nice
    // error message) if we cannot parse them. This avoids resetting the H1 if
    // a bad command line argument is used.
    let delay = cmdline_matches.value_of("delay")
        .map_or(100, |d| d.parse().expect("Unable to parse --delay value"));

    // When this runner starts, the H1 will already be running. As a result, we
    // may have missed some of its output. This is particularly problematic for
    // --test, as we may have missed important markers.
    //
    // To collect as much of the H1's output as possible, we perform the
    // following sequence:
    //   1. Power down the H1 (write "0").
    //   2. Wait a bit (unfortunately we don't have a way to know for sure
    //      whether the device is powered down). Flush the target's console
    //      during/after the wait.
    //   3. Start listening to the debug console output (this happens
    //      implicitly).
    //   4. Power up the H1 (write "1").
    let mut debug_console = std::fs::OpenOptions::new()
                            .append(true)
                            .open("/dev/ttyUltraConsole3")
                            .expect("Unable to open /dev/ttyUltraConsole3");
    // 1. Power down the H1
    debug_console.write_all(b"0").expect("Unable to reset H1 (failed write)");
    debug_console.flush().expect("Unable to reset H1 (failed flush)");

    // 2. Wait for --delay milliseconds.
    std::thread::sleep(std::time::Duration::from_millis(delay));

    // 3. Open the console
    let target_console = std::fs::OpenOptions::new()
                         .read(true)
                         .open("/dev/ttyUltraTarget2")
                         .expect("Unable to open /dev/ttyUltraTarget2");

    // 4. Power up the H1.
    debug_console.write_all(b"1").expect("Unable to restart H1 (failed write)");
    debug_console.flush().expect("Unable to restart H1 (failed flush)");

    // If we're not in --test mode, return 0 on SIGINT.
    let test_mode = cmdline_matches.is_present("test");
    if !test_mode {
        unsafe { libc::signal(libc::SIGINT, sigint_handler as usize); }
    }

    // Stream in the console output, and echo it to stdout. If --test was
    // passed, we search for \nTEST_FINISHED: [FAIL|SUCCESS]\n and terminate
    // (with the corresponding error code) once found.
    let fail_message = b"\nTEST_FINISHED: FAIL\n";
    let success_message = b"\nTEST_FINISHED: SUCCESS\n";
    // The buffer length needs to match the larger of fail_message and
    // success_message.
    let mut buffer = vec![0; std::cmp::max(fail_message.len(), success_message.len())];
    for byte in target_console.bytes() {
        let byte = byte.expect("Console read error");
        std::io::stdout().write(&[byte]).expect("Failed to echo to stdout");

        if test_mode {
            // Rotate byte into the buffer (shifting the buffer contents 1 byte to
            // the left and appending byte).
            for i in 1..buffer.len() { buffer[i-1] = buffer[i]; }
            *buffer.last_mut().expect("empty buffer") = byte;

            if &buffer[success_message.len()-fail_message.len()..] == fail_message {
                // Return 3 to match Bazel's behavior (build successful but tests
                // failed).
                std::process::exit(3);
            }

            if &buffer == success_message {
                return;
            }
        }
    }

    // Unexpected: we received EOF but tests did not finish. Return 6 (Bazel's
    // "run failure" error message).
    println!("\nUnexpected EOF from target console.");
    std::process::exit(6);
}
