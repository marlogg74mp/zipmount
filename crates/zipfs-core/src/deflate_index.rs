//! A checkpoint index for deflate streams — the `zran` technique from the
//! zlib examples.
//!
//! The problem: deflate cannot be read from an arbitrary position, because
//! decompressing each byte depends on the previous 32 KB. The naive answer —
//! restarting the stream from the beginning on every backward read — turns
//! random access on a large entry into hundreds of megabytes of wasted work.
//!
//! The way around: every few megabytes, record enough state to resume — the
//! position in bits inside the compressed stream, plus 32 KB of already
//! decompressed data (the dictionary). Any read then costs at most one span
//! between checkpoints.
//!
//! Restoring the state takes three zlib functions that convenient wrappers do
//! not expose: `inflatePrime` (bring the decoder to the right bit boundary),
//! `inflateSetDictionary` (hand it the window) and `inflateReset2` (raw
//! deflate). They come from `libz-rs-sys` — the same C-compatible interface,
//! but implemented in Rust, so neither a C compiler nor an external library
//! enters the build.

use std::ffi::c_int;

use anyhow::{bail, Context, Result};
use libz_rs_sys::{
    inflate, inflateEnd, inflateInit2_, inflatePrime, inflateReset2, inflateSetDictionary,
    z_stream, zlibVersion, Z_BLOCK, Z_OK, Z_STREAM_END,
};
use zipmount_i18n::t;

/// Size of the deflate sliding window. Fixed by the format.
const WINDOW: usize = 32 * 1024;

/// Span between checkpoints, in decompressed data.
///
/// A straight trade-off: the smaller the span, the cheaper a seek and the
/// fatter the index. At 16 MB, a gigabyte of data gets 64 points — 2 MB of
/// index — and any seek costs at most 16 MB of decompression.
pub const DEFAULT_SPAN: u64 = 16 * 1024 * 1024;

/// Lower bound for the span. There is no point going finer: each point costs
/// 32 KB of dictionary, and below a megabyte the index starts to weigh a
/// noticeable share of the data itself while the gain barely grows.
pub const MIN_SPAN: u64 = 512 * 1024;

/// Memory ceiling for the index of one entry.
///
/// The span is derived from this rather than from a point count: a seek
/// costs one span, so a fixed number of points would make every seek on a
/// large entry more expensive in proportion to its size. With a fixed memory
/// ceiling the span grows much more slowly.
const MAX_INDEX_BYTES: u64 = 32 * 1024 * 1024;

/// An index does not pay off on every entry: for small ones, restarting the
/// stream is cheaper.
///
/// The threshold is compared with the **decompressed** size, not the
/// compressed one: the cost of a restart is how many bytes have to be
/// produced again, not how many lie in the archive.
pub const MIN_INDEXABLE: u64 = 4 * 1024 * 1024;

struct Checkpoint {
    /// Position in the decompressed data.
    out_pos: u64,
    /// How many compressed bytes have been consumed by this point.
    in_pos: u64,
    /// How many bits of the previous byte belong to the next block (0..7).
    bits: u8,
    /// The 32 KB of decompressed data before the point — the dictionary to
    /// resume with.
    window: Box<[u8]>,
}

pub struct DeflateIndex {
    points: Vec<Checkpoint>,
    span: u64,
}

