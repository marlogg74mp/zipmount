//! A format-independent description of an archive entry.
//!
//! The directory tree is built from this alone: the name is already decoded,
//! so dealing with encodings stays inside the zip backend and does not leak
//! out (7z names are always Unicode, so there is nothing to decode).

#[derive(Debug, Clone)]
pub struct EntryMeta {
    /// Path inside the archive as the archiver wrote it, with either separator.
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    /// Modification time as a Windows FILETIME; 0 means unknown.
    pub mtime: u64,
}

/// Format of an open archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Zip,
    SevenZ,
    Tar,
    /// A tar compressed with gzip.
    TarGz,
    Rar,
}

impl Format {
    pub fn name(self) -> &'static str {
        match self {
            Format::Zip => "zip",
            Format::SevenZ => "7z",
            Format::Tar => "tar",
            Format::TarGz => "tar.gz",
            Format::Rar => "rar",
        }
    }

    /// Detects the format by the signature at the start of the file, not by
    /// the extension: a file name lies more often than its first bytes do.
    ///
    /// Pass at least 512 bytes: the tar magic sits at offset 257, and very
    /// old tars have none at all, leaving a header checksum that adds up as
    /// the only sign.
    pub fn detect(head: &[u8]) -> Option<Self> {
        const SEVENZ_MAGIC: &[u8] = &[b'7', b'z', 0xBC, 0xAF, 0x27, 0x1C];
        if head.starts_with(SEVENZ_MAGIC) {
            return Some(Format::SevenZ);
        }
        // A local header, an empty archive, or a spanning marker.
        if head.starts_with(b"PK\x03\x04")
            || head.starts_with(b"PK\x05\x06")
            || head.starts_with(b"PK\x07\x08")
        {
            return Some(Format::Zip);
        }
        // Both RAR versions start the same way: 4.x adds 0x00, 5.x adds
        // 0x01 0x00. We need not tell them apart; the unpacker does.
        if head.starts_with(b"Rar!\x1a\x07") {
            return Some(Format::Rar);
        }
        // gzip. Whether there really is a tar inside only shows after
        // decompression.
        if head.starts_with(&[0x1f, 0x8b]) {
            return Some(Format::TarGz);
        }
        if crate::tarfmt::looks_like_tar(head) {
            return Some(Format::Tar);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_zip_signature() {
        assert_eq!(Format::detect(b"PK\x03\x04rest"), Some(Format::Zip));
        assert_eq!(Format::detect(b"PK\x05\x06"), Some(Format::Zip));
    }

    #[test]
    fn detects_sevenz_signature() {
        assert_eq!(
            Format::detect(&[b'7', b'z', 0xBC, 0xAF, 0x27, 0x1C, 0, 4]),
            Some(Format::SevenZ)
        );
    }

    #[test]
    fn detects_rar_signatures() {
        // RAR 4.x and RAR 5.x differ in the tail of the signature.
        assert_eq!(Format::detect(b"Rar!\x1a\x07\x00rest"), Some(Format::Rar));
        assert_eq!(
            Format::detect(b"Rar!\x1a\x07\x01\x00rest"),
            Some(Format::Rar)
        );
        assert_eq!(Format::detect(b"Rar\x00"), None);
    }

    #[test]
    fn detects_gzip_signature() {
        assert_eq!(
            Format::detect(&[0x1f, 0x8b, 0x08, 0x00]),
            Some(Format::TarGz)
        );
    }

    #[test]
    fn rejects_unknown_and_short_input() {
        assert_eq!(Format::detect(b"RAR!"), None);
        assert_eq!(Format::detect(b"PK"), None);
        assert_eq!(Format::detect(b""), None);
    }
}
