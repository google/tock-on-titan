use common::volatile_cell::VolatileCell;

#[repr(C, packed)]
struct Nvic {
    iser: [VolatileCell<u32>; 7],
    _reserved1: [u32; 25],
    icer: [VolatileCell<u32>; 7],
    _reserved2: [u32; 25],
    ispr: [VolatileCell<u32>; 7],
    _reserved3: [VolatileCell<u32>; 25],
    icpr: [VolatileCell<u32>; 7]

}

const BASE_ADDRESS: *mut Nvic = 0xe000e100 as *mut Nvic;

pub unsafe fn enable(signal: u32) {
    let nvic: &Nvic = &*BASE_ADDRESS;
    let idx = signal as usize;

    nvic.iser[idx / 32].set(1 << (signal & 31));
}

pub unsafe fn disable(signal: u32) {
    let nvic: &Nvic = &*BASE_ADDRESS;
    let idx = signal as usize;

    nvic.icer[idx / 32].set(1 << (signal & 31));
}

pub unsafe fn clear_pending(signal: u32) {
    let nvic: &Nvic = &*BASE_ADDRESS;
    let idx = signal as usize;

    nvic.icpr[idx / 32].set(1 << (signal & 31));
}

