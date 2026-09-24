use super::*;
use std::path::PathBuf;

use zipfs_core::NameEncoding;

/// nfsstat3 has no PartialEq: compare by pattern.
macro_rules! assert_err {
    ($result:expr, $status:ident) => {
        let result = $result;
        assert!(
            matches!(result, Err(nfsstat3::$status)),
            "expected {}, got {:?}",
            stringify!($status),
            result.err()
        )
    };
}

const BLOCK: usize = 512;

/// A ustar header with a correct checksum.
fn header(name: &str, size: usize, typeflag: u8) -> Vec<u8> {
    let mut h = vec![0u8; BLOCK];
    h[..name.len()].copy_from_slice(name.as_bytes());
    let size_field = format!("{size:011o}\0");
    h[124..124 + size_field.len()].copy_from_slice(size_field.as_bytes());
    let mtime_field = format!("{:011o}\0", 1_700_000_000u64);
    h[136..136 + mtime_field.len()].copy_from_slice(mtime_field.as_bytes());
    h[156] = typeflag;
    h[257..263].copy_from_slice(b"ustar\0");
    h[148..156].fill(b' ');
    let sum: u64 = h.iter().map(|&b| u64::from(b)).sum();
    let sum_field = format!("{sum:06o}\0 ");
    h[148..148 + sum_field.len()].copy_from_slice(sum_field.as_bytes());
    h
}

fn big_body() -> Vec<u8> {
    (0..300_000u32).map(|i| (i % 251) as u8).collect()
}

/// docs/readme.txt, docs/a..e.txt and big.bin, in a tar in a temp directory.
fn archive() -> (ZipNfs, PathBuf) {
    let mut tar = Vec::new();
    let mut add = |name: &str, body: &[u8]| {
        tar.extend(header(name, body.len(), b'0'));
        tar.extend_from_slice(body);
        tar.extend(std::iter::repeat_n(
            0u8,
            (BLOCK - body.len() % BLOCK) % BLOCK,
        ));
    };
    add("docs/readme.txt", b"hello from the archive");
    for c in ['a', 'b', 'c', 'd', 'e'] {
        add(&format!("docs/{c}.txt"), c.to_string().as_bytes());
    }
    add("big.bin", &big_body());
    tar.extend(std::iter::repeat_n(0u8, BLOCK * 2));

    let path = std::env::temp_dir().join(format!(
        "zipfs-nfs-test-{}-{:?}.tar",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::write(&path, tar).expect("write the test archive");
    let archive = Archive::open(&path, NameEncoding::Auto).expect("open the test archive");
    (ZipNfs::new(archive, 16 << 20), path)
}

fn run<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .build()
        .expect("a runtime")
        .block_on(future)
}

async fn id_of_path(fs: &ZipNfs, path: &str) -> Result<fileid3, nfsstat3> {
    let mut id = fs.root_dir();
    for part in path.split('/') {
        id = fs.lookup(id, &part.as_bytes().into()).await?;
    }
    Ok(id)
}

#[test]
fn looks_up_paths_dots_included() {
    let (fs, path) = archive();
    run(async {
        let docs = id_of_path(&fs, "docs").await.unwrap();
        let readme = id_of_path(&fs, "docs/readme.txt").await.unwrap();
        let dot = fs.lookup(docs, &b".".as_slice().into()).await.unwrap();
        assert_eq!(dot, docs);
        let up = fs.lookup(docs, &b"..".as_slice().into()).await.unwrap();
        assert_eq!(up, fs.root_dir());
        assert_err!(
            fs.lookup(docs, &b"missing".as_slice().into()).await,
            NFS3ERR_NOENT
        );
        assert_err!(
            fs.lookup(readme, &b"x".as_slice().into()).await,
            NFS3ERR_NOTDIR
        );
    });
    let _ = std::fs::remove_file(path);
}

#[test]
fn attributes_are_read_only() {
    let (fs, path) = archive();
    run(async {
        let docs = fs
            .getattr(id_of_path(&fs, "docs").await.unwrap())
            .await
            .unwrap();
        assert!(matches!(docs.ftype, ftype3::NF3DIR));
        assert_eq!(docs.mode, 0o555);
        let big = fs
            .getattr(id_of_path(&fs, "big.bin").await.unwrap())
            .await
            .unwrap();
        assert!(matches!(big.ftype, ftype3::NF3REG));
        assert_eq!(big.mode, 0o444);
        assert_eq!(big.size, 300_000);
        assert_eq!(big.mtime.seconds, 1_700_000_000);
        assert_err!(fs.getattr(999_999).await, NFS3ERR_STALE);
        assert_err!(fs.getattr(0).await, NFS3ERR_STALE);
    });
    let _ = std::fs::remove_file(path);
}

#[test]
fn reads_in_pieces_and_flags_the_end() {
    let (fs, path) = archive();
    let expected = big_body();
    run(async {
        let id = id_of_path(&fs, "big.bin").await.unwrap();
        let mut got = Vec::new();
        loop {
            let (data, eof) = fs.read(id, got.len() as u64, 65_536).await.unwrap();
            got.extend_from_slice(&data);
            if eof {
                break;
            }
            assert_eq!(data.len(), 65_536, "a short read before the end");
        }
        assert_eq!(got, expected);
        // Past the end: nothing, and the end.
        assert_eq!(fs.read(id, 400_000, 10).await.unwrap(), (Vec::new(), true));
        let dir = id_of_path(&fs, "docs").await.unwrap();
        assert_err!(fs.read(dir, 0, 10).await, NFS3ERR_ISDIR);
    });
    let _ = std::fs::remove_file(path);
}

#[test]
fn lists_directories_page_by_page() {
    let (fs, path) = archive();
    run(async {
        let docs = id_of_path(&fs, "docs").await.unwrap();
        let mut names = Vec::new();
        let mut after = 0;
        loop {
            let page = fs.readdir(docs, after, 2).await.unwrap();
            assert!(page.entries.len() <= 2);
            for e in &page.entries {
                names.push(String::from_utf8(e.name.0.clone()).unwrap());
                after = e.fileid;
            }
            if page.end {
                break;
            }
        }
        assert_eq!(
            names,
            ["a.txt", "b.txt", "c.txt", "d.txt", "e.txt", "readme.txt"]
        );
        assert_err!(fs.readdir(docs, 123_456, 2).await, NFS3ERR_BAD_COOKIE);
    });
    let _ = std::fs::remove_file(path);
}

#[test]
fn refuses_every_change() {
    let (fs, path) = archive();
    run(async {
        let root = fs.root_dir();
        let name: filename3 = b"new".as_slice().into();
        assert_err!(fs.write(root, 0, b"x").await, NFS3ERR_ROFS);
        assert_err!(fs.mkdir(root, &name).await, NFS3ERR_ROFS);
        assert_err!(fs.remove(root, &name).await, NFS3ERR_ROFS);
        assert!(matches!(fs.capabilities(), VFSCapabilities::ReadOnly));
    });
    let _ = std::fs::remove_file(path);
}

#[test]
fn times_convert_from_filetime() {
    let t = nfs_time((1_704_067_200 + FILETIME_TO_UNIX_SECS) * 10_000_000 + 7);
    assert_eq!((t.seconds, t.nseconds), (1_704_067_200, 700));
    assert_eq!(nfs_time(0).seconds, FALLBACK_SECS);
}
