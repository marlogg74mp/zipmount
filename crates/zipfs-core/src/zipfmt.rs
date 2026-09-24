//! Parsing ZIP structures: EOCD, zip64 and the central directory.
//!
//! The parser is our own on purpose rather than the `zip` crate: we need raw
//! local header offsets, full control over name encodings, and access to the
//! extra fields (zip64, NTFS times, Info-ZIP Unicode Path).

use anyhow::{bail, Context, Result};
use zipmount_i18n::t;

pub const SIG_EOCD: u32 = 0x0605_4b50;
pub const SIG_EOCD64_LOCATOR: u32 = 0x0706_4b50;
pub const SIG_EOCD64: u32 = 0x0606_4b50;
pub const SIG_CENTRAL: u32 = 0x0201_4b50;
pub const SIG_LOCAL: u32 = 0x0403_4b50;

pub const METHOD_STORED: u16 = 0;
pub const METHOD_DEFLATE: u16 = 8;

const FLAG_ENCRYPTED: u16 = 1 << 0;
const FLAG_UTF8: u16 = 1 << 11;

/// The fixed part of a central directory file header.
const CENTRAL_HEADER_LEN: usize = 46;
/// The fixed part of a local file header.
const LOCAL_HEADER_LEN: u64 = 30;
/// Minimum EOCD size without a comment.
const EOCD_MIN: usize = 22;

fn rd_u16(buf: &[u8], off: usize) -> Result<u16> {
    let b = buf
        .get(off..off + 2)
        .with_context(|| format!("u16 read past the end of the buffer (offset {off})"))?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}

