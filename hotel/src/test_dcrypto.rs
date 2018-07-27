//! Test DCRYPTO hardware

use core::cell::Cell;
use crypto::dcrypto::{Dcrypto, DcryptoClient, DcryptoEngine, ProgramFault};
use kernel::returncode::ReturnCode;

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
        static INSTRUCTIONS: [u32; 1] = [
            0x0c000000, // RET
        ];
        self.dcrypto.write_instructions(&INSTRUCTIONS, 0, 1);
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
        static INSTRUCTIONS: [u32; 2] = [
            // This instruction just calls itself: it's an infinitely
            // recursive program.
            0x08000000, // CALL 0
            0x08000000, // CALL 0
        ];
        self.dcrypto.write_instructions(&INSTRUCTIONS, 0, 2);
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
