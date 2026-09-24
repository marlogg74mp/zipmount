fn main() {
    // unrar_sys calls registry and access-token functions (RegOpenKeyExW,
    // OpenProcessToken and others) but does not declare its dependency on
    // advapi32. The crate's test binary then fails to link: the main binary
    // only linked because the WinFsp layer pulled advapi32 in via windows-sys.
    if std::env::var_os("CARGO_FEATURE_RAR").is_some() {
        println!("cargo:rustc-link-lib=dylib=advapi32");
    }
}
