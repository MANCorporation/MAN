//! On Windows targets, link the icon, version info and the manifest that
//! asks for administrator rights (DUHACH! changes partitions).
fn main() {
    println!("cargo:rerun-if-changed=data/duhach.rc");
    println!("cargo:rerun-if-changed=data/duhach.manifest");
    println!("cargo:rerun-if-changed=data/duhach.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("duhach-res.o");
    let windres = std::env::var("WINDRES").unwrap_or_else(|_| "x86_64-w64-mingw32-windres".into());
    let st = std::process::Command::new(&windres)
        .current_dir("data")
        .args(["duhach.rc", "-O", "coff", "-o"])
        .arg(&out)
        .status()
        .expect("windres not found (set WINDRES)");
    assert!(st.success(), "windres failed");
    println!("cargo:rustc-link-arg-bins={}", out.display());
}
