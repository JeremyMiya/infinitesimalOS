#!/usr/bin/env python3
"""获取固定版本的引导器，核对哈希后仅解包构建所需文件。工具运行于开发机。"""
import argparse
import hashlib
from pathlib import Path
import tarfile
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
VERSION = "12.9.0"
SHA256 = "9a738586bff5790bd8bfef4a4868a2939cba3f81f22f121306d668c97f1c85d8"
URL = f"https://github.com/limine-bootloader/limine/releases/download/v{VERSION}/limine-binary.tar.xz"


def prepare(archive=None):
    destination = ROOT / "build/limine"
    manifest = ROOT / "tools/limine.sha256"
    expected = [line.split() for line in manifest.read_text().splitlines()]
    # 已下载的文件逐一验证；缓存遭到修改时不会悄悄用于打包。
    if all((destination / name).is_file() and
           hashlib.sha256((destination / name).read_bytes()).hexdigest() == digest
           for digest, name in expected):
        return destination
    archive = Path(archive) if archive else ROOT / "build/limine-binary.tar.xz"
    archive.parent.mkdir(parents=True, exist_ok=True)
    if not archive.exists():
        print(f"下载 Limine {VERSION}: {URL}", flush=True)
        # 先写临时文件，下载失败不会留下看似完整的压缩包。
        temporary = archive.with_suffix(".part")
        with urllib.request.urlopen(URL, timeout=60) as response:
            temporary.write_bytes(response.read())
        temporary.replace(archive)
    if hashlib.sha256(archive.read_bytes()).hexdigest() != SHA256:
        raise SystemExit(f"引导器压缩包 SHA256 不匹配，请删除后重新下载：{archive}")
    destination.mkdir(parents=True, exist_ok=True)
    # 不调用 extractall：只取白名单文件的内容，忽略路径与链接，避免任意路径写入。
    with tarfile.open(archive) as source:
        for digest, name in expected:
            matches = [m for m in source.getmembers() if m.isfile() and Path(m.name).name == name]
            if len(matches) != 1:
                raise SystemExit(f"压缩包文件不唯一或不存在：{name}")
            data = source.extractfile(matches[0]).read()
            if hashlib.sha256(data).hexdigest() != digest:
                raise SystemExit(f"引导器文件校验失败：{name}")
            (destination / name).write_bytes(data)
    return destination


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, help="可选的离线官方压缩包")
    print(prepare(parser.parse_args().archive))
