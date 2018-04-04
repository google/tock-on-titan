MEMORY {
        rom (rx): ORIGIN = 0x44400 LENGTH = 256K
        ram (w):  ORIGIN = 0x10000 LENGTH = 64K
}
__stack_size__ = DEFINED(__stack_size__) ? __stack_size__ : 0x1000;
SECTIONS {
  .text : {
    KEEP(*(.vectors))
    KEEP(*(.irqs))
    *(.text)
    /* codesigner needs start to be defined.
     * add 1 to set bit 0 (T-bit/Thumb mode). */
    start = reset_handler + 1;
    *(.ARM.extab* .gnu.linkonce.armextab.*)
  } > rom

  .apps 0x54000 :
  {
    _sapps = .;
    KEEP(*(.app.*))
    LONG(0) /* Marks the end of the app list with a sentinel value. */
    . = 64K;
  } > rom

  /* .ARM.exidx is sorted, so has to go in its own output section.  */
  PROVIDE_HIDDEN (__exidx_start = .);
  .ARM.exidx :
  {
    *(.ARM.exidx* .gnu.linkonce.armexidx.*)
  } > rom
  PROVIDE_HIDDEN (__exidx_end = .);

  .rodata : {
    *(.rodata*)
  } > rom

  _ero = .;

  .data : AT(_ero) {
    . = ALIGN(4);
    _sdata = .;
    *(.data .data.*)
    . = ALIGN(4);
    _edata = .;
  } > ram

  .bss (NOLOAD) : {
    . = ALIGN(4);
    _sbss = .;
    *(.bss .bss.*)
    _ebss = .;

    _sstack = .;
    . = . + __stack_size__;
    . = ALIGN(8);
    _estack = .;

    . = ALIGN(8K);
    *(.app_memory)
  } > ram

}
