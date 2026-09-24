fn main() {
    // The linker warns (LNK4104) that COM entry points must not go into the
    // import library: nobody should link against the DLL directly, COM loads
    // it itself. They are marked PRIVATE.
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    println!("cargo:rustc-link-arg-cdylib=/DEF:{manifest}/zipmount_shell.def");
    println!("cargo:rerun-if-changed=zipmount_shell.def");
}
