# Third-party notices

ZipMount is licensed under the GNU General Public License, version 3 or (at
your option) any later version — see [LICENSE](LICENSE). It builds on the
following work.

## WinFsp

> WinFsp - Windows File System Proxy, Copyright (C) Bill Zissimopoulos
> <https://github.com/winfsp/winfsp>

Licensed under GPLv3 with a FLOSS exception. The installer bundle carries the
official, unmodified WinFsp installer with its author's signature intact, and
installs it only when it is missing.

`vendor/winfsp-sys/` is a copy of the `winfsp-sys` 0.12.1 crate (GPL-3.0),
including the WinFsp headers, import libraries and DLLs it ships. It is
modified in one respect: `build.rs` uses pre-generated bindings instead of
running bindgen, so the build does not need LLVM. The comments in that
`build.rs` describe the change.

## winfsp-rs

The `winfsp` and `winfsp-sys` crates by SnowflakePowered,
<https://github.com/SnowflakePowered/winfsp-rs>, licensed under GPL-3.0. This
is what makes a built `zipmount.exe` a GPLv3 work.

## Archive formats

| Component | Used for | License |
|---|---|---|
| [sevenz-rust2](https://crates.io/crates/sevenz-rust2) | 7z | Apache-2.0 |
| [libdeflater](https://crates.io/crates/libdeflater) / [libdeflate](https://github.com/ebiggers/libdeflate) | deflate in zip | Apache-2.0 / MIT |
| [zlib-rs](https://github.com/trifectatechfoundation/zlib-rs) (`libz-rs-sys`) | checkpoint index for deflate | Zlib |
| [flate2](https://crates.io/crates/flate2) | tar.gz | MIT OR Apache-2.0 |
| [grep-searcher, grep-regex](https://github.com/BurntSushi/ripgrep) | content search | Unlicense OR MIT |
| [fluent-bundle, unic-langid](https://github.com/projectfluent/fluent-rs) | translations | Apache-2.0 OR MIT |

## Linux

| Component | Used for | License |
|---|---|---|
| [fuser](https://github.com/cberner/fuser) | mounting through FUSE | MIT |
| [libc](https://github.com/rust-lang/libc) | user and group ids, signals | MIT OR Apache-2.0 |

FUSE itself is part of the Linux kernel; mounting uses the system's
`fusermount3` helper, which is not bundled.

## macOS

| Component | Used for | License |
|---|---|---|
| [nfsserve](https://github.com/huggingface/nfsserve) | the local NFS server | BSD-3-Clause |
| [tokio](https://tokio.rs) | its runtime | MIT |
| [async-trait](https://github.com/dtolnay/async-trait) | its filesystem interface | MIT OR Apache-2.0 |

The NFS client and `mount_nfs` are part of macOS.

## RAR (not in official builds)

RAR support uses RARLAB's UnRAR source code through the
[unrar](https://crates.io/crates/unrar) crate. The UnRAR license forbids using
that code to re-create the RAR compression algorithm — a restriction that is
incompatible with the GPL and with WinFsp's FLOSS exception. For that reason
RAR support is a cargo feature, off by default, and **the builds published in
this repository's Releases do not contain UnRAR.** Building with
`--features rar` for your own use is fine; distributing such a build is not.

## Everything else

The remaining Rust dependencies are under permissive licenses (MIT,
Apache-2.0, BSD, Zlib, Unlicense and similar). The full list with versions is
in `Cargo.lock`; `cargo metadata` prints each crate's license.
