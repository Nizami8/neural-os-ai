OUTPUT_ARCH(riscv)
ENTRY(_start)

SECTIONS
{
    . = 0x80000000;

    .text : ALIGN(4) {
        KEEP(*(.text.boot))
        *(.text .text.*)
    }

    .rodata : ALIGN(8) {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }

    .data : ALIGN(8) {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }

    .bss : ALIGN(8) {
        _bss_start = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        *(COMMON)
        _bss_end = .;
    }

    . = ALIGN(16);
    _stack_start = .;
    . = . + 0x4000;
    _stack_top = .;

    . = ALIGN(16);
    _trap_stack_start = .;
    . = . + 0x2000;
    _trap_stack_top = .;

    _heap_start = ALIGN(4096);
}
