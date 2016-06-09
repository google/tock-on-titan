
/*
 * Layout for boot_loader, residing at start of FLASH0
*/
MEMORY {
        rom (rx): ORIGIN = 0x44400 LENGTH = 60K
        ram (w):  ORIGIN = 0x10000 LENGTH = 64K
}
__stack_size__ = DEFINED(__stack_size__) ? __stack_size__ : 0x1000;
SECTIONS {

.text : { KEEP(*(.vectors)) *(.text) }
.tarpit : { *(.tarpit) }
.rodata : { *(.rodata*) }
/*
 * This is where ram starts.
 *
 * We want expectations data first, so we have known
 * address where boot_rom passes the structure to us.
 */
.guarded_data : {
        *(.expectations_data)
        /*
         * Put barrier between expectations and verify data
         */
        _guard0 = ABSOLUTE(.);
        . = . + 32;
        *(.verify_data)
        /*
         * Put barrier between verify data and .data
         */
        _guard1 = ABSOLUTE(.);
        . = . + 32;
  }
.data : { *(.data) }
.bss : {
        *(.bss)
        /*
         * Put up barrier at end of .bss to stop overruns
         * from reaching stack
         */
        _guard2 = ABSOLUTE(.);
        . = . + 32;

        _sstack = .;
        . = . + __stack_size__;
        . = ALIGN(8);
        _estack = .;

  }
}
