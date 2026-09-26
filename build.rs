// build.rs 在开发机上运行，不会进入内核，也不增加内核体积。
fn main() {
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    // 告诉链接器如何排列 ELF 的各段，并关闭不需要的构建标识。
    println!("cargo:rustc-link-arg=-T{root}/linker.ld");
    println!("cargo:rustc-link-arg=--build-id=none");
    // 只改链接脚本时也必须重新链接，否则会误测旧内核。
    println!("cargo:rerun-if-changed=linker.ld");
}
