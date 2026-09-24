// ZipMount patch: bindgen is replaced with the ready bindings from
// src/bindings.rs, so part of this build script's original code went unused.
#![allow(dead_code, unused_imports)]

use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
#[cfg(feature = "system")]
use windows_registry::LOCAL_MACHINE;

static HEADER: &str = r#"
#include <winfsp/winfsp.h>
#include <winfsp/fsctl.h>
#include <winfsp/launch.h>
"#;

#[cfg(not(feature = "system"))]
fn local() -> String {
    let project_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    println!(
        "cargo:rustc-link-search={}",
        project_dir.join("winfsp/lib").to_string_lossy()
    );

    "--include-directory=winfsp/inc".into()
}

#[cfg(feature = "system")]
fn system() -> String {
    if !cfg!(windows) {
        panic!("'system' feature not supported for cross-platform compilation.");
    }

    let directory = LOCAL_MACHINE
        .open("SOFTWARE\\WOW6432Node\\WinFsp")
        .ok()
        .and_then(|u| u.get_string("InstallDir").ok())
        .expect("WinFsp installation directory not found.");

    println!("cargo:rustc-link-search={}/lib", directory);

    // ZipMount: a plain WinFsp install through winget ships only bin — there
    // are no lib and inc directories (the development SDK). Add the import
    // library bundled with the crate as a fallback search path: it is only
    // needed to generate the delay-load thunks, and the DLL itself is still
    // loaded from the system installation at run time.
    let project_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    println!(
        "cargo:rustc-link-search={}",
        project_dir.join("winfsp/lib").to_string_lossy()
    );

    format!("--include-directory={}/inc", directory)
}

fn copy_winfsp_dll(winfsp_lib: &str) {
    println!("cargo:rerun-if-env-changed=WINFSP_DLL_OUTPUT_PATH");

    // Get the output path from environment variable
    let dll_out_path = match env::var("WINFSP_DLL_OUTPUT_PATH") {
        Ok(path) => PathBuf::from(path),
        Err(_) => {
            return;
        }
    };

    if let Err(e) = fs::create_dir_all(&dll_out_path) {
        panic!(
            "Failed to create WinFSP DLL output directory {}: {}",
            dll_out_path.display(),
            e
        );
    }

    let project_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let dll_path = project_dir
        .join("winfsp/bin")
        .join(format!("{}.dll", winfsp_lib));
    if !dll_path.exists() {
        panic!(
            "WinFSP DLL source file does not exist: {}",
            dll_path.display()
        );
    }

    let dll_dest = dll_out_path.join(format!("{}.dll", winfsp_lib));
    if let Err(e) = fs::copy(&dll_path, &dll_dest) {
        panic!(
            "Failed to copy {} to {}: {}",
            dll_path.display(),
            dll_dest.display(),
            e
        );
    }
}

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // host needs to be windows
    if cfg!(feature = "docsrs") {
        println!("cargo:warning=WinFSP does not build on any operating system but Windows. This feature is meant for docs.rs only. It will not link when compiled into a binary.");
        File::create(out_dir.join("bindings.rs")).unwrap();
        return;
    }

    // Use the target OS configuration instead of the host OS configuration to enable cross-platform compilation
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_else(|_| "unknown".to_string());
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "unknown".to_string());
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_else(|_| "unknown".to_string());

    if target_os != "windows" {
        panic!("WinFSP is only supported on Windows.");
    }

    // The return value (include arguments for clang) is no longer needed,
    // but the call itself is: it prints cargo:rustc-link-search, without
    // which the linker cannot find winfsp-x64.lib.
    #[cfg(feature = "system")]
    let _link_include = system();
    #[cfg(not(feature = "system"))]
    let _link_include = local();

    println!("cargo:rustc-link-lib=dylib=delayimp");

    // Architecture-specific configuration
    let (winfsp_lib, clang_target) = match (target_arch.as_str(), target_env.as_str()) {
        ("x86_64", "msvc") => ("winfsp-x64", "x86_64-pc-windows-msvc"),
        ("x86", "msvc") => ("winfsp-x86", "x86-pc-windows-msvc"),
        ("aarch64", "msvc") => ("winfsp-a64", "aarch64-pc-windows-msvc"),
        _ => panic!("unsupported triple {}", env::var("TARGET").unwrap()),
    };

    println!("cargo:rustc-link-lib=dylib={}", winfsp_lib);
    println!("cargo:rustc-link-arg=/DELAYLOAD:{}.dll", winfsp_lib);

    let bindings_path_str = out_dir.join("bindings.rs");

    // ZipMount: instead of running bindgen, take the bindings the crate
    // already carries in src/bindings.rs — generated for this very WinFsp 2.1
    // and for x86_64-pc-windows-msvc. So the build needs no LLVM/libclang.
    let _ = clang_target;
    if !Path::new(&bindings_path_str).exists() {
        let prebuilt = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("src")
            .join("bindings.rs");
        fs::copy(&prebuilt, &bindings_path_str)
            .expect("cannot copy the ready winfsp-sys bindings");
        println!("cargo:rerun-if-changed={}", prebuilt.display());
    }

    #[cfg(not(feature = "system"))]
    copy_winfsp_dll(winfsp_lib);
}