impl DeflateIndex {
    /// Builds the index in one pass over the compressed data.
    ///
    /// The pass costs exactly one full decompression, so it makes sense to
    /// call it lazily — once random access has actually happened, not in
    /// advance.
    pub fn build(comp: &[u8], span: u64) -> Result<Self> {
        let span = span.max(WINDOW as u64);
        let mut inflater = Inflater::new_raw()?;
        let strm = inflater.stream();

        // The output buffer is used as a ring: it is the sliding window
        // itself, so at a checkpoint the dictionary is already in it.
        let mut window = vec![0u8; WINDOW];
        let mut points: Vec<Checkpoint> = Vec::new();

        let mut total_in: u64 = 0;
        let mut total_out: u64 = 0;
        let mut last_point: u64 = 0;

        strm.next_in = comp.as_ptr();
        strm.avail_in = 0;
        strm.avail_out = 0;
        let mut fed: usize = 0;

        loop {
            if strm.avail_in == 0 {
                if fed >= comp.len() {
                    break;
                }
                // Feed in portions: avail_in is 32 bits wide.
                let chunk = (comp.len() - fed).min(1 << 30);
                strm.next_in = comp[fed..].as_ptr();
                strm.avail_in = chunk as u32;
                fed += chunk;
            }

            loop {
                if strm.avail_out == 0 {
                    strm.avail_out = WINDOW as u32;
                    strm.next_out = window.as_mut_ptr();
                }

                let before_in = strm.avail_in;
                let before_out = strm.avail_out;
                // SAFETY: strm is initialized; next_in/next_out point to live
                // buffers of the stated sizes.
                let ret = unsafe { inflate(strm, Z_BLOCK) };
                total_in += (before_in - strm.avail_in) as u64;
                total_out += (before_out - strm.avail_out) as u64;

                if ret == Z_STREAM_END {
                    return Ok(Self { points, span });
                }
                if ret != Z_OK {
                    bail!(t!("core-deflate-corrupt", code = ret));
                }

                // Bit 7 of data_type means the decoder stopped at a block
                // boundary; bit 6, that it was the last block. Points go only
                // on boundaries, because only there is the state restorable
                // from a dictionary.
                let at_block_boundary = strm.data_type & 128 != 0;
                let is_last_block = strm.data_type & 64 != 0;
                if at_block_boundary && !is_last_block && total_out - last_point >= span {
                    points.push(Checkpoint {
                        out_pos: total_out,
                        in_pos: total_in,
                        bits: (strm.data_type & 7) as u8,
                        window: snapshot_window(&window, strm.avail_out as usize),
                    });
                    last_point = total_out;
                }

                if strm.avail_in == 0 {
                    break;
                }
            }
        }

        Ok(Self { points, span })
    }

    pub fn point_count(&self) -> usize {
        self.points.len()
    }

    pub fn span(&self) -> u64 {
        self.span
    }

    /// How much memory the index takes.
    pub fn memory_bytes(&self) -> usize {
        self.points.len() * (WINDOW + std::mem::size_of::<Checkpoint>())
    }

    /// The nearest point at or before the given position.
    fn point_before(&self, offset: u64) -> Option<&Checkpoint> {
        let idx = self.points.partition_point(|p| p.out_pos <= offset);
        if idx == 0 {
            None
        } else {
            Some(&self.points[idx - 1])
        }
    }
}

/// Takes the last 32 KB of output, in the right order, from the ring buffer.
fn snapshot_window(window: &[u8], avail_out: usize) -> Box<[u8]> {
    let mut out = vec![0u8; WINDOW];
    let written = WINDOW - avail_out;
    // The older part lies after the write position, the fresher one before it.
    out[..avail_out].copy_from_slice(&window[written..]);
    out[avail_out..].copy_from_slice(&window[..written]);
    out.into_boxed_slice()
}

/// A reader over the index: forwards it streams, backwards it jumps to the
/// nearest checkpoint instead of restarting from the beginning.
pub struct IndexedReader {
    index: Option<DeflateIndex>,
    inflater: Inflater,
    /// Current position in the decompressed data.
    out_pos: u64,
    /// How many compressed bytes have been fed to the decoder.
    in_off: usize,
    /// The decoder is usable (after a reset and until an error).
    ready: bool,
    span: u64,
    /// Decompressed size of the entry — decides whether an index pays off.
    total_out: u64,
    scratch: Vec<u8>,
}

impl IndexedReader {
    /// `span_cap` is the upper bound for the span; the actual span is chosen
    /// by the entry's size.
    ///
    /// A fixed span does not work here: on a 12 MB entry a 16 MB span yields
    /// not a single point and the index comes out empty — exactly the mistake
    /// that was made at first.
    pub fn new(span_cap: u64, total_out: u64) -> Result<Self> {
        // There will be total_out/span points of 32 KB each. Hence the
        // smallest span at which the index fits in its memory allowance.
        let by_memory = total_out.saturating_mul(WINDOW as u64) / MAX_INDEX_BYTES;
        let span = by_memory.clamp(MIN_SPAN, span_cap.max(MIN_SPAN));
        Ok(Self {
            index: None,
            inflater: Inflater::new_raw()?,
            out_pos: 0,
            in_off: 0,
            ready: false,
            span,
            total_out,
            scratch: vec![0u8; 64 * 1024],
        })
    }

    pub fn index_points(&self) -> usize {
        self.index.as_ref().map_or(0, |i| i.point_count())
    }

    pub fn has_index(&self) -> bool {
        self.index.is_some()
    }

