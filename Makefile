# 这些命令运行于开发机，不属于内核；make run 打开窗口，make test 无界面验证。
PYTHON ?= python3
CARGO ?= cargo

.PHONY: all build iso run test test-bios inspect fmt clean
all: iso
build:
	$(CARGO) build --release --locked --offline
iso: build
	$(PYTHON) tools/build_iso.py
run: iso
	$(PYTHON) tools/test_boot.py --interactive
test: iso
	$(PYTHON) tools/check_elf.py
	$(PYTHON) tools/test_boot.py
test-bios: iso
	$(PYTHON) tools/test_boot.py --bios --output build/test-bios
inspect: build
	$(PYTHON) tools/check_elf.py
fmt:
	$(CARGO) fmt --check
clean:
	$(CARGO) clean
	$(PYTHON) -c 'import shutil; shutil.rmtree("build", ignore_errors=True)'