fn rd_u32(buf: &[u8], off: usize) -> Result<u32> {
    let b = buf
        .get(off..off + 4)
        .with_context(|| format!("u32 read past the end of the buffer (offset {off})"))?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn rd_u64(buf: &[u8], off: usize) -> Result<u64> {
    let b = buf
        .get(off..off + 8)
        .with_context(|| format!("u64 read past the end of the buffer (offset {off})"))?;
    Ok(u64::from_le_bytes([
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
    ]))
}

/// WinZip AES encryption details from extra field 0x9901.
///
/// Such an entry has 99 as its main compression method, and the real one
/// lies here: without parsing this field the entry looks like "unknown
/// compression method".
#[derive(Debug, Clone, Copy)]
pub struct AesInfo {
    pub vendor_version: u16,
    /// 1 = AES-128, 2 = AES-192, 3 = AES-256.
    pub strength: u8,
    /// The compression method applied before encryption.
    pub actual_method: u16,
}

/// A central directory entry in raw form: the name is not decoded yet,
/// because the encoding is chosen a level up (see the `names` module).
#[derive(Debug, Clone)]
pub struct RawEntry {
    pub name_raw: Vec<u8>,
    /// The name from the Info-ZIP Unicode Path Extra Field (0x7075), if there
    /// is one and its CRC matches the raw name.
    pub unicode_path: Option<String>,
    pub flags: u16,
    pub method: u16,
    pub crc32: u32,
    pub comp_size: u64,
    pub uncomp_size: u64,
    pub local_header_offset: u64,
    pub dos_time: u16,
    pub dos_date: u16,
    /// FILETIME from the NTFS extra field (0x000A) — more precise than DOS time.
    pub ntfs_mtime: Option<u64>,
    /// Unix time from the extended timestamp (0x5455).
    pub unix_mtime: Option<i64>,
    pub external_attrs: u32,
    pub host_system: u8,
    /// Set only for WinZip AES entries.
    pub aes: Option<AesInfo>,
}

impl RawEntry {
    /// The compression method, bearing in mind that for encrypted entries the
    /// real method hides in an extra field behind a placeholder 99.
    pub fn effective_method(&self) -> u16 {
        match self.aes {
            Some(info) => info.actual_method,
            None => self.method,
        }
    }

    /// The entry's CRC32, if it is stored at all.
    ///
    /// In WinZip AES version AE-2 the field is zeroed on purpose: integrity is
    /// confirmed by the HMAC computed during decryption, not by a checksum.
    /// Comparing the contents with zero would make no sense.
    pub fn stored_crc32(&self) -> Option<u32> {
        match self.aes {
            Some(info) if info.vendor_version >= 2 => None,
            _ => Some(self.crc32),
        }
    }

    pub fn has_utf8_flag(&self) -> bool {
        self.flags & FLAG_UTF8 != 0
    }

    pub fn is_encrypted(&self) -> bool {
        self.flags & FLAG_ENCRYPTED != 0
    }

    /// A directory is recognised by three independent signs: the trailing
    /// slash is the most reliable; the others cover archivers that omit it.
    pub fn is_dir(&self) -> bool {
        if matches!(self.name_raw.last(), Some(b'/') | Some(b'\\')) {
            return true;
        }
        // FILE_ATTRIBUTE_DIRECTORY in the DOS attributes.
        if self.external_attrs & 0x10 != 0 {
            return true;
        }
        // S_IFDIR in the high 16 bits, if the archive was made on Unix.
        if self.host_system == 3 && (self.external_attrs >> 16) & 0xF000 == 0x4000 {
            return true;
        }
        false
    }

    /// Modification time as a Windows FILETIME, from the most precise source
    /// available.
    pub fn mtime_filetime(&self) -> u64 {
        if let Some(ft) = self.ntfs_mtime {
            return ft;
        }
        if let Some(unix) = self.unix_mtime {
            return unix_to_filetime(unix);
        }
        dos_to_filetime(self.dos_date, self.dos_time)
    }
}

/// Where the central directory lies. Values are already taken from zip64, if
/// there was one.
#[derive(Debug, Clone, Copy)]
pub struct CdLocation {
    pub offset: u64,
    pub size: u64,
    pub entries: u64,
}

/// Finds the EOCD from the end of the file. Checks not only the signature
/// but also that the comment length reaches exactly to the end of the file —
/// otherwise a stray signature inside a comment or compressed data is easy
/// to catch.
pub fn find_eocd(buf: &[u8]) -> Result<usize> {
    if buf.len() < EOCD_MIN {
        bail!(t!("core-zip-too-small", size = buf.len()));
    }
    let max_comment = 0xFFFF_usize;
    let scan_start = buf.len().saturating_sub(EOCD_MIN + max_comment);
    let mut i = buf.len() - EOCD_MIN;
    loop {
        if rd_u32(buf, i).unwrap_or(0) == SIG_EOCD {
            let comment_len = rd_u16(buf, i + 20).unwrap_or(0) as usize;
            if i + EOCD_MIN + comment_len == buf.len() {
                return Ok(i);
            }
        }
        if i == scan_start {
            break;
        }
        i -= 1;
    }
    bail!(t!("core-zip-no-eocd"))
}

/// Locates the central directory, following zip64 where needed.
pub fn locate_central_directory(buf: &[u8]) -> Result<CdLocation> {
    let eocd = find_eocd(buf)?;
    let mut entries = rd_u16(buf, eocd + 10)? as u64;
    let mut size = rd_u32(buf, eocd + 12)? as u64;
    let mut offset = rd_u32(buf, eocd + 16)? as u64;

    // Any field at its maximum means the real value is in zip64.
    let needs_zip64 = entries == 0xFFFF || size == 0xFFFF_FFFF || offset == 0xFFFF_FFFF;

    let mut found_zip64 = false;
    if eocd >= 20 {
        let loc = eocd - 20;
        if rd_u32(buf, loc).unwrap_or(0) == SIG_EOCD64_LOCATOR {
            let z64_off = usize::try_from(rd_u64(buf, loc + 8)?)
                .context("zip64 EOCD offset does not fit in usize")?;
            if rd_u32(buf, z64_off).unwrap_or(0) == SIG_EOCD64 {
                entries = rd_u64(buf, z64_off + 32)?;
                size = rd_u64(buf, z64_off + 40)?;
                offset = rd_u64(buf, z64_off + 48)?;
                found_zip64 = true;
            }
        }
    }
    if needs_zip64 && !found_zip64 {
        bail!(t!("core-zip64-missing"));
    }

    Ok(CdLocation {
        offset,
        size,
        entries,
    })
}

/// Full parse of the central directory. One pass, no file bodies read.
pub fn parse_central_directory(buf: &[u8]) -> Result<Vec<RawEntry>> {
    let cd = locate_central_directory(buf)?;
    let start =
        usize::try_from(cd.offset).context("central directory offset does not fit in usize")?;
    let end = start
        .checked_add(
            usize::try_from(cd.size).context("central directory size does not fit in usize")?,
        )
        .with_context(|| t!("core-zip-cd-out-of-bounds"))?;
    let cdb = buf
        .get(start..end)
        .with_context(|| t!("core-zip-cd-out-of-bounds"))?;

    // `entries` from the EOCD is a hint for reserving, but it cannot be
    // trusted: cap it, so a damaged header cannot order a gigabyte allocation.
    let mut out = Vec::with_capacity(cd.entries.min(1 << 20) as usize);

    let mut p = 0usize;
    while p + CENTRAL_HEADER_LEN <= cdb.len() {
        if rd_u32(cdb, p)? != SIG_CENTRAL {
            break;
        }
        let version_made_by = rd_u16(cdb, p + 4)?;
        let flags = rd_u16(cdb, p + 8)?;
        let method = rd_u16(cdb, p + 10)?;
        let dos_time = rd_u16(cdb, p + 12)?;
        let dos_date = rd_u16(cdb, p + 14)?;
        let crc32_field = rd_u32(cdb, p + 16)?;
        let comp_size_32 = rd_u32(cdb, p + 20)?;
        let uncomp_size_32 = rd_u32(cdb, p + 24)?;
        let name_len = rd_u16(cdb, p + 28)? as usize;
        let extra_len = rd_u16(cdb, p + 30)? as usize;
        let comment_len = rd_u16(cdb, p + 32)? as usize;
        let disk_start = rd_u16(cdb, p + 34)?;
        let external_attrs = rd_u32(cdb, p + 38)?;
        let local_offset_32 = rd_u32(cdb, p + 42)?;

        let name_at = p + CENTRAL_HEADER_LEN;
        let extra_at = name_at + name_len;
        let comment_at = extra_at + extra_len;
        let next = comment_at + comment_len;
        if next > cdb.len() {
            bail!(t!("core-zip-cd-out-of-bounds"));
        }

        let mut entry = RawEntry {
            name_raw: cdb[name_at..extra_at].to_vec(),
            unicode_path: None,
            flags,
            method,
            crc32: crc32_field,
            comp_size: comp_size_32 as u64,
            uncomp_size: uncomp_size_32 as u64,
            local_header_offset: local_offset_32 as u64,
            dos_time,
            dos_date,
            ntfs_mtime: None,
            unix_mtime: None,
            external_attrs,
            host_system: (version_made_by >> 8) as u8,
            aes: None,
        };

        let sentinels = Zip64Sentinels {
            uncomp: uncomp_size_32 == 0xFFFF_FFFF,
            comp: comp_size_32 == 0xFFFF_FFFF,
            offset: local_offset_32 == 0xFFFF_FFFF,
            disk: disk_start == 0xFFFF,
        };
        parse_extra_fields(&cdb[extra_at..comment_at], sentinels, &mut entry);

        out.push(entry);
        p = next;
    }

    Ok(out)
}

/// Which fields of the main header turned out to be placeholders and must be
/// read from the zip64 extra field — strictly in this order.
#[derive(Clone, Copy)]
struct Zip64Sentinels {
    uncomp: bool,
    comp: bool,
    offset: bool,
    disk: bool,
}

fn parse_extra_fields(extra: &[u8], sentinels: Zip64Sentinels, entry: &mut RawEntry) {
    let mut p = 0usize;
    while p + 4 <= extra.len() {
        let (Ok(id), Ok(size)) = (rd_u16(extra, p), rd_u16(extra, p + 2)) else {
            return;
        };
        let size = size as usize;
        let body_at = p + 4;
        let Some(body) = extra.get(body_at..body_at + size) else {
            return;
        };

        match id {
            // Zip64 extended information.
            0x0001 => {
                let mut q = 0usize;
                if sentinels.uncomp {
                    if let Ok(v) = rd_u64(body, q) {
                        entry.uncomp_size = v;
                    }
                    q += 8;
                }
                if sentinels.comp {
                    if let Ok(v) = rd_u64(body, q) {
                        entry.comp_size = v;
                    }
                    q += 8;
                }
                if sentinels.offset {
                    if let Ok(v) = rd_u64(body, q) {
                        entry.local_header_offset = v;
                    }
                    q += 8;
                }
                // The disk number is of no use: multi-volume archives are out of
                // scope.
                let _ = (q, sentinels.disk);
            }
            // NTFS: precise times as FILETIME.
            0x000A => {
                let mut q = 4usize; // 4 reserved bytes
                while q + 4 <= body.len() {
                    let tag = rd_u16(body, q).unwrap_or(0);
                    let tsize = rd_u16(body, q + 2).unwrap_or(0) as usize;
                    if tag == 0x0001 && tsize >= 8 {
                        if let Ok(v) = rd_u64(body, q + 4) {
                            entry.ntfs_mtime = Some(v);
                        }
                    }
                    q += 4 + tsize;
                }
            }
            // Info-ZIP Unicode Path: the rescue from mojibake when the UTF-8
            // flag is not set.
            0x7075 => {
                if body.len() > 5 && body[0] == 1 {
                    let name_crc = u32::from_le_bytes([body[1], body[2], body[3], body[4]]);
                    if crc32(&entry.name_raw) == name_crc {
                        if let Ok(s) = std::str::from_utf8(&body[5..]) {
                            entry.unicode_path = Some(s.to_string());
                        }
                    }
                }
            }
            // WinZip AES: the real compression method and the key strength.
            0x9901 if body.len() >= 7 => {
                entry.aes = Some(AesInfo {
                    vendor_version: rd_u16(body, 0).unwrap_or(0),
                    strength: body[4],
                    actual_method: rd_u16(body, 5).unwrap_or(0),
                });
            }
            // Extended timestamp: the low bit of the flags means an mtime is
            // present.
            0x5455 if body.first().is_some_and(|flags| flags & 1 != 0) => {
                if let Ok(v) = rd_u32(body, 1) {
                    entry.unix_mtime = Some(v as i64);
                }
            }
            _ => {}
        }

        p = body_at + size;
    }
}

/// Offset of the start of an entry's data. Computed from the local header,
/// because the extra field length there may differ from the one in the
/// central directory.
pub fn data_offset(buf: &[u8], local_header_offset: u64) -> Result<u64> {
    let lh = usize::try_from(local_header_offset)
        .context("local header offset does not fit in usize")?;
    if rd_u32(buf, lh)? != SIG_LOCAL {
        bail!(t!("core-zip-bad-local-header", offset = lh));
    }
    let name_len = rd_u16(buf, lh + 26)? as u64;
    let extra_len = rd_u16(buf, lh + 28)? as u64;
    Ok(local_header_offset + LOCAL_HEADER_LEN + name_len + extra_len)
}

pub fn crc32(data: &[u8]) -> u32 {
    let mut c = libdeflater::Crc::new();
    c.update(data);
    c.sum()
}

/// Days between 1601-01-01 and 1970-01-01.
const DAYS_1601_TO_1970: i64 = 134_774;
const FILETIME_TICKS_PER_SEC: i64 = 10_000_000;

pub fn unix_to_filetime(unix_secs: i64) -> u64 {
    let ticks = (unix_secs + DAYS_1601_TO_1970 * 86_400) * FILETIME_TICKS_PER_SEC;
    ticks.max(0) as u64
}

/// DOS time has a 2-second resolution and counts from 1980.
pub fn dos_to_filetime(date: u16, time: u16) -> u64 {
    let year = ((date >> 9) & 0x7F) as i64 + 1980;
    let month = ((date >> 5) & 0x0F) as i64;
    let day = (date & 0x1F) as i64;
    let hour = ((time >> 11) & 0x1F) as i64;
    let min = ((time >> 5) & 0x3F) as i64;
    let sec = ((time & 0x1F) as i64) * 2;

    // Empty or broken DOS time: return the start of the FILETIME epoch rather
    // than garbage.
    if month == 0 || day == 0 {
        return 0;
    }

    let days = days_from_civil(year, month, day);
    let secs = (days + DAYS_1601_TO_1970) * 86_400 + hour * 3600 + min * 60 + sec;
    (secs * FILETIME_TICKS_PER_SEC).max(0) as u64
}

/// Days since 1970-01-01 in the Gregorian calendar (Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filetime_to_unix(ft: u64) -> i64 {
        ft as i64 / FILETIME_TICKS_PER_SEC - DAYS_1601_TO_1970 * 86_400
    }

    #[test]
    fn dos_time_epoch_start() {
        // 1980-01-01 00:00:00 — the earliest representable DOS time.
        let date = (((1980 - 1980) as u16) << 9) | (1 << 5) | 1;
        assert_eq!(filetime_to_unix(dos_to_filetime(date, 0)), 315_532_800);
    }

    #[test]
    fn dos_time_known_date() {
        // 2024-03-15 14:30:20; DOS stores seconds divided by 2.
        let date = (((2024 - 1980) as u16) << 9) | (3 << 5) | 15;
        let time = (14u16 << 11) | (30 << 5) | 10;
        assert_eq!(filetime_to_unix(dos_to_filetime(date, time)), 1_710_513_020);
    }

    #[test]
    fn malformed_dos_time_is_not_garbage() {
        assert_eq!(dos_to_filetime(0, 0), 0);
    }

    #[test]
    fn unix_and_dos_agree_on_same_moment() {
        let date = (((2024 - 1980) as u16) << 9) | (3 << 5) | 15;
        let time = (14u16 << 11) | (30 << 5) | 10;
        assert_eq!(dos_to_filetime(date, time), unix_to_filetime(1_710_513_020));
    }

    #[test]
    fn eocd_rejects_non_zip() {
        assert!(find_eocd(&vec![0u8; 1000]).is_err());
    }

    #[test]
    fn eocd_rejects_tiny_file() {
        assert!(find_eocd(b"PK").is_err());
    }

    #[test]
    fn crc32_matches_known_vector() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }
}
