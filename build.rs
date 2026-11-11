use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=assets/windows.rc");
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-env-changed=RC");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let compiler =
        env::var("RC").unwrap_or_else(|_| if cfg!(windows) { "rc" } else { "llvm-rc" }.into());
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("app.res");
    let status = Command::new(&compiler)
        .current_dir(PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("assets"))
        .arg("/fo")
        .arg(&output)
        .arg("windows.rc")
        .status()
        .unwrap_or_else(|error| panic!("Cannot run Windows resource compiler {compiler}: {error}"));
    assert!(status.success(), "Windows icon resource compilation failed");
    println!("cargo:rustc-link-arg-bin=simdisplay={}", output.display());
}
