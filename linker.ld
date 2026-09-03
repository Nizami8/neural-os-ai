/* Linker script for the Neural OS bare-metal RISC-V kernel.
 * QEMU `virt` loads a raw (-bios none) kernel image at 0x80000000 and jumps
 * there in M-mode, so that is our load/link address. */
OUTPUT_ARCH(riscv)
ENTRY(_start)

SECTIONS
{
    . = 0x80000000;

    .text : {
        *(.text.entry)   /* _start must come first */
        *(.text .text.*)
    }

    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }

    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }

    .bss : {
        _bss_start = .;
        *(.sbss .sbss.*)
        *(.bss .bss.*)
        *(COMMON)
        _bss_end = .;
    }

    /* Kernel stack (grows down from _stack_top). */
    . = ALIGN(16);
    . += 0x10000;
    _stack_top = .;

    /* Scratch heap region (bump allocator, currently unused by the kernel). */
    . = ALIGN(16);
    _heap_start = .;
    . += 0x20000;
    _heap_end = .;

    /* Durable neural-weight store. NOLOAD: not zeroed with BSS, so a soft
     * reset that preserves RAM can restore previously learned weights. */
    . = ALIGN(16);
    .persist (NOLOAD) : {
        _persist_start = .;
        *(.persist)
        _persist_end = .;
    }

    /DISCARD/ : {
        *(.eh_frame)
        *(.comment)
        *(.note .note.*)
    }
}
