//! 只定义本阶段使用的 Limine 协议字段，不引入完整协议 crate。
//! 对应固定的 Limine 12.9.0、基础修订版 6；来源及许可见 docs/PROVENANCE.md。
//! repr(C) 固定字段布局，让引导器与 Rust 按相同偏移读取结构体。
//! 引导器在进入 _start 前写回响应；本阶段不回收其内存、不换页表、不启动其他 CPU。

use core::{cell::UnsafeCell, ptr};

#[repr(C)]
struct Request<T> {
    id: [u64; 4],
    revision: u64,
    // 外部引导器会改写这一字段；UnsafeCell 避免把它误当作永不变化的常量。
    response: UnsafeCell<*const T>,
}

// 安全前提：只有引导器在 Rust 入口执行前写入响应，之后只读。
unsafe impl<T> Sync for Request<T> {}

impl<T> Request<T> {
    const fn new(id: [u64; 2]) -> Self {
        Self {
            id: [0xc7b1dd30df4c8b88, 0x0a82e883a194f07b, id[0], id[1]],
            revision: 0,
            response: UnsafeCell::new(ptr::null()),
        }
    }

    fn get(&self) -> Option<&'static T> {
        // 安全前提：非空指针由可信引导器映射并对齐；本阶段始终保留其底层内存。
        unsafe { ptr::read_volatile(self.response.get()).as_ref() }
    }
}

#[repr(transparent)]
struct BaseRevision(UnsafeCell<[u64; 3]>);
// 安全前提：与 Request 相同，写入只发生在进入内核之前。
unsafe impl Sync for BaseRevision {}

// used 保留静态对象，link_section 将请求放到链接脚本 KEEP 的扫描区间。
#[used]
#[unsafe(link_section = ".requests_start_marker")]
static START: [u64; 4] = [
    0xf6b8f4b39de7d1ae,
    0xfab91a6940fcb9cf,
    0x785c6ed015d3e316,
    0x181e920a7852b9d9,
];

#[used]
#[unsafe(link_section = ".requests")]
static BASE: BaseRevision =
    BaseRevision(UnsafeCell::new([0xf9562b2d5c95a6c8, 0x6a7b384944536bdc, 6]));

#[used]
#[unsafe(link_section = ".requests")]
static FRAMEBUFFER: Request<FramebufferResponse> =
    Request::new([0x9d5827dcd881dd75, 0xa3148604f6fab11b]);

#[used]
#[unsafe(link_section = ".requests")]
static MEMMAP: Request<MemoryMapResponse> = Request::new([0x67cf3d9d378a806f, 0xe304acdfc50c3c62]);

#[used]
#[unsafe(link_section = ".requests")]
static FIRMWARE: Request<FirmwareResponse> = Request::new([0x8c2f75d90bef28a8, 0x7045a4688eac00c3]);

#[used]
#[unsafe(link_section = ".requests_end_marker")]
static END: [u64; 2] = [0xadc0e0531bb10d03, 0x9572709f31764c62];

#[repr(C)]
pub struct Framebuffer {
    pub address: *mut u8,
    pub width: u64,
    pub height: u64,
    pub pitch: u64,
    pub bpp: u16,
    pub memory_model: u8,
    pub red_mask_size: u8,
    pub red_mask_shift: u8,
    pub green_mask_size: u8,
    pub green_mask_shift: u8,
    pub blue_mask_size: u8,
    pub blue_mask_shift: u8,
    _unused: [u8; 7],
    _edid_size: u64,
    _edid: *const u8,
    // 只声明请求修订版 0 的前缀；不访问协议后续添加的字段。
}

#[repr(C)]
struct FramebufferResponse {
    revision: u64,
    count: u64,
    framebuffers: *const *const Framebuffer,
}

#[repr(C)]
struct MemoryMapResponse {
    revision: u64,
    count: u64,
    entries: *const *const MemoryEntry,
}

#[repr(C)]
struct MemoryEntry {
    base: u64,
    length: u64,
    kind: u64,
}

#[repr(C)]
struct FirmwareResponse {
    revision: u64,
    kind: u64,
}

pub fn supported() -> bool {
    // 安全前提：Limine 已完成写回。第三个数变成 0 表示支持请求的基础修订版。
    unsafe { ptr::read_volatile(BASE.0.get().cast::<u64>().add(2)) == 0 }
}

pub fn firmware() -> &'static str {
    match FIRMWARE.get().map(|r| r.kind) {
        Some(0) => "BIOS",
        Some(1) => "UEFI 32-BIT",
        Some(2) => "UEFI 64-BIT",
        _ => "UNKNOWN",
    }
}

pub fn framebuffer() -> Option<&'static Framebuffer> {
    let response = FRAMEBUFFER.get()?;
    if response.count == 0 || response.framebuffers.is_null() {
        return None;
    }
    // 安全前提：引导器给出的指针数组有效；只取第一个显示器，不释放其内存。
    unsafe { (*response.framebuffers).as_ref() }
}

/// 统计引导时标为可用的 RAM；这不是内核实际占用量，也不是分配器。
pub fn usable_bytes() -> Option<u64> {
    let response = MEMMAP.get()?;
    if response.count == 0 || response.count > 65536 || response.entries.is_null() {
        return None;
    }
    let mut usable = 0u64;
    for i in 0..response.count as usize {
        // 安全前提：下标小于可信响应的 count；这里只统计范围，不分配或读写该 RAM。
        let entry = unsafe { (*response.entries.add(i)).as_ref()? };
        if entry.kind == 0 {
            usable = usable.checked_add(entry.length)?;
        }
    }
    Some(usable)
}

// 编译期验证关键大小和偏移；改错协议结构时直接报错，避免启动后读取错误地址。
const _: () = {
    assert!(core::mem::size_of::<Request<u64>>() == 48);
    assert!(core::mem::offset_of!(Request<u64>, response) == 40);
    assert!(core::mem::size_of::<Framebuffer>() == 64);
    assert!(core::mem::offset_of!(Framebuffer, bpp) == 32);
    assert!(core::mem::size_of::<MemoryEntry>() == 24);
};
