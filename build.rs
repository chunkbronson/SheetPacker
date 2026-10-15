fn main() {
    println!("cargo:rerun-if-changed=assets/SheetPacker.rc");
    println!("cargo:rerun-if-changed=assets/SheetPacker.ico");

    // Only Windows needs the icon resource compiled in.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    // Compile assets/SheetPacker.rc -> an object file and link it, so the .exe
    // shows SheetPacker.ico in Explorer / the taskbar.
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR");
    let obj = format!("{out_dir}/icon.o");
    let windres =
        std::env::var("WINDRES").unwrap_or_else(|_| "x86_64-w64-mingw32-windres".to_string());

    let status = std::process::Command::new(&windres)
        .args([
            "-I",
            "assets",
            "assets/SheetPacker.rc",
            "-O",
            "coff",
            "-o",
            &obj,
        ])
        .status()
        .unwrap_or_else(|e| {
            panic!("failed to run `{windres}` ({e}); is the LLVM-MinGW toolchain on PATH?")
        });

    if !status.success() {
        panic!("`{windres}` failed with {status}");
    }

    println!("cargo:rustc-link-arg={obj}");
}
