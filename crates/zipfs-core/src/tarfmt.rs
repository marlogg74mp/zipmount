//! Parsing the tar format.
//!
//! The format is as simple as it gets: a 512-byte header, then the data padded
//! to a 512-byte boundary, and so on. There is no table of contents at all —
//! learning what is inside means walking the whole chain of headers. On the
//! other hand each file's data is one contiguous piece, so random access comes
//! for free: it is just a slice.
//!
//! There are three historical extensions, and without them half of real-world
//! archives read wrong:
//!
//! * **ustar** — the `prefix` field, which allows long paths;
//! * **GNU longname** (type `L`) — the full name lies in a data block before
//!   the actual header;
//! * **PAX** (type `x`) — `key=value` records, of which `path` (the name in
//!   UTF-8) and `size` (for files over 8 GB) matter to us.

use anyhow::{bail, Result};
use zipmount_i18n::t;

use crate::zipfmt::unix_to_filetime;

pub const BLOCK: usize = 512;

/// Field offsets inside a header.
const OFF_NAME: usize = 0;
const OFF_SIZE: usize = 124;
const OFF_MTIME: usize = 136;
const OFF_CHKSUM: usize = 148;
const OFF_TYPEFLAG: usize = 156;
const OFF_MAGIC: usize = 257;
const OFF_PREFIX: usize = 345;

const LEN_NAME: usize = 100;
const LEN_SIZE: usize = 12;
const LEN_MTIME: usize = 12;
const LEN_CHKSUM: usize = 8;
const LEN_PREFIX: usize = 155;

#[derive(Debug, Clone)]
pub struct TarEntry {
    pub path: String,
    pub size: u64,
    /// Modification time as a Windows FILETIME.
    pub mtime: u64,
    /// Offset of the data in the stream (the decompressed one, if the archive
    /// was compressed).
    pub data_offset: u64,
    pub is_dir: bool,
}

/// Walks the whole stream and collects the list of entries.
pub fn parse_tar(data: &[u8]) -> Result<Vec<TarEntry>> {
    let mut entries = Vec::new();
    let mut pos = 0usize;

    // The name and size may come from a preceding auxiliary block.
    let mut pending_name: Option<String> = None;
    let mut pending_size: Option<u64> = None;

    let mut saw_any_header = false;

    while pos + BLOCK <= data.len() {
        let header = &data[pos..pos + BLOCK];

        // The end of the archive is two zero blocks. One is enough to stop:
        // by the specification only padding follows.
        if header.iter().all(|&b| b == 0) {
            break;
        }

        if !checksum_matches(header) {
            if saw_any_header {
                // Garbage after valid entries — stop quietly: that happens
                // with archives that had something foreign appended.
                break;
            }
            bail!(t!("core-tar-bad-header"));
        }
        saw_any_header = true;

        let typeflag = header[OFF_TYPEFLAG];
        let size = pending_size
            .take()
            .unwrap_or_else(|| numeric_field(&header[OFF_SIZE..OFF_SIZE + LEN_SIZE]).unwrap_or(0));
        let data_pos = pos + BLOCK;
        let padded = size.div_ceil(BLOCK as u64) * BLOCK as u64;
        let body = data
            .get(data_pos..data_pos + size.min(u32::MAX as u64) as usize)
            .unwrap_or(&[]);

        match typeflag {
            // GNU longname: the data holds the full name of the next entry.
            b'L' => {
                pending_name = Some(cstr(body).to_string());
            }
            // PAX: "key=value" records for the next entry.
            b'x' | b'X' => {
                let (path, sz) = parse_pax(body);
                if path.is_some() {
                    pending_name = path;
                }
                if sz.is_some() {
                    pending_size = sz;
                }
            }
            // Global PAX records are of no interest to us.
            b'g' => {}
            // A regular file (`0`, the historical `\0`, and contiguous `7`).
            b'0' | 0 | b'7' => {
                let path = pending_name.take().unwrap_or_else(|| header_name(header));
                if !path.is_empty() {
                    entries.push(TarEntry {
                        path,
                        size,
                        mtime: header_mtime(header),
                        data_offset: data_pos as u64,
                        is_dir: false,
                    });
                }
            }
            b'5' => {
                let path = pending_name.take().unwrap_or_else(|| header_name(header));
                if !path.is_empty() {
                    entries.push(TarEntry {
                        path,
                        size: 0,
                        mtime: header_mtime(header),
                        data_offset: data_pos as u64,
                        is_dir: true,
                    });
                }
            }
            // Links, devices, FIFOs: nothing in a read-only filesystem
            // corresponds to them, so they are skipped.
            _ => {
                pending_name = None;
            }
        }

        pos = data_pos + padded as usize;
    }

    Ok(entries)
}

