//! Compiles the C++ host for the system Widevine CDM (see `shim/widevine_shim.cc`).

fn main() {
    // Android has no browser CDM: the system's MediaDrm decrypts there.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        return;
    }
    println!("cargo:rerun-if-changed=shim/widevine_shim.cc");
    println!("cargo:rerun-if-changed=vendor/content_decryption_module.h");

    cc::Build::new()
        .cpp(true)
        // The shim is tiny; always optimising keeps glibc's _FORTIFY_SOURCE quiet.
        .opt_level(2)
        .file("shim/widevine_shim.cc")
        .include("vendor")
        .flag_if_supported("-std=c++14")
        .flag_if_supported("-Wno-unused-parameter")
        .compile("widevine_shim");

    println!("cargo:rustc-link-lib=dylib=stdc++");
    println!("cargo:rustc-link-lib=dylib=dl");
}
