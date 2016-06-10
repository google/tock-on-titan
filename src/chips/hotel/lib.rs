#![crate_name = "hotel"]
#![crate_type = "rlib"]
#![no_std]
#![feature(asm,const_fn)]

extern crate common;
extern crate hil;
extern crate support;

pub mod gpio;
pub mod pinmux;
pub mod pmu;
pub mod uart;

unsafe extern "C" fn unhandled_interrupt() {
    let mut interrupt_number: u32;

    // IPSR[8:0] holds the currently active interrupt
    asm!(
        "mrs    r0, ipsr                    "
        : "={r0}"(interrupt_number)
        :
        : "r0"
        :
        );

    interrupt_number = interrupt_number & 0x1ff;

    panic!("Unhandled Interrupt. ISR {} is active.", interrupt_number);
}

extern {
    // _estack is not really a function, but it makes the types work
    // You should never actually invoke it!!
    fn _estack();

    // Defined in src/main/main.rs
    fn main();

    fn SVC_Handler();

    static mut _ero : u32;
    static mut _sdata : u32;
    static mut _edata : u32;
    static mut _sbss : u32;
    static mut _ebss : u32;
}

// marks the end of the app code segment with a null byte
#[link_section=".endapp"]
pub static ENDAPP: usize = 0;

#[link_section=".vectors"]
pub static ISR_VECTOR: [Option<unsafe extern fn()>; 16] = [
    /* Stack top */     Option::Some(_estack),
    /* Reset */         Option::Some(reset_handler),
    /* NMI */           Option::Some(unhandled_interrupt),
    /* Hard Fault */    Option::Some(unhandled_interrupt),
    /* MemManage */     Option::Some(unhandled_interrupt),
    /* BusFault */      Option::Some(unhandled_interrupt),
    /* UsageFault*/     Option::Some(unhandled_interrupt),
    None, None, None, None,
    /* SVC */           Option::Some(SVC_Handler),
    /* DebugMon */      Option::Some(unhandled_interrupt),
    None,
    /* PendSV */        Option::Some(unhandled_interrupt),
    /* SysTick */       Option::Some(unhandled_interrupt),
];

unsafe extern "C" fn reset_handler() {
    // Relocate data segment.
    // Assumes data starts right after text segment as specified by the linker
    // file.
    let mut pdest  = &mut _sdata as *mut u32;
    let pend  = &mut _edata as *mut u32;
    let mut psrc = &_ero as *const u32;

    if psrc != pdest {
        while (pdest as *const u32) < pend {
            *pdest = *psrc;
            pdest = pdest.offset(1);
            psrc = psrc.offset(1);
        }
    }

    // Clear the zero segment (BSS)
    let pzero = &_ebss as *const u32;
    pdest = &mut _sbss as *mut u32;

    while (pdest as *const u32) < pzero {
        *pdest = 0;
        pdest = pdest.offset(1);
    }

    main()
}

