#!/usr/bin/env python3
"""创建仅供 QEMU 使用的 256 MiB Ventoy 测试磁盘文件。
从已解压的官方 Ventoy 发行包读取引导内容，在普通文件内建立 MBR、FAT32、EFI 分区。
不运行 Ventoy2Disk、不访问真实 U 盘、不挂载文件系统。需要 mkfs.fat 与 mtools。
"""
import argparse
import json
import lzma
import os
from pathlib import Path
import shutil
import struct
import subprocess
import uuid

ROOT = Path(__file__).resolve().parents[1]


def run(*args):
    subprocess.run([str(arg) for arg in args], check=True)


def regular_output(path, size):
    if path.is_symlink() or (path.exists() and not path.is_file()):
        raise SystemExit(f"Refusing non-regular output: {path}")
    with path.open("wb") as stream:
        stream.truncate(size)


def chs(lba):
    cylinder, rem = divmod(lba, 255 * 63)
    head, sector = divmod(rem, 63)
    if cylinder > 1023:
        return bytes([254, 255, 255])
    return bytes([head, (sector + 1) | ((cylinder >> 2) & 0xC0), cylinder & 255])


def partition(start, size, kind, active=0):
    return bytes([active]) + chs(start) + bytes([kind]) + chs(start + size - 1) + struct.pack("<II", start, size)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ventoy-dir", type=Path, required=True)
    parser.add_argument("--image", type=Path, default=ROOT / "build/infinitesimalOS.iso")
    parser.add_argument("--memdisk", action="store_true")
    args = parser.parse_args()
    ventoy = args.ventoy_dir.resolve()
    iso = args.image.resolve()
    if not iso.is_file():
        parser.error(f"ISO missing: {iso}")
    boot = (ventoy / "boot/boot.img").read_bytes()
    core = lzma.decompress((ventoy / "boot/core.img.xz").read_bytes())
    efi = lzma.decompress((ventoy / "ventoy/ventoy.disk.img.xz").read_bytes())
    if len(efi) != 32 * 1024 * 1024 or len(boot) < 512 or len(core) < 2047 * 512:
        raise SystemExit("Unexpected Ventoy release image layout")
    output = ROOT / "build/ventoy-fixture"
    output.mkdir(parents=True, exist_ok=True)
    disk = output / "ventoy-test.img"
    data = output / "data.fat"
    total_sectors = 256 * 1024 * 1024 // 512
    p1_start, p2_size = 2048, 65536
    p2_start = total_sectors - p2_size
    p1_size = p2_start - p1_start
    regular_output(data, p1_size * 512)
    run(os.environ.get("MKFS_FAT", "mkfs.fat"), "-F", "32", "-n", "Ventoy", data)
    mcopy = os.environ.get("MCOPY", "mcopy")
    run(mcopy, "-i", data, iso, "::/infinitesimalOS.iso")
    run(os.environ.get("MMD", "mmd"), "-i", data, "::/ventoy")
    config = output / "ventoy.json"
    settings = {"control": [
        {"VTOY_MENU_TIMEOUT": "5"},
        {"VTOY_DEFAULT_IMAGE": "/infinitesimalOS.iso"},
        {"VTOY_SECONDARY_TIMEOUT": "2"},
        {"VTOY_MENU_LANGUAGE": "en_US"},
    ]}
    if args.memdisk:
        settings["auto_memdisk"] = ["/infinitesimalOS.iso"]
    config.write_text(json.dumps(settings, indent=2) + "\n")
    run(mcopy, "-i", data, config, "::/ventoy/ventoy.json")
    # 分区表仅写入此测试文件；EFI 分区内容保持官方发行包原样。
    mbr = bytearray(boot[:512])
    mbr[384:400] = uuid.uuid4().bytes
    mbr[440:444] = uuid.uuid4().bytes[:4]
    mbr[446:510] = bytes(64)
    mbr[446:462] = partition(p1_start, p1_size, 0x07, 0x80)
    mbr[462:478] = partition(p2_start, p2_size, 0xEF)
    mbr[510:512] = b"\x55\xaa"
    regular_output(disk, total_sectors * 512)
    with disk.open("r+b") as stream:
        stream.write(mbr)
        stream.seek(512)
        stream.write(core[:2047 * 512])
        stream.seek(p1_start * 512)
        with data.open("rb") as source:
            shutil.copyfileobj(source, stream)
        stream.seek(p2_start * 512)
        stream.write(efi)
    print(f"QEMU-only test image: {disk}")
    print("Boot with: python3 tools/test_boot.py --usb --image build/ventoy-fixture/ventoy-test.img --output build/test-ventoy --capture-interval 2")


if __name__ == "__main__":
    main()
