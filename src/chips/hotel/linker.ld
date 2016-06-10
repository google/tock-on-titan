MEMORY {
        rom (rx): ORIGIN = 0x44400 LENGTH = 60K
        ram (w):  ORIGIN = 0x10000 LENGTH = 64K
}
__stack_size__ = DEFINED(__stack_size__) ? __stack_size__ : 0x1000;
SECTIONS {
  .text : {
    KEEP(*(.vectors))
    *(.text)

    _sapps = .;
  } > rom

  .rodata : {
    *(.rodata*)
  } > rom

  _ero = .;

  .data : AT(_ero) {
    . = ALIGN(4);
    _sdata = .;
    *(.data)
    _edata = .;
  } > ram

  .bss : {
    . = ALIGN(4);
    _sbss = .;
    *(.bss)
    _ebss = .;

    _sstack = .;
    . = . + __stack_size__;
    . = ALIGN(8);
    _estack = .;
  } > ram

}