/// Whether a buffer looks like the start of a tar archive.
///
/// The `ustar` magic first, then the header checksum: old v7 archives have no
/// magic at all, and the checksum is the only reliable sign left.
pub fn looks_like_tar(data: &[u8]) -> bool {
    if data.len() < BLOCK {
        return false;
    }
    let magic = &data[OFF_MAGIC..OFF_MAGIC + 5];
    if magic == b"ustar" {
        return true;
    }
    checksum_matches(&data[..BLOCK])
}

/// The header checksum: the sum of all its bytes, with the checksum field
/// itself counted as filled with spaces.
fn checksum_matches(header: &[u8]) -> bool {
    let Some(expected) = numeric_field(&header[OFF_CHKSUM..OFF_CHKSUM + LEN_CHKSUM]) else {
        return false;
    };

    let mut unsigned: u64 = 0;
    let mut signed: i64 = 0;
    for (i, &byte) in header.iter().enumerate() {
        let value = if (OFF_CHKSUM..OFF_CHKSUM + LEN_CHKSUM).contains(&i) {
            b' '
        } else {
            byte
        };
        unsigned += value as u64;
        signed += value as i8 as i64;
    }

    // Some ancient archivers summed signed bytes.
    unsigned == expected || signed == expected as i64
}

/// A tar numeric field: usually octal ASCII, but for large values GNU uses a
/// binary form flagged by the high bit of the first byte.
fn numeric_field(field: &[u8]) -> Option<u64> {
    if field.is_empty() {
        return None;
    }

    if field[0] & 0x80 != 0 {
        // base-256: the remaining bytes are a big-endian number.
        let mut value: u64 = (field[0] & 0x7f) as u64;
        for &byte in &field[1..] {
            value = value.checked_mul(256)?.checked_add(byte as u64)?;
        }
        return Some(value);
    }

    let text = field
        .iter()
        .copied()
        .take_while(|&b| b != 0)
        .filter(|&b| !b.is_ascii_whitespace())
        .collect::<Vec<u8>>();
    if text.is_empty() {
        return Some(0);
    }
    let text = std::str::from_utf8(&text).ok()?;
    u64::from_str_radix(text, 8).ok()
}

fn cstr(data: &[u8]) -> std::borrow::Cow<'_, str> {
    let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
    String::from_utf8_lossy(&data[..end])
}

/// The name from a header, with the ustar `prefix` field taken into account.
fn header_name(header: &[u8]) -> String {
    let name = cstr(&header[OFF_NAME..OFF_NAME + LEN_NAME]);
    let prefix = cstr(&header[OFF_PREFIX..OFF_PREFIX + LEN_PREFIX]);
    if prefix.is_empty() {
        name.into_owned()
    } else {
        format!("{prefix}/{name}")
    }
}

fn header_mtime(header: &[u8]) -> u64 {
    match numeric_field(&header[OFF_MTIME..OFF_MTIME + LEN_MTIME]) {
        Some(secs) => unix_to_filetime(secs as i64),
        None => 0,
    }
}

