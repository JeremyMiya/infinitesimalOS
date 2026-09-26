# 来源、固定版本与合并决策

## 用户代码

输入：用户上传的 `infinitesimalOS.zip`。
SHA256：`1ae91cc9a3c2928ca0c347dfbc4e974c731d20c906256f692c370160eb723f55`。

保留其 Rust 裸机入口设计、`.data` / `.bss` 自检目的、COM1 日志及 Make 构建入口。删除普通 Rust 函数内部改写 rsp、未被可靠使用的 16 KiB 静态栈、假定正在 Mac 实机运行的固定日志。修复不同权限段共享页面及基础协议版本不匹配。

## 之前的 Live 版本

合入协议响应读取、带超时的串口、帧缓冲与微型字库、ISO 打包和 QEMU / Ventoy 测试工具。去掉装饰色块、重复栈请求、预编译内核入口和工程名称混用。保留已有 BIOS 支持，不另引入新功能。

两个 UART 实现合为一个；`x86_64` crate 在用户版本只用于端口 I/O，改为少量明确的汇编封装。完整 `limine` crate 改为本阶段所需的协议结构和编译期布局断言。这减少依赖并允许使用固定稳定版 Rust；不能仅凭删除 crate 就断言节省了多少机器码，实际尺寸见验证记录。

## 上游

- Rust 1.90.0，目标 `x86_64-unknown-none`；工具链由 rustup 管理。
- Limine 12.9.0 官方二进制发行包：<https://github.com/limine-bootloader/limine/releases/tag/v12.9.0>。
- 归档 SHA256：`9a738586bff5790bd8bfef4a4868a2939cba3f81f22f121306d668c97f1c85d8`。
- 逐文件 SHA256 在 `tools/limine.sha256`；自动下载的上游 LICENSE 保留在缓存和 ISO 中。
- 协议定义参考 limine-protocol 提交 `da65184e91f80fcb397270121b1e2515a11e01ee`：<https://github.com/limine-bootloader/limine-protocol/tree/da65184e91f80fcb397270121b1e2515a11e01ee>。保留其 0BSD 许可证于 `docs/limine-protocol-LICENSE`。
- 虚拟机 Ventoy 验证使用官方 1.1.16 Linux 包，仓库不分发该包。

原创代码的最终许可证由项目所有者决定；本次不擅自新增授权条款。

## 原始归档

原始 C `minikernel-step1.zip` 和用户 Rust ZIP 均未被覆盖。本次不将历史压缩包或旧 RISC-V 验证日志混入新项目源码。

C 归档 SHA256：`0b69fd9497d096b7aaa6c6237e31b81130777c489075019e0bbfbb9ce8e5eddd`。
