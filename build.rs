fn main() {
    println!("cargo:rustc-link-search=all=./c");
    println!("cargo:rustc-link-search=all=/usr/local/vitasdk/arm-vita-eabi/lib");
    // vita2d
    println!("cargo:rustc-link-lib=static=vita2d");
    println!("cargo:rustc-link-lib=static=SceDisplay_stub");
    println!("cargo:rustc-link-lib=static=SceGxm_stub");
    println!("cargo:rustc-link-lib=static=SceSysmodule_stub");
    println!("cargo:rustc-link-lib=static=SceCtrl_stub");
    println!("cargo:rustc-link-lib=static=ScePgf_stub");
    println!("cargo:rustc-link-lib=static=SceCommonDialog_stub");
    println!("cargo:rustc-link-lib=static=freetype");
    println!("cargo:rustc-link-lib=static=png");
    println!("cargo:rustc-link-lib=static=jpeg");
    println!("cargo:rustc-link-lib=static=z");
    println!("cargo:rustc-link-lib=static=m");
    println!("cargo:rustc-link-lib=static=c");
    println!("cargo:rustc-link-lib=static=SceAppMgr_stub");
    // tai
    println!("cargo:rustc-link-lib=static=taihen_stub");
    println!("cargo:rustc-link-lib=static=SceVshBridge_stub");
    println!("cargo:rustc-link-lib=static=SceRegistryMgr_stub");
    println!("cargo:rustc-link-lib=static=SceAppUtil_stub");
    // sqlite
    println!("cargo:rustc-link-lib=static=sqlite");
    println!("cargo:rustc-link-lib=static=SceSqlite_stub");
    println!("cargo:rustc-link-lib=static=SceLibKernel_stub");
    println!("cargo:rustc-link-lib=static=VitaShellUser_stub_weak");

    cc::Build::new()
        .file("./c/tai.c")
        .static_flag(true)
        // .flag("-g")
        .compile("libtai.a");

    cc::Build::new()
        .file("./c/v2d.c")
        .static_flag(true)
        // .flag("-g")
        .compile("libv2d.a");

    cc::Build::new()
        .file("./c/ime.c")
        .static_flag(true)
        // .flag("-g")
        .compile("libime.a");
}