    /// Reads `buf` starting at `offset` in the decompressed data.
    pub fn read_at(&mut self, comp: &[u8], offset: u64, buf: &mut [u8]) -> Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        self.position_at(comp, offset)?;

        let mut written = 0usize;
        while written < buf.len() {
            let produced = self.pump(comp, &mut buf[written..])?;
            if produced == 0 {
                break;
            }
            written += produced;
        }
        Ok(written)
    }

    /// Brings the decoder to the wanted position the cheapest way available.
    fn position_at(&mut self, comp: &[u8], offset: u64) -> Result<()> {
        // Already there — nothing to do.
        if self.ready && self.out_pos == offset {
            return Ok(());
        }

        // Forwards is always possible; backwards only through a point.
        if self.ready && self.out_pos < offset {
            let from_index = self
                .index
                .as_ref()
                .and_then(|i| i.point_before(offset))
                .map_or(0, |p| p.out_pos);
            // If a checkpoint is closer to the target than the current
            // position, jumping to it is cheaper.
            if from_index <= self.out_pos {
                return self.skip_to(comp, offset);
            }
            self.restore(comp, offset)?;
            return self.skip_to(comp, offset);
        }

        // Build the index only when the stream would otherwise have to be
        // replayed from the very beginning — that is, on a backward read. A
        // cold start does not need it: the wanted spot has to be reached once
        // anyway.
        let going_back = self.ready && offset < self.out_pos;
        if going_back && self.index.is_none() && self.total_out >= MIN_INDEXABLE {
            self.index = Some(DeflateIndex::build(comp, self.span)?);
        }

        self.restore(comp, offset)?;
        self.skip_to(comp, offset)
    }

    /// Returns the decoder to the state of the nearest checkpoint.
    fn restore(&mut self, comp: &[u8], offset: u64) -> Result<()> {
        let point = self
            .index
            .as_ref()
            .and_then(|i| i.point_before(offset))
            .map(|p| (p.out_pos, p.in_pos, p.bits, p.window.clone()));

        let strm = self.inflater.stream();
        // SAFETY: strm is initialized; -15 means raw deflate without a header.
        let ret = unsafe { inflateReset2(strm, -15) };
        if ret != Z_OK {
            bail!("failed to reset the deflate decoder: zlib code {ret}");
        }

        match point {
            Some((out_pos, in_pos, bits, window)) => {
                let mut start =
                    usize::try_from(in_pos).context("checkpoint offset does not fit in usize")?;
                if bits > 0 {
                    // The point falls mid-byte: some of its bits already
                    // belong to the next block and must be given back to the
                    // decoder.
                    start -= 1;
                    let byte = *comp
                        .get(start)
                        .context("checkpoint points past the end of the data")?;
                    let value = (byte >> (8 - bits)) as c_int;
                    // SAFETY: strm was just reset; bits is within 1..=7.
                    let ret = unsafe { inflatePrime(strm, bits as c_int, value) };
                    if ret != Z_OK {
                        bail!("inflatePrime returned zlib code {ret}");
                    }
                    start += 1;
                }

                // SAFETY: window lives until the end of the call and is
                // WINDOW long.
                let ret = unsafe { inflateSetDictionary(strm, window.as_ptr(), WINDOW as u32) };
                if ret != Z_OK {
                    bail!("inflateSetDictionary returned zlib code {ret}");
                }

                self.in_off = start;
                self.out_pos = out_pos;
            }
            None => {
                self.in_off = 0;
                self.out_pos = 0;
            }
        }

        strm.avail_in = 0;
        strm.avail_out = 0;
        self.ready = true;
        Ok(())
    }

    /// Skips forward to the wanted position.
    fn skip_to(&mut self, comp: &[u8], offset: u64) -> Result<()> {
        while self.out_pos < offset {
            let want = ((offset - self.out_pos) as usize).min(self.scratch.len());
            let mut scratch = std::mem::take(&mut self.scratch);
            let produced = self.pump(comp, &mut scratch[..want]);
            self.scratch = scratch;
            if produced? == 0 {
                // The stream ended before the requested offset.
                break;
            }
        }
        Ok(())
    }

    /// One step of decompression. Returns the number of bytes produced; 0
    /// means the stream is exhausted.
    fn pump(&mut self, comp: &[u8], out: &mut [u8]) -> Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }

        let in_off = self.in_off;
        let strm = self.inflater.stream();

        if strm.avail_in == 0 && in_off < comp.len() {
            let chunk = (comp.len() - in_off).min(1 << 30);
            strm.next_in = comp[in_off..].as_ptr();
            strm.avail_in = chunk as u32;
        }

        strm.next_out = out.as_mut_ptr();
        strm.avail_out = out.len() as u32;

        let before_in = strm.avail_in;
        let before_out = strm.avail_out;
        // SAFETY: the buffers are alive and their lengths match
        // avail_in/avail_out.
        let ret = unsafe { inflate(strm, Z_BLOCK) };

        let consumed = (before_in - strm.avail_in) as usize;
        let produced = (before_out - strm.avail_out) as usize;
        self.in_off += consumed;
        self.out_pos += produced as u64;

        if ret != Z_OK && ret != Z_STREAM_END {
            self.ready = false;
            bail!(t!("core-deflate-corrupt", code = ret));
        }
        Ok(produced)
    }
}

