pub unsafe extern "C" fn generic_isr() {
    let nvic: u32;

    asm!("mrs $0, IPSR" : "=r"(nvic));

    let nvic = (nvic & 0xff) - 16;
    ::cortexm3::nvic::Nvic::new(nvic).disable();
}

#[no_mangle]
#[cfg_attr(rustfmt, rustfmt_skip)]
pub static INTERRUPT_TABLE: [unsafe extern fn(); 203] = [generic_isr; 203];
