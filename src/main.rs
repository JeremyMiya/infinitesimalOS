//! 内核入口：接收引导器交接 → 自检 → 输出结果 → 停机等待。
//! no_std 禁用依赖宿主系统的标准库；no_main 禁用普通应用程序的启动流程。
//! 本阶段没有分配器、线程、文件系统或中断处理，所有输出都同步完成。
#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

mod boot;
mod font;
mod runtime;
mod screen;
mod serial;

use core::{arch::asm, fmt::Write, panic::PanicInfo, ptr};
use screen::Screen;

// 保留用户版本的装载自检：零初值进入 .bss，非零初值进入 .data。
// volatile 强制实际访问内存，避免优化器直接代入已知初始值而让测试失效。
static mut BSS_PROBE: u64 = 0;
static mut DATA_PROBE: u64 = 0x1234_5678;

fn runtime_ok() -> bool {
    // 安全前提：只有启动 CPU 运行；中断关闭；没有其他代码访问这两个变量。
    unsafe {
        let zero = ptr::read_volatile(&raw const BSS_PROBE);
        let data = ptr::read_volatile(&raw const DATA_PROBE);
        ptr::write_volatile(&raw mut BSS_PROBE, 42);
        ptr::write_volatile(&raw mut DATA_PROBE, 43);
        zero == 0
            && data == 0x1234_5678
            && ptr::read_volatile(&raw const BSS_PROBE) == 42
            && ptr::read_volatile(&raw const DATA_PROBE) == 43
    }
}

fn halt() -> ! {
    loop {
        // 安全前提：CPU 在 ring 0。未安装 IDT，因此不能打开中断；hlt 避免忙等。
        unsafe {
            asm!("cli", "hlt", options(nomem, nostack));
        }
    }
}

// 链接脚本指定这个符号为 ELF 入口。extern C 遵循 Limine 要求的调用约定。
// Limine 已建立页表、提供对齐的栈；禁止在普通 Rust 函数内部擅自替换 rsp。
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    // 安全前提：引导器已进入 64 位 ring 0。cld 保证字符串操作方向符合 ABI。
    unsafe {
        asm!("cli", "cld", options(nomem, nostack));
    }
    let mut log = serial::Serial::new();
    let _ = writeln!(log, "infinitesimalOS ENTRY x86_64");
    if !boot::supported() {
        let _ = writeln!(log, "BOOT_FAIL: unsupported Limine base revision");
        halt();
    }

    let cs: u16;
    // CS 最低两位是实际权限级别：读取它来验证 ring 0，而非硬编码成功信息。
    unsafe {
        asm!("mov {0:x}, cs", out(reg) cs, options(nomem, nostack, preserves_flags));
    }
    let cpl = cs & 3;
    let initialized = runtime_ok();
    let memory = boot::usable_bytes();
    let firmware = boot::firmware();
    let _ = writeln!(log, "FIRMWARE={firmware} CPL={cpl} BASE_REVISION=6");
    let _ = writeln!(
        log,
        "RUNTIME_SELFTEST={} USABLE_BYTES={}",
        if initialized { "PASS" } else { "FAIL" },
        memory.unwrap_or(0)
    );

    let Some(fb) = boot::framebuffer() else {
        let _ = writeln!(log, "BOOT_FAIL: no framebuffer");
        halt();
    };
    // 安全前提：帧缓冲由 Limine 映射；整个内核只创建一个写入者。
    let Some(mut screen) = (unsafe { Screen::new(fb) }) else {
        let _ = writeln!(log, "BOOT_FAIL: invalid framebuffer");
        halt();
    };
    let _ = writeln!(
        log,
        "FRAMEBUFFER={}x{} BPP={} PITCH={}",
        fb.width, fb.height, fb.bpp, fb.pitch
    );
    let success = cpl == 0 && initialized && memory.is_some_and(|n| n > 0);

    // 只保留文字诊断，不引入窗口、图片或整屏后备缓冲。清屏直接写显存。
    screen.rect(0, 0, screen.width, screen.height, 0);
    let scale = if screen.width >= 800 && screen.height >= 600 {
        2
    } else {
        1
    };
    let mut y = 16;
    // format_args! 生成借用参数；没有 String，也不会分配堆内存。
    macro_rules! row {
        ($($arg:tt)*) => {{
            screen.write_at(16, y, scale, 0xffffff, format_args!($($arg)*));
            y += 12 * scale;
        }};
    }
    row!("infinitesimalOS 0.1");
    row!("{}", if success { "BOOT_OK" } else { "BOOT_FAIL" });
    row!("FIRMWARE: {firmware}");
    row!("CPU: RING {cpl}");
    row!("USABLE RAM: {} MIB", memory.unwrap_or(0) / 1024 / 1024);
    row!("SCREEN: {} X {} / {} BPP", fb.width, fb.height, fb.bpp);
    row!("DATA / BSS: {}", if initialized { "PASS" } else { "FAIL" });
    row!("RAM ONLY / NO DISK DRIVER");
    row!("KEYBOARD NOT IMPLEMENTED");
    screen.text(16, y, scale, 0xffffff, "HOLD POWER BUTTON TO EXIT");

    // 安全前提：x86_64 支持 sfence；确保显存写入提交后才发布成功标记。
    unsafe {
        asm!("sfence", options(nostack, preserves_flags));
    }
    let _ = writeln!(
        log,
        "{} FRAMEBUFFER_RENDERED RAM_ONLY NO_DISK_DRIVER",
        if success { "BOOT_OK" } else { "BOOT_FAIL" }
    );
    halt()
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    // 尚未引入可重入全局屏幕状态；panic 的详细信息通过可选串口输出。
    // 安全前提：单 CPU 的特权代码，中断关闭后无其他串口写入者。
    unsafe {
        asm!("cli", options(nomem, nostack));
    }
    let _ = writeln!(serial::Serial::new(), "BOOT_FAIL: PANIC {info}");
    halt()
}
