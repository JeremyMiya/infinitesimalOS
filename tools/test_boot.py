#!/usr/bin/env python3
"""在真实 QEMU 进程中启动内核；校验串口并保存屏幕、CPU 寄存器。
通过 QMP 管理虚拟机，所有磁盘都是普通文件，不需要 root。
可用 QEMU / QEMU_DATA_DIR / OVMF_CODE / OVMF_VARS 环境变量覆盖依赖路径。
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import select
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]


def firmware(name, candidates):
    if os.environ.get(name):
        path = Path(os.environ[name]).resolve()
        if path.is_file():
            return path
    for candidate in candidates:
        path = Path(candidate)
        if path.is_file():
            return path
    raise SystemExit(f"Set {name} to an installed OVMF firmware file (see README).")


class QMP:
    def __init__(self, reader, writer):
        self.reader = reader
        self.writer = writer
        self.buffer = b""
        greeting = self.read()
        assert "QMP" in greeting, greeting
        self.call("qmp_capabilities")

    def read(self):
        while b"\n" not in self.buffer:
            if not select.select([self.reader], [], [], 5)[0]:
                raise RuntimeError("QMP response timed out")
            data = os.read(self.reader.fileno(), 65536)
            if not data:
                raise RuntimeError("QMP disconnected")
            self.buffer += data
        line, self.buffer = self.buffer.split(b"\n", 1)
        return json.loads(line)

    def call(self, command, **args):
        request = {"execute": command}
        if args:
            request["arguments"] = args
        self.writer.write((json.dumps(request) + "\n").encode())
        self.writer.flush()
        while True:
            response = self.read()
            if "error" in response:
                raise RuntimeError(response["error"])
            if "return" in response:
                return response["return"]

    def close(self):
        self.reader.close()
        self.writer.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--image", type=Path, default=ROOT / "build/infinitesimalOS.iso")
    parser.add_argument("--output", type=Path, default=ROOT / "build/test-uefi")
    parser.add_argument("--usb", action="store_true", help="Boot image as a USB mass-storage device")
    parser.add_argument("--bios", action="store_true", help="Use SeaBIOS instead of OVMF")
    parser.add_argument("--interactive", action="store_true", help="Open a QEMU display; exit with its window close control")
    parser.add_argument("--timeout", type=float, default=60)
    parser.add_argument("--capture-interval", type=float, default=0, help="Optional intermediate screen captures, seconds")
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    image = args.image.resolve()
    if not image.is_file():
        parser.error(f"Image missing: {image}")
    serial_path = output / "serial.log"
    serial_path.write_text("")
    # 禁用网络、使用单 CPU；当前内核未初始化其他 CPU，也没有网络驱动。
    command = [os.environ.get("QEMU", "qemu-system-x86_64"),
               "-machine", "q35,accel=tcg", "-cpu", "qemu64", "-m", "256M",
               "-smp", "1", "-nic", "none", "-vga", "std", "-no-reboot", "-no-shutdown",
               "-serial", f"file:{serial_path}"]
    if os.environ.get("QEMU_DATA_DIR"):
        command += ["-L", os.environ["QEMU_DATA_DIR"]]
    if not args.interactive:
        command += ["-display", "none", "-qmp", "stdio"]
    # 每次复制独立固件变量文件，避免修改系统 OVMF 模板或沿用旧启动状态。
    if not args.bios:
        code = firmware("OVMF_CODE", ["/usr/share/OVMF/OVMF_CODE_4M.fd", "/usr/share/OVMF/OVMF_CODE.fd"])
        template = firmware("OVMF_VARS", ["/usr/share/OVMF/OVMF_VARS_4M.fd", "/usr/share/OVMF/OVMF_VARS.fd"])
        variables = output / "OVMF_VARS.fd"
        shutil.copyfile(template, variables)
        command += ["-drive", f"if=pflash,format=raw,readonly=on,file={code}",
                    "-drive", f"if=pflash,format=raw,file={variables}"]
    if args.usb:
        command += ["-device", "qemu-xhci", "-drive", f"if=none,id=stick,format=raw,readonly=on,file={image}",
                    "-device", "usb-storage,drive=stick,bootindex=1"]
    else:
        command += ["-cdrom", str(image), "-boot", "d"]
    (output / "command.json").write_text(json.dumps(command, indent=2) + "\n")
    if args.interactive:
        subprocess.run(command, check=True)
        return
    qmp = None
    with (output / "qemu.log").open("w") as errors:
        process = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                   stderr=errors, bufsize=0)
        try:
            qmp = QMP(process.stdout, process.stdin)
            end = time.monotonic() + args.timeout
            next_capture = time.monotonic() + args.capture_interval
            capture = 0
            success = False
            # 成功必须来自本次内核输出；超时或 BOOT_FAIL 都不会算通过。
            while time.monotonic() < end and process.poll() is None:
                if args.capture_interval > 0 and time.monotonic() >= next_capture:
                    qmp.call("screendump", filename=str(output / f"stage-{capture:02}.ppm"))
                    capture += 1
                    next_capture += args.capture_interval
                log = serial_path.read_text(errors="replace")
                if "BOOT_FAIL" in log:
                    break
                if "BOOT_OK FRAMEBUFFER_RENDERED RAM_ONLY NO_DISK_DRIVER" in log:
                    success = True
                    break
                time.sleep(0.2)
            if qmp is not None:
                qmp.call("screendump", filename=str(output / "screen.ppm"))
                registers = qmp.call("human-monitor-command", **{"command-line": "info registers"})
                (output / "registers.txt").write_text(registers)
            log = serial_path.read_text(errors="replace")
            print(log)
            expected_firmware = "FIRMWARE=BIOS" if args.bios else "FIRMWARE=UEFI 64-BIT"
            required = [expected_firmware, "CPL=0", "RUNTIME_SELFTEST=PASS", "FRAMEBUFFER=", "BOOT_OK FRAMEBUFFER_RENDERED"]
            if not success or not all(marker in log for marker in required):
                raise SystemExit(f"Boot test FAILED. See {output} (including screen.ppm and qemu.log)")
            (output / "PASS.txt").write_text("PASS: actual QEMU boot, expected firmware, CPL0, runtime self-test, framebuffer render.\n")
            print(f"PASS: {output}")
        finally:
            if qmp is not None:
                try:
                    qmp.call("quit")
                except (OSError, RuntimeError):
                    pass
                qmp.close()
            if process.poll() is None:
                process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()


if __name__ == "__main__":
    main()