/// Owner of a `z_stream` that closes it on drop.
struct Inflater {
    strm: Box<z_stream>,
}

impl Inflater {
    fn new_raw() -> Result<Self> {
        let mut strm = Box::new(z_stream::default());
        // SAFETY: strm was just created and is not initialized twice.
        // -15 = raw deflate: zip and gzip bodies carry no zlib header.
        let ret = unsafe {
            inflateInit2_(
                strm.as_mut(),
                -15,
                zlibVersion(),
                std::mem::size_of::<z_stream>() as c_int,
            )
        };
        if ret != Z_OK {
            bail!("failed to initialize the deflate decoder: zlib code {ret}");
        }
        Ok(Self { strm })
    }

    fn stream(&mut self) -> &mut z_stream {
        self.strm.as_mut()
    }
}

// SAFETY: z_stream holds raw pointers, which makes it !Send, but moving the
// decoder between threads is safe. Concurrent access is ruled out by the
// type: every operation takes &mut, and further up the stack the handle sits
// under a mutex. Between calls, next_in/next_out point into the archive data
// itself, which outlives the decoder and does not move.
// Sync is deliberately not implemented: shared access from several threads
// is not needed.
unsafe impl Send for Inflater {}

impl Drop for Inflater {
    fn drop(&mut self) {
        // SAFETY: strm was successfully initialized in new_raw.
        unsafe {
            inflateEnd(self.strm.as_mut());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::DeflateEncoder;
    use flate2::Compression;
    use std::io::Write;

    /// Pseudo-text: compresses well but does not degenerate into one
    /// repeating sequence, which would make checking offsets meaningless.
    fn sample(len: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(len + 64);
        let mut i = 0usize;
        while out.len() < len {
            out.extend_from_slice(
                format!("line {i} of text for checking random access\n").as_bytes(),
            );
            i += 1;
        }
        out.truncate(len);
        out
    }

    fn deflate(data: &[u8]) -> Vec<u8> {
        let mut enc = DeflateEncoder::new(Vec::new(), Compression::default());
        enc.write_all(data).unwrap();
        enc.finish().unwrap()
    }

    #[test]
    fn reads_sequentially_without_index() {
        let data = sample(300_000);
        let comp = deflate(&data);
        let mut r = IndexedReader::new(DEFAULT_SPAN, data.len() as u64).unwrap();

        let mut got = Vec::new();
        let mut buf = [0u8; 8192];
        let mut off = 0u64;
        loop {
            let n = r.read_at(&comp, off, &mut buf).unwrap();
            if n == 0 {
                break;
            }
            got.extend_from_slice(&buf[..n]);
            off += n as u64;
        }
        assert_eq!(got.len(), data.len());
        assert!(got == data, "sequential read diverged");
    }

    #[test]
    fn serves_arbitrary_offsets() {
        let data = sample(2_000_000);
        let comp = deflate(&data);
        let mut r = IndexedReader::new(64 * 1024, data.len() as u64).unwrap();

        for &off in &[1_500_000u64, 10, 900_000, 0, 1_999_000, 500_000, 1_200_000] {
            let mut buf = [0u8; 4096];
            let n = r.read_at(&comp, off, &mut buf).unwrap();
            let end = (off as usize + n).min(data.len());
            assert!(n > 0, "a read at {off} must not be empty");
            assert_eq!(
                &buf[..n],
                &data[off as usize..end],
                "mismatch reading at {off}"
            );
        }
    }

    #[test]
    fn index_is_built_and_used_for_backward_seeks() {
        // Enough data for an index to make sense; a small span for more points.
        let data = sample(6 * 1024 * 1024);
        let comp = deflate(&data);
        let mut r = IndexedReader::new(256 * 1024, data.len() as u64).unwrap();

        let mut buf = [0u8; 1024];
        r.read_at(&comp, 5_000_000, &mut buf).unwrap();
        assert!(!r.has_index(), "moving forwards needs no index");

        r.read_at(&comp, 1_000, &mut buf).unwrap();
        assert!(r.has_index(), "a backward read must build the index");
        assert!(r.index_points() > 1, "there must be several points");
        assert_eq!(&buf[..], &data[1_000..1_000 + 1024]);
    }

    #[test]
    fn index_points_land_on_block_boundaries() {
        let data = sample(4 * 1024 * 1024);
        let comp = deflate(&data);
        let index = DeflateIndex::build(&comp, 256 * 1024).unwrap();
        assert!(index.point_count() > 0, "the index must not be empty");

        // Every point must restore faithfully.
        for point in &index.points {
            let mut r = IndexedReader::new(256 * 1024, data.len() as u64).unwrap();
            let mut buf = [0u8; 512];
            let n = r.read_at(&comp, point.out_pos, &mut buf).unwrap();
            assert!(n > 0);
            let end = point.out_pos as usize + n;
            assert_eq!(
                &buf[..n],
                &data[point.out_pos as usize..end],
                "the point at {} does not restore",
                point.out_pos
            );
        }
    }

    #[test]
    fn backward_seek_is_exact_at_every_point() {
        let data = sample(3 * 1024 * 1024);
        let comp = deflate(&data);
        let mut r = IndexedReader::new(128 * 1024, data.len() as u64).unwrap();

        // Go to the end first, then read backwards out of order.
        let mut buf = [0u8; 2048];
        r.read_at(&comp, 3_000_000, &mut buf).unwrap();

        let mut offsets: Vec<u64> = (0..3_000_000u64).step_by(271_733).collect();
        offsets.reverse();
        for off in offsets {
            let n = r.read_at(&comp, off, &mut buf).unwrap();
            let end = (off as usize + n).min(data.len());
            assert_eq!(&buf[..n], &data[off as usize..end], "diverged at {off}");
        }
    }

    #[test]
    fn past_end_returns_nothing() {
        let data = sample(50_000);
        let comp = deflate(&data);
        let mut r = IndexedReader::new(DEFAULT_SPAN, data.len() as u64).unwrap();
        let mut buf = [0u8; 256];
        assert_eq!(r.read_at(&comp, 50_000, &mut buf).unwrap(), 0);
        assert_eq!(r.read_at(&comp, 999_999, &mut buf).unwrap(), 0);
    }

    #[test]
    fn small_entries_do_not_build_an_index() {
        // For a small entry a restart is cheaper than an index.
        let data = sample(100_000);
        let comp = deflate(&data);
        let mut r = IndexedReader::new(DEFAULT_SPAN, data.len() as u64).unwrap();
        let mut buf = [0u8; 512];
        r.read_at(&comp, 90_000, &mut buf).unwrap();
        r.read_at(&comp, 10, &mut buf).unwrap();
        assert!(!r.has_index(), "an index does not pay off on a small entry");
        assert_eq!(&buf[..], &data[10..10 + 512]);
    }

    #[test]
    fn span_grows_slower_than_entry_size() {
        // The key property of the chosen policy: when the entry grows 100
        // times, the span grows but stays bounded, and the index stays within
        // its ceiling.
        let small = IndexedReader::new(DEFAULT_SPAN, 10 * 1024 * 1024).unwrap();
        let large = IndexedReader::new(DEFAULT_SPAN, 1024 * 1024 * 1024).unwrap();
        assert_eq!(small.span, MIN_SPAN, "a small entry gets the minimal span");
        assert!(
            large.span > small.span,
            "a large one must get a larger span"
        );
        assert!(large.span <= DEFAULT_SPAN, "but not above the cap");

        let points = 1024 * 1024 * 1024 / large.span;
        assert!(
            points * WINDOW as u64 <= MAX_INDEX_BYTES,
            "the index must stay within its memory allowance"
        );
    }

    #[test]
    fn index_memory_is_proportional_to_points() {
        let data = sample(4 * 1024 * 1024);
        let comp = deflate(&data);
        let index = DeflateIndex::build(&comp, 512 * 1024).unwrap();
        let expected = index.point_count() * WINDOW;
        assert!(index.memory_bytes() >= expected);
        // The index must be many times smaller than the data itself.
        assert!(index.memory_bytes() < data.len());
    }
}
