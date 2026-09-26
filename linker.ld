/* ELF 装载图：代码 RX、常量 R、请求与数据 RW。
 * 不同权限的 PT_LOAD 分占 4 KiB 页面，防止 Limine 拒绝加载。
 * 请求和普通数据同为 RW，合用一个段，避免为少量请求浪费整页。
 */
OUTPUT_FORMAT(elf64-x86-64)
OUTPUT_ARCH(i386:x86-64)
ENTRY(_start)

PHDRS
{
    text   PT_LOAD FLAGS(5);
    rodata PT_LOAD FLAGS(4);
    data   PT_LOAD FLAGS(6);
}

SECTIONS
{
    /* 高半区静态地址；引导器建立页表，本阶段沿用该映射。 */
    . = 0xffffffff80000000;
    .text : { *(.text .text.*) } :text
    . = ALIGN(4096);
    .rodata : { *(.rodata .rodata.*) } :rodata
    . = ALIGN(4096);
    /* 扫描标记与请求必须保留，且保持 START → 请求 → END 顺序。 */
    .requests : ALIGN(8) {
        KEEP(*(.requests_start_marker))
        KEEP(*(.requests))
        KEEP(*(.requests_end_marker))
    } :data
    .data : { *(.data .data.*) *(.got .got.*) } :data
    /* .bss 只有内存大小，由引导器清零，不占同等大小的文件空间。 */
    .bss : { *(.bss .bss.*) *(COMMON) } :data
    /DISCARD/ : { *(.eh_frame*) *(.note*) }
}
