// Copyright 2018 Google LLC
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

//! Test DCRYPTO hardware

use core::cell::Cell;
use crate::crypto::dcrypto::{Dcrypto, DcryptoClient, DcryptoEngine, ProgramFault};
use kernel::ReturnCode;

#[derive(Clone, Copy, Debug, PartialEq)]
enum TestCase {
    None,
    SuccessfulExecution,
    StackError,
}

pub struct TestDcrypto<'a> {
    dcrypto: &'a DcryptoEngine<'a>,
    case: Cell<TestCase>,
}

impl<'a> TestDcrypto<'a> {
    pub fn new(d: &'a DcryptoEngine<'a>) -> Self {
        TestDcrypto {
            dcrypto: d,
            case: Cell::new(TestCase::None),
        }
    }

    pub fn run(&self) {
        self.start_test_exec();
    }

    fn start_test_exec(&self) {
        self.case.set(TestCase::SuccessfulExecution);
        println!("DCRYPTO Testing single-instruction program that returns.");
        static INSTRUCTIONS: [u8; 4] = [
            0x00, 0x00, 0x00, 0x0c, // RET
        ];
        self.dcrypto.write_instructions(&INSTRUCTIONS, 0, 4);
        self.dcrypto.call_imem(0);
    }

    fn complete_test_exec(&self, error: ReturnCode, fault: ProgramFault) {
        if error == ReturnCode::SUCCESS {
            println!("DCRYPTO pass: Program completed with ReturnCode {:?}.", error);
        } else {
            println!("DCRYPTO fail: Program completed with fault {:?}.", fault);
        }
    }

    fn start_test_stack(&self) {
        self.case.set(TestCase::StackError);
        println!("DCRYPTO Testing program that overflows call stack.");
        static INSTRUCTIONS: [u8; 8] = [
            // This instruction just calls itself: it's an infinitely
            // recursive program. It should trigger a PC stack overflow
            // error.
            //
            // Following it with a BREAK instruction prevents
            // a subsequent TRAP interrupt, I do not know why. -pal
            0x00, 0x00, 0x00, 0x08, // CALL 0
            0x00, 0x00, 0x00, 0x00, // BREAK
        ];
        self.dcrypto.write_instructions(&INSTRUCTIONS, 0, 8);
        self.dcrypto.call_imem(0);
    }

    // A PC stack overflow raises two interrupts, first an overflow then
    // a trap. 
    fn complete_test_stack(&self, error: ReturnCode, fault: ProgramFault) {
        if error == ReturnCode::FAIL && fault == ProgramFault::StackOverflow {
            println!("DCRYPTO pass: Program completed with fault {:?}.", fault);
        } else if error == ReturnCode::FAIL && fault == ProgramFault::Trap {
            println!("DCRYPTO pass: Program completed with fault {:?}.", fault);
            self.case.set(TestCase::None);
        }
        else {
            println!("DCRYPTO fail: program completed with ReturnCode {:?} and fault {:?}.", error, fault);
        }
    }
}

impl<'a> DcryptoClient<'a> for TestDcrypto<'a> {
    fn execution_complete(&self, error: ReturnCode, fault: ProgramFault) {
        match self.case.get() {
            TestCase::SuccessfulExecution => {
                self.complete_test_exec(error, fault);
                self.start_test_stack();
            }
            TestCase::StackError => {
                self.complete_test_stack(error, fault);
            }
            TestCase::None => {
                println!("DCRYPTO received execution complete for no test case.");
            }
        }
        if self.case.get() == TestCase::None {
            println!("DCRYPTO all tests passed!");
        }
    }

    fn reset_complete(&self, _error: ReturnCode) {
        println!("ERROR: Dcrypto test: reset_complete invoked, but should never be called.");
    }

    fn secret_wipe_complete(&self, _error: ReturnCode) {
        println!("ERROR: Dcrypto test: secret_wipe_complete invoked, but should never be called.");
    }

}
