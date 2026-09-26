//! 编译器可能生成 memcpy 等调用；裸机没有 libc，必须提供对应符号。
//! volatile 字节循环防止 LLVM 将函数体再优化为对自身的调用，造成无限递归。
//! 本阶段优先简单、可审查；链接器会删除未被引用的函数。不是通用内存安全接口。
use core::ptr;

#[unsafe(no_mangle)]
unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    for i in 0..n {
        // 安全前提：调用者提供有效的 n 字节读写范围；memcpy 的外部调用不得重叠。
        unsafe {
            ptr::write_volatile(dest.add(i), ptr::read_volatile(src.add(i)));
        }
    }
    dest
}

#[unsafe(no_mangle)]
unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if (dest as usize) <= src as usize {
        // 安全前提：源、目标范围有效；此处复用逐字节实现，目标在前时顺序复制可处理重叠。
        unsafe { memcpy(dest, src, n) }
    } else {
        for i in (0..n).rev() {
            // 安全前提：范围有效；从末尾复制，避免目标在后时覆盖尚未读取的源数据。
            unsafe {
                ptr::write_volatile(dest.add(i), ptr::read_volatile(src.add(i)));
            }
        }
        dest
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn memset(dest: *mut u8, value: i32, n: usize) -> *mut u8 {
    for i in 0..n {
        // 安全前提：调用者提供可写的 n 字节范围。
        unsafe {
            ptr::write_volatile(dest.add(i), value as u8);
        }
    }
    dest
}

#[unsafe(no_mangle)]
unsafe extern "C" fn memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    for i in 0..n {
        // 安全前提：调用者提供两个可读的 n 字节范围。
        let (a, b) = unsafe { (ptr::read_volatile(a.add(i)), ptr::read_volatile(b.add(i))) };
        if a != b {
            return i32::from(a) - i32::from(b);
        }
    }
    0
}
