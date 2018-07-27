use hotel::test_dcrypto::TestDcrypto;
use hotel::crypto::dcrypto;
#[allow(unused_imports)]
use hotel::crypto::dcrypto::{Dcrypto, DcryptoClient, DcryptoEngine};

pub unsafe fn run_dcrypto() {
    let r = static_init_test_dcrypto();
    dcrypto::DCRYPTO.set_client(r);
    r.run();
}

unsafe fn static_init_test_dcrypto() -> &'static mut TestDcrypto<'static> {
    static_init!(
        TestDcrypto<'static>,
        TestDcrypto::new(&dcrypto::DCRYPTO)
    )
}
