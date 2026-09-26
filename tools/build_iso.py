#!/usr/bin/env python3
"""制作可用于 QEMU / Ventoy 的混合启动 ISO；所有输出均位于 build/。"""
import argparse
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
from fetch_limine import prepare

ROOT = Path(__file__).resolve().parents[1]


def run(*args):
    subprocess.run([str(a) for a in args], check=True, cwd=ROOT)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--kernel", type=Path, default=Path("target/x86_64-unknown-none/release/kernel"))
    args = parser.parse_args()
    kernel = (ROOT / args.kernel).resolve()
    if not kernel.is_file():
        parser.error(f"Kernel not found: {kernel}; run make build first")
    vendor = prepare()
    build = ROOT / "build"
    stage = build / "iso-root"
    # 每次从空目录打包，防止已经删除的旧文件混入 ISO。
    if stage.exists():
        shutil.rmtree(stage)
    (stage / "boot/limine").mkdir(parents=True, exist_ok=True)
    (stage / "EFI/BOOT").mkdir(parents=True, exist_ok=True)
    shutil.copy2(kernel, stage / "boot/kernel.elf")
    shutil.copy2(ROOT / "limine.conf", stage / "boot/limine/limine.conf")
    for name in ["limine-bios.sys", "limine-bios-cd.bin", "limine-uefi-cd.bin"]:
        shutil.copy2(vendor / name, stage / "boot/limine" / name)
    shutil.copy2(vendor / "BOOTX64.EFI", stage / "EFI/BOOT/BOOTX64.EFI")
    shutil.copy2(ROOT / "limine.conf", stage / "EFI/BOOT/limine.conf")
    # EFI 启动映像是 ISO 内部的 FAT 卷。链式启动时固件可能只暴露此卷。
    # 因此在卷内也放内核和配置：这里的少量重复用于保留 Ventoy 启动兼容性。
    efi_image = stage / "boot/limine/limine-uefi-cd.bin"
    run(os.environ.get("MMD", "mmd"), "-i", efi_image, "::/boot")
    run(os.environ.get("MCOPY", "mcopy"), "-i", efi_image, kernel, "::/boot/kernel.elf")
    run(os.environ.get("MCOPY", "mcopy"), "-i", efi_image, ROOT / "limine.conf", "::/EFI/BOOT/limine.conf")
    # 告知 Ventoy 这是兼容启动镜像；内核进入后不依赖虚拟磁盘。
    (stage / "ventoy.dat").write_text("infinitesimalOS runs entirely in RAM.\n")
    for name in ["LICENSE"]:
        shutil.copy2(vendor / name, stage / "boot/limine" / name)
    iso = build / "infinitesimalOS.iso"
    run(os.environ.get("XORRISO", "xorriso"), "-as", "mkisofs", "-R", "-r", "-J",
        "-V", "INFINITESIMALOS", "-b", "boot/limine/limine-bios-cd.bin",
        "-no-emul-boot", "-boot-load-size", "4", "-boot-info-table", "-hfsplus",
        "-apm-block-size", "2048", "--efi-boot", "boot/limine/limine-uefi-cd.bin",
        "-efi-boot-part", "--efi-boot-image", "--protective-msdos-label",
        stage, "-o", iso)
    installer = build / "limine-host"
    run(os.environ.get("CC", "cc"), "-O2", "-std=c99", "-D_FILE_OFFSET_BITS=64",
        vendor / "limine.c", "-o", installer)
    # 保留原有 BIOS 启动支持。安装器仅操作刚生成的普通 ISO 文件，不访问物理磁盘。
    if not iso.is_file() or iso.is_symlink():
        raise SystemExit("Refusing non-regular output image")
    run(installer, "bios-install", iso)
    digest = hashlib.sha256(iso.read_bytes()).hexdigest()
    (build / "SHA256SUMS").write_text(f"{digest}  {iso.name}\n")
    print(f"ISO: {iso}\nSHA256: {digest}")


if __name__ == "__main__":
    main()
