#!/usr/bin/env python3
"""检查真实 ELF 装载布局，报告文件大小与映射下界；无需第三方 Python 包。"""
import argparse
import json
from pathlib import Path
import struct

ROOT = Path(__file__).resolve().parents[1]


def inspect(path, budget):
    data = path.read_bytes()
    if data[:7] != b"\x7fELF\x02\x01\x01":
        raise SystemExit("需要小端 ELF64 文件")
    kind, machine = struct.unpack_from("<HH", data, 16)
    if (kind, machine) != (2, 62):
        raise SystemExit("需要 x86_64 静态可执行文件")
    entry, offset = struct.unpack_from("<QQ", data, 24)
    size, count = struct.unpack_from("<HH", data, 54)
    segments = []
    if size != 56:
        raise SystemExit("ELF 程序头大小不符")
    for i in range(count):
        t, flags, off, address, _, filesz, memsz, align = struct.unpack_from("<IIQQQQQQ", data, offset + i * size)
        if t in (2, 3):
            raise SystemExit("裸机内核不能依赖动态链接器")
        if t != 1 or memsz == 0:
            continue
        if filesz > memsz or off + filesz > len(data) or flags & 3 == 3:
            raise SystemExit("无效装载范围或可写且可执行的段")
        if align != 4096 or (off - address) % align:
            raise SystemExit("段未按 4 KiB 对齐")
        segments.append((address, address + memsz, flags, filesz))
    if not any(a <= entry < b and f & 1 for a, b, f, _ in segments):
        raise SystemExit("入口不在可执行段中")
    for a, b, flags, _ in segments:
        for c, d, other, _ in segments:
            if flags != other and a // 4096 < (d + 4095) // 4096 and c // 4096 < (b + 4095) // 4096:
                raise SystemExit("不同权限的段共用页面，Limine 会拒绝加载")
    if len(data) > budget:
        raise SystemExit(f"内核 {len(data)} 字节超出 {budget} 字节预算，需要解释增长原因")
    pages = set()
    for a, b, _, _ in segments:
        pages.update(range(a // 4096, (b + 4095) // 4096))
    # 这里只计 ELF 的装载页，不包括引导器、栈、页表、显存，不能冒充总内存占用。
    return {"kernel_file_bytes": len(data), "load_file_bytes": sum(s[3] for s in segments),
            "load_memory_bytes": sum(b-a for a,b,_,_ in segments),
            "load_page_bytes": len(pages) * 4096, "kernel_budget_bytes": budget}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--kernel", type=Path, default=ROOT / "target/x86_64-unknown-none/release/kernel")
    parser.add_argument("--budget", type=int, default=32768)
    args = parser.parse_args()
    result = inspect(args.kernel, args.budget)
    iso = ROOT / "build/infinitesimalOS.iso"
    if iso.exists():
        result["iso_bytes"] = iso.stat().st_size
    print(json.dumps(result, indent=2))
