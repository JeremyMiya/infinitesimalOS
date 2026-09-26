# infinitesimalOS

用 Rust 学习如何制作操作系统，并在明确的功能边界内持续压缩体积。
长期目标是挑战“功能完整条件下的世界最小操作系统”；目前是能启动、能显示诊断信息的内核骨架，尚未达到完整操作系统，也未证明世界最小。

目标硬件：x86_64 虚拟机，以及 ShadowSecretary（MacBook Air 11 英寸 Early 2015）的 Ventoy Live 启动。独立于其他操作系统项目。

## 当前功能

- 通过 Limine 进入 64 位 ring 0，沿用引导器提供的页表和栈。
- 实际读写 `.data` / `.bss`，检查装载和清零是否成功。
- 可选 COM1 串口日志；设备无响应会超时退出，不阻塞屏幕输出。
- 16 / 24 / 32 位 RGB 帧缓冲文字输出，显示固件、权限级别、可用 RAM 和自检结果。
- 读取引导时内存地图；目前只统计，不负责内存分配。
- 生成含 UEFI / BIOS 引导的 ISO；已通过 QEMU + OVMF、SeaBIOS、Ventoy 1.1.16 Memdisk 模式的虚拟机测试。

**尚未实现**：键盘输入、命令行、IDT / IRQ、分配器、任务调度、用户态、文件系统、磁盘、网络。ShadowSecretary 实机待验证；虚拟机通过不代表实机已通过。

## 构建和运行（Ubuntu）

先安装 Rust / rustup。仓库固定 Rust 1.90.0 和 `x86_64-unknown-none` 目标，进入项目执行 Cargo 时由 rustup 按 `rust-toolchain.toml` 提供工具链。

```sh
sudo apt install make gcc python3 xorriso mtools qemu-system-x86 ovmf
git clone https://github.com/JeremyMiya/infinitesimalOS.git
cd infinitesimalOS
make test
make run
```

首次制作 ISO 会从官方 GitHub 发行页下载固定版本 Limine 12.9.0，并校验归档和各文件 SHA256；后续使用本地缓存。Rust 内核没有第三方 crate，Cargo 构建使用 `--locked --offline`，不访问 crates.io。

`make run` 打开 QEMU 图形窗口，关闭窗口退出；串口保存在 `build/test-uefi/serial.log`。`make test` 无界面执行实际启动，保存日志、屏幕图和 CPU 状态，检查 `BOOT_OK`、ring 0、自检和帧缓冲。

| 命令 | 用途 |
| --- | --- |
| `make build` | 构建发布内核 |
| `make iso` | 输出 `build/infinitesimalOS.iso` |
| `make inspect` | 检查 ELF 段布局和 32 KiB 体积预算 |
| `make test-bios` | 额外检查保留的 BIOS 启动路径 |
| `make fmt` | 检查 Rust 格式 |
| `cargo build --locked --offline` | 保留调试信息的 dev 构建 |
| `make clean` | 删除生成文件和引导器缓存 |

如果固件路径不同，设置配套的 `OVMF_CODE` / `OVMF_VARS`；例如 Ubuntu 的 `/usr/share/OVMF/OVMF_CODE_4M.fd` 与 `/usr/share/OVMF/OVMF_VARS_4M.fd`。不要混用不同大小或不同类型的固件对。

离线准备引导器：`python3 tools/fetch_limine.py --archive /path/to/limine-binary.tar.xz`。仍会校验固定 SHA256。工具链本身也须提前安装。

## Ventoy / ShadowSecretary

1. 执行 `make iso`，将 `build/infinitesimalOS.iso` 复制到现有 Ventoy 数据分区。
2. 在 ShadowSecretary 开机选择 Ventoy U 盘，再选择此 ISO。
3. 当前已验证的路径是 **Memdisk 模式**。按 Ventoy 的菜单提示选择该模式；普通模式不列为已通过。
4. 预期看到黑底白字的 `INFINITESIMALOS 0.1`、`BOOT_OK`、`DATA / BSS: PASS`。
5. 暂无键盘及关机驱动；测试完可长按电源关机。内核没有磁盘写入代码。

实机测试请记录：Ventoy 版本和模式、是否进入内核、显示分辨率、屏幕照片。到达引导菜单本身不算内核启动成功。

在 QEMU 中复现 Ventoy 测试（提前解压官方 Ventoy 1.1.16 Linux 发行包，并安装 `dosfstools`）：

```sh
python3 tools/make_ventoy_fixture.py --ventoy-dir /path/to/ventoy-1.1.16 --memdisk
python3 tools/test_boot.py --usb --image build/ventoy-fixture/ventoy-test.img --output build/test-ventoy
```

该工具只创建普通测试文件，不向真实 U 盘安装 Ventoy。

## 小体积原则

当前发布内核 **17,320 字节（约 16.91 KiB）**；含引导器的 ISO **4,331,520 字节（约 4.13 MiB）**。这两种大小分别统计，不把引导依赖藏在指标之外。详见 [验证记录](docs/VALIDATION.md)。

- 没有第三方 Rust crate、堆分配、完整字库、图片、窗口系统或占位模块。
- 请求与数据共享 RW 段，避免重复页面；不同权限的段仍保持隔离。
- 发布版启用大小优化、LTO、符号剥离；注释不会增加发布内核体积。
- `make test` 对内核执行 32 KiB 预算检查。预算是当前阶段的增长警戒线，不是世界纪录。
- 保留有效诊断和边界检查；每加入一个功能都说明收益和大小变化。
- ISO 中存在为链式启动兼容而保留的内核副本和固件镜像空间，现阶段不盲删。

## 从哪里读代码

依次阅读 `src/main.rs` → `linker.ld` → `src/boot.rs` → `src/serial.rs` → `src/screen.rs`。中文注释解释行为和安全前提。详见 [学习导读](docs/LEARNING.md)、[路线图](docs/ROADMAP.md)、[来源与合并说明](docs/PROVENANCE.md)。

源码和文档进入 Git；`build/`、`target/`、外部引导器缓存及生成 ISO 不进入 Git。原始 C 压缩包与用户原始 Rust 压缩包保留在原位置，不覆盖。
