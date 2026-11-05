//! Compiles the C++ layer that implements Chromium's CDM interface (shim/), except on Android,
//! where the system's MediaDrm decrypts and no C++ is built.

fn main() {
    println!("cargo:rerun-if-changed=shim");
    println!("cargo:rerun-if-changed=vendor");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        return;
    }
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        // Optimised in every profile: hardened toolchains (Nix's among them) define
        // _FORTIFY_SOURCE, which warns without optimisation, and the layer has nothing to debug.
        .opt_level(2)
        .include("vendor")
        .include("shim")
        .file("shim/cdm_11.cc")
        .warnings(true)
        .extra_warnings(true)
        .warnings_into_errors(true)
        .compile("widevine_shim");
}
