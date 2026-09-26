# 验证记录：2026-09-25

## 实际执行结果

环境：Linux x86_64 开发容器；Rust 1.90.0；x86_64-unknown-none；Limine 12.9.0；QEMU TCG、q35、单 CPU、256 MiB。

| 检查 | 结果 |
| --- | --- |
| 发布构建、dev 构建 | 通过；无第三方 crate |
| Rust 格式、Python 语法 | 通过 |
| ELF 权限、页面隔离、静态装载及 32 KiB 预算 | 通过 |
| QEMU + OVMF UEFI 光盘启动 | 通过 |
| QEMU + SeaBIOS 光盘启动 | 通过 |
| QEMU + Ventoy 1.1.16 Memdisk / UEFI USB 启动 | 通过 |
| 屏幕 | 1024×768、32 bpp，已检查实际截屏 |
| ShadowSecretary 实机 | 待验证 |
| Ventoy 普通模式 | 未列为通过 |

每条通过的启动路径都实际进入内核，输出 ring 0、装载自检 PASS、帧缓冲信息以及 BOOT_OK。没有把“编译通过”当作启动成功。

## 体积

| 指标 | 字节 |
| --- | ---: |
| 当前发布 ELF（剥离符号） | 17,320 |
| 装载段中的文件内容总量 | 8,748 |
| 装载段内存总量（含 BSS，不计页面取整） | 8,756 |
| ELF 装载页面总量 | 16,384 |
| 混合启动 ISO | 4,331,520 |
| 之前 Live 版本同工具链发布构建，再用 objcopy 去除全部符号 | 25,296 |

相对于最后一项，当前 ELF 缩小约 31.5%。比较对象是之前的 Live 代码，不是用户上传包中可能过时的预编译内核。ISO 仍包含独立 EFI FAT 映像、引导器、启动文件系统开销，以及为链式启动保留的内核副本。

ELF 装载页面不是系统总内存：没有计入默认至少 64 KiB 栈、引导器保留区、页表和显存。测试使用 256 MiB 虚拟机配置；未测量最低可启动内存，不宣称系统只需 16 KiB。

ISO 封装包含时间等元数据，重新构建不保证逐字节相同。下面的哈希标识本次测试产物。

- 内核 SHA256：`d1afbf7a4dc475ad1cfc923e3588df056315c178b36ae6323af419ea5e095f91`
- ISO SHA256：`5c1a83ab5b16f6a68a733c42bb73c2570b31995770468fdae89669e29c79eefc`

## uefi 内核日志

```text
infinitesimalOS ENTRY x86_64
FIRMWARE=UEFI 64-BIT CPL=0 BASE_REVISION=6
RUNTIME_SELFTEST=PASS USABLE_BYTES=220479488
FRAMEBUFFER=1024x768 BPP=32 PITCH=4096
BOOT_OK FRAMEBUFFER_RENDERED RAM_ONLY NO_DISK_DRIVER
```

## bios 内核日志

```text
infinitesimalOS ENTRY x86_64
FIRMWARE=BIOS CPL=0 BASE_REVISION=6
RUNTIME_SELFTEST=PASS USABLE_BYTES=266866688
FRAMEBUFFER=1024x768 BPP=32 PITCH=4096
BOOT_OK FRAMEBUFFER_RENDERED RAM_ONLY NO_DISK_DRIVER
```

## ventoy 内核日志

```text
infinitesimalOS ENTRY x86_64
FIRMWARE=UEFI 64-BIT CPL=0 BASE_REVISION=6
RUNTIME_SELFTEST=PASS USABLE_BYTES=147234816
FRAMEBUFFER=1024x768 BPP=32 PITCH=4096
BOOT_OK FRAMEBUFFER_RENDERED RAM_ONLY NO_DISK_DRIVER
```
