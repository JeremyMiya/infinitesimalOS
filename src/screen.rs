//! 最小位图文字输出：直接写帧缓冲，不使用堆、GPU 驱动或整屏像素副本。
use crate::{boot::Framebuffer, font};
use core::{fmt, ptr};

pub struct Screen {
    address: *mut u8,
    pub width: usize,
    pub height: usize,
    pitch: usize,
    bytes_per_pixel: usize,
    masks: [(u8, u8); 3],
}

impl Screen {
    /// # Safety
    /// 调用者保证 fb 显存映射有效，可写 height * pitch 字节，且由本对象独占。
    /// pitch 是每行字节数，可能大于 width * 每像素字节数，不能直接按宽度寻址。
    pub unsafe fn new(fb: &Framebuffer) -> Option<Self> {
        if fb.address.is_null() || fb.memory_model != 1 || ![16, 24, 32].contains(&fb.bpp) {
            return None;
        }
        let (width, height, pitch) = (fb.width as usize, fb.height as usize, fb.pitch as usize);
        let bytes_per_pixel = usize::from(fb.bpp) / 8;
        let span = pitch.checked_mul(height)?;
        if width == 0
            || height == 0
            || width > 16384
            || height > 16384
            || pitch < width.checked_mul(bytes_per_pixel)?
            || span > isize::MAX as usize
            || (fb.address as usize).checked_add(span).is_none()
        {
            return None;
        }
        // 固件可能使用 RGB 或 BGR 排列；按协议掩码转换，不能假定固定颜色顺序。
        let masks = [
            (fb.red_mask_size, fb.red_mask_shift),
            (fb.green_mask_size, fb.green_mask_shift),
            (fb.blue_mask_size, fb.blue_mask_shift),
        ];
        let mut occupied = 0u32;
        for (size, shift) in masks {
            if size == 0 || size > 8 || u16::from(size) + u16::from(shift) > fb.bpp {
                return None;
            }
            let mask = ((1u32 << size) - 1) << shift;
            if occupied & mask != 0 {
                return None;
            }
            occupied |= mask;
        }
        Some(Self {
            address: fb.address,
            width,
            height,
            pitch,
            bytes_per_pixel,
            masks,
        })
    }

    // 将 24 位 RGB 颜色缩放到显卡实际提供的 16/24/32 位像素格式。
    fn native_color(&self, rgb: u32) -> u32 {
        let components = [(rgb >> 16) & 255, (rgb >> 8) & 255, rgb & 255];
        let mut pixel = 0;
        for (component, (size, shift)) in components.into_iter().zip(self.masks) {
            pixel |= (component >> (8 - size)) << shift;
        }
        pixel
    }

    // 饱和加法和屏幕边界裁剪防止溢出；越界的文字自然被裁掉。
    pub fn rect(&mut self, x: usize, y: usize, w: usize, h: usize, rgb: u32) {
        let end_x = x.saturating_add(w).min(self.width);
        let end_y = y.saturating_add(h).min(self.height);
        let color = self.native_color(rgb);
        for row in y..end_y {
            for col in x..end_x {
                // 安全前提：坐标已裁剪，地址范围经过 new 检查。volatile 保留硬件写入。
                // 对齐时一次写完整像素；24 位或未对齐情况逐字节写入。
                unsafe {
                    let p = self
                        .address
                        .add(row * self.pitch + col * self.bytes_per_pixel);
                    if self.bytes_per_pixel == 4 && (p as usize) % 4 == 0 {
                        ptr::write_volatile(p.cast::<u32>(), color);
                    } else if self.bytes_per_pixel == 2 && (p as usize) % 2 == 0 {
                        ptr::write_volatile(p.cast::<u16>(), color as u16);
                    } else {
                        for byte in 0..self.bytes_per_pixel {
                            ptr::write_volatile(p.add(byte), (color >> (8 * byte)) as u8);
                        }
                    }
                }
            }
        }
    }

    // 5×7 点阵按整数倍放大；每个字形只需要 7 字节，无需大字体文件。
    pub fn text(&mut self, x: usize, y: usize, scale: usize, color: u32, text: &str) {
        let mut pen = x;
        for c in text.bytes() {
            if pen.saturating_add(5 * scale) > self.width {
                break;
            }
            for (row, bits) in font::glyph(c).into_iter().enumerate() {
                for col in 0..5 {
                    if bits & (1 << (4 - col)) != 0 {
                        self.rect(pen + col * scale, y + row * scale, scale, scale, color);
                    }
                }
            }
            pen += 6 * scale;
        }
    }

    pub fn write_at(
        &mut self,
        x: usize,
        y: usize,
        scale: usize,
        color: u32,
        args: fmt::Arguments<'_>,
    ) {
        use fmt::Write;
        let _ = Text {
            screen: self,
            x,
            y,
            scale,
            color,
        }
        .write_fmt(args);
    }
}

// 实现 core::fmt::Write 接口，将格式化片段直接写屏幕，不创建临时 String。
struct Text<'a> {
    screen: &'a mut Screen,
    x: usize,
    y: usize,
    scale: usize,
    color: u32,
}

impl fmt::Write for Text<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.screen.text(self.x, self.y, self.scale, self.color, s);
        self.x = self
            .x
            .saturating_add(s.len().saturating_mul(6 * self.scale));
        Ok(())
    }
}
