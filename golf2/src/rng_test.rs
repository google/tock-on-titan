use capsules::test::rng::TestRng;
use kernel::hil::rng::RNG;
use hotel::trng;

pub unsafe fn run_rng() {
    let r = static_init_test_rng();
    trng::TRNG0.set_client(r);
    r.run();
}

unsafe fn static_init_test_rng() -> &'static mut TestRng<'static> {
    static_init!(
        TestRng<'static>,
        TestRng::new(&trng::TRNG0)
    )
}
