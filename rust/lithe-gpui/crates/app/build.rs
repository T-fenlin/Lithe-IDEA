#[cfg(windows)]
fn main() {
    println!("cargo:rerun-if-changed=lithe.rc");
    println!("cargo:rerun-if-changed=../../assets/app-icon/icon.ico");

    embed_resource::compile("lithe.rc", embed_resource::NONE)
        .manifest_optional()
        .map_err(|reason| format!("{reason:?}"))
        .expect("嵌入 Windows 图标资源失败：找不到可用的 rc.exe，或 lithe.rc 编译不过");
}

#[cfg(not(windows))]
fn main() {}
