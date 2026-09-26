//! 可选 COM1 调试日志。Mac 的可见输出由 screen 提供，不假定存在物理串口。
//! 所有等待都有上限：设备缺失或无响应时关闭日志，保证内核还能继续显示。
use core::{arch::asm, fmt};

pub struct Serial {
    present: bool,
}

unsafe fn out(port: u16, byte: u8) {
    // 安全前提：ring 0、关中断，且访问限定为 COM1 端口。
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") byte, options(nomem, nostack));
    }
}

unsafe fn input(port: u16) -> u8 {
    let byte: u8;
    // 安全前提：与 out 相同，端口 I/O 只能在允许访问硬件的特权级执行。
    unsafe {
        asm!("in al, dx", in("dx") port, out("al") byte, options(nomem, nostack));
    }
    byte
}

impl Serial {
    pub fn new() -> Self {
        // 安全前提：只有启动 CPU 在 ring 0 使用串口。未实现端口通常读到 0xff；
        // 即使探测不准确，后续发送超时也会退出，绝不无限等待。
        unsafe {
            if input(0x3fd) == 0xff {
                return Self { present: false };
            }
            out(0x3f9, 0); // 禁用串口中断：本阶段使用轮询。
            out(0x3fb, 0x80); // 打开 DLAB，访问波特率分频寄存器。
            out(0x3f8, 1); // 分频 1：标准 UART 时钟下为 115200 波特。
            out(0x3f9, 0);
            out(0x3fb, 3); // 关闭 DLAB，设置 8 数据位、无校验、1 停止位。
            out(0x3fa, 0xc7); // 启用并清空收发 FIFO。
            out(0x3fc, 3); // 拉起 DTR / RTS。
        }
        Self { present: true }
    }

    fn byte(&mut self, value: u8) {
        if !self.present {
            return;
        }
        for _ in 0..10000 {
            // 安全前提：ring 0，且没有其他代码并发操作此 UART。
            unsafe {
                if input(0x3fd) & 0x20 != 0 {
                    out(0x3f8, value);
                    return;
                }
            }
            core::hint::spin_loop();
        }
        self.present = false;
    }
}

impl fmt::Write for Serial {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for byte in text.bytes() {
            if byte == b'\n' {
                self.byte(b'\r');
            }
            self.byte(byte);
        }
        Ok(())
    }
}