/// Parses PAX records of the form `<length> <key>=<value>\n`.
fn parse_pax(body: &[u8]) -> (Option<String>, Option<u64>) {
    let mut path = None;
    let mut size = None;

    let mut rest = body;
    while !rest.is_empty() {
        // The record length comes first, in decimal, and includes itself.
        let Some(space) = rest.iter().position(|&b| b == b' ') else {
            break;
        };
        let Ok(len_text) = std::str::from_utf8(&rest[..space]) else {
            break;
        };
        let Ok(len) = len_text.parse::<usize>() else {
            break;
        };
        if len == 0 || len > rest.len() {
            break;
        }

        let record = &rest[space + 1..len];
        let record = record.strip_suffix(b"\n").unwrap_or(record);
        if let Some(eq) = record.iter().position(|&b| b == b'=') {
            let key = &record[..eq];
            let value = &record[eq + 1..];
            match key {
                b"path" => path = Some(String::from_utf8_lossy(value).into_owned()),
                b"size" => {
                    size = std::str::from_utf8(value).ok().and_then(|s| s.parse().ok());
                }
                _ => {}
            }
        }

        rest = &rest[len..];
    }

    (path, size)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a valid tar header with a correct checksum.
    fn make_header(name: &str, size: u64, typeflag: u8) -> Vec<u8> {
        let mut h = vec![0u8; BLOCK];
        h[..name.len()].copy_from_slice(name.as_bytes());
        let size_field = format!("{size:011o}\0");
        h[OFF_SIZE..OFF_SIZE + size_field.len()].copy_from_slice(size_field.as_bytes());
        let mtime_field = format!("{:011o}\0", 1_700_000_000u64);
        h[OFF_MTIME..OFF_MTIME + mtime_field.len()].copy_from_slice(mtime_field.as_bytes());
        h[OFF_TYPEFLAG] = typeflag;
        h[OFF_MAGIC..OFF_MAGIC + 6].copy_from_slice(b"ustar\0");

        // The sum is taken with the field filled with spaces.
        h[OFF_CHKSUM..OFF_CHKSUM + LEN_CHKSUM].fill(b' ');
        let sum: u64 = h.iter().map(|&b| b as u64).sum();
        let sum_field = format!("{sum:06o}\0 ");
        h[OFF_CHKSUM..OFF_CHKSUM + sum_field.len()].copy_from_slice(sum_field.as_bytes());
        h
    }

    fn with_body(header: Vec<u8>, body: &[u8]) -> Vec<u8> {
        let mut out = header;
        out.extend_from_slice(body);
        let pad = (BLOCK - body.len() % BLOCK) % BLOCK;
        out.extend(std::iter::repeat_n(0u8, pad));
        out
    }

    #[test]
    fn parses_single_file() {
        let data = with_body(make_header("hello.txt", 5, b'0'), b"world");
        let entries = parse_tar(&data).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "hello.txt");
        assert_eq!(entries[0].size, 5);
        assert_eq!(entries[0].data_offset, BLOCK as u64);
        assert!(!entries[0].is_dir);
    }

    #[test]
    fn data_offset_points_at_real_content() {
        let data = with_body(make_header("a.txt", 5, b'0'), b"world");
        let e = &parse_tar(&data).unwrap()[0];
        let start = e.data_offset as usize;
        assert_eq!(&data[start..start + e.size as usize], b"world");
    }

    #[test]
    fn parses_directory_entry() {
        let data = with_body(make_header("docs/", 0, b'5'), b"");
        let entries = parse_tar(&data).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_dir);
    }

    #[test]
    fn several_entries_follow_each_other() {
        let mut data = with_body(make_header("a.txt", 3, b'0'), b"aaa");
        data.extend(with_body(make_header("b.txt", 600, b'0'), &[b'b'; 600]));
        let entries = parse_tar(&data).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[1].path, "b.txt");
        // The second file is longer than a block: check that padding is honoured.
        let start = entries[1].data_offset as usize;
        assert_eq!(data[start], b'b');
        assert_eq!(data[start + 599], b'b');
    }

    #[test]
    fn gnu_long_name_overrides_header_name() {
        let long = "очень/длинный/путь/".repeat(8) + "файл.txt";
        let mut data = with_body(
            make_header("././@LongLink", long.len() as u64, b'L'),
            long.as_bytes(),
        );
        data.extend(with_body(make_header("short.txt", 2, b'0'), b"hi"));
        let entries = parse_tar(&data).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, long);
    }

    /// Builds a PAX record. The leading length includes itself, so it has to
    /// be found by iteration — exactly as a real tar does it.
    fn pax_record(key: &str, value: &str) -> Vec<u8> {
        let payload = format!("{key}={value}\n");
        let mut len = payload.len() + 2;
        loop {
            let candidate = format!("{len} {payload}");
            if candidate.len() == len {
                return candidate.into_bytes();
            }
            len = candidate.len();
        }
    }

    #[test]
    fn pax_record_length_includes_itself() {
        let r = pax_record("path", "a.txt");
        let space = r.iter().position(|&b| b == b' ').unwrap();
        let declared: usize = std::str::from_utf8(&r[..space]).unwrap().parse().unwrap();
        assert_eq!(declared, r.len());
    }

    #[test]
    fn pax_path_overrides_header_name() {
        let record = pax_record("path", "путь/а.txt");
        let mut data = with_body(
            make_header("PaxHeaders/0", record.len() as u64, b'x'),
            &record,
        );
        data.extend(with_body(make_header("short.txt", 2, b'0'), b"hi"));
        let entries = parse_tar(&data).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "путь/а.txt");
    }

    #[test]
    fn pax_size_overrides_header_size() {
        // For files over 8 GB the size comes in PAX, with zero in the header.
        let record = pax_record("size", "5");
        let mut data = with_body(
            make_header("PaxHeaders/0", record.len() as u64, b'x'),
            &record,
        );
        data.extend(with_body(make_header("big.bin", 0, b'0'), b"12345"));
        let entries = parse_tar(&data).unwrap();
        assert_eq!(entries[0].size, 5);
    }

    #[test]
    fn ustar_prefix_is_joined_with_name() {
        let mut h = make_header("name.txt", 1, b'0');
        h[OFF_PREFIX..OFF_PREFIX + 7].copy_from_slice(b"pre/fix");
        // Recompute the checksum after the edit.
        h[OFF_CHKSUM..OFF_CHKSUM + LEN_CHKSUM].fill(b' ');
        let sum: u64 = h.iter().map(|&b| b as u64).sum();
        let sum_field = format!("{sum:06o}\0 ");
        h[OFF_CHKSUM..OFF_CHKSUM + sum_field.len()].copy_from_slice(sum_field.as_bytes());

        let data = with_body(h, b"x");
        assert_eq!(parse_tar(&data).unwrap()[0].path, "pre/fix/name.txt");
    }

    #[test]
    fn symlinks_and_devices_are_skipped() {
        let mut data = with_body(make_header("link", 0, b'2'), b"");
        data.extend(with_body(make_header("real.txt", 1, b'0'), b"x"));
        let entries = parse_tar(&data).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "real.txt");
    }

    #[test]
    fn stops_at_end_of_archive_blocks() {
        let mut data = with_body(make_header("a.txt", 1, b'0'), b"x");
        data.extend(std::iter::repeat_n(0u8, BLOCK * 2));
        assert_eq!(parse_tar(&data).unwrap().len(), 1);
    }

    #[test]
    fn base256_size_field_is_understood() {
        // GNU writes sizes over 8 GB as a binary field flagged by the high bit.
        let mut field = [0u8; 12];
        field[0] = 0x80;
        field[11] = 0x2a;
        assert_eq!(numeric_field(&field), Some(42));
    }

    #[test]
    fn octal_field_with_spaces_is_understood() {
        assert_eq!(numeric_field(b"0000644 \0   "), Some(0o644));
    }

    #[test]
    fn detects_tar_by_magic_and_checksum() {
        let data = with_body(make_header("a.txt", 1, b'0'), b"x");
        assert!(looks_like_tar(&data));
        assert!(!looks_like_tar(&[0u8; BLOCK]));
        assert!(!looks_like_tar(b"PK\x03\x04"));
    }

    #[test]
    fn rejects_garbage_as_first_header() {
        let data = vec![0x41u8; BLOCK * 2];
        assert!(parse_tar(&data).is_err());
    }
}
