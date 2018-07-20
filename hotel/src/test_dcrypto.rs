//! Test DCRYPTO hardware


use kernel::returncode::ReturnCode;
#[allow(unused_imports)]
use crypto::dcrypto::{Dcrypto, DcryptoClient, DcryptoEngine};

pub struct TestDcrypto<'a> {
    dcrypto: &'a DcryptoEngine<'a>,
}

impl<'a> TestDcrypto<'a> {
    pub fn new(d: &'a DcryptoEngine<'a>) -> Self {
        TestDcrypto { dcrypto: d }
    }

    pub fn run(&self) {
        static INSTRUCTIONS: [u32; 1] = [
            0x0c000000, // RET
        ];
        self.dcrypto.write_instructions(&INSTRUCTIONS, 0, 1);
        self.dcrypto.call_imem(0);
    }
}

impl<'a> DcryptoClient<'a> for TestDcrypto<'a> {
    fn execution_complete(&self, error: ReturnCode) {
        println!("Execution of program completed with ReturnCode {:?}.", error);
    }

    fn reset_complete(&self, _error: ReturnCode) {
        println!("ERROR: Dcrypto test: reset_complete invoked, but should never be called.");
    }

    fn secret_wipe_complete(&self, _error: ReturnCode) {
        println!("ERROR: Dcrypto test: secret_wipe_complete invoked, but should never be called.");
    }

}
