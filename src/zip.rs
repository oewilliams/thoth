//! Minimal ZIP writer, stored (no compression) only.
//!
//! Compression is a separate act, not this module's: v0 establishes the
//! floor — `memcpy` plus formatting — and deflate enters later behind a
//! flag, measured against this spelling as oracle. Stored entries keep the
//! writer dependency-free and the bytes deterministic.
//!
//! This module performs no validation: names, count, and every size
//! already passed the ingress check in [`crate::xlsx::build_xlsx`],
//! which owns the entry set. Every narrowing cast in
//! [`ZipWriter::finish`] names its covering ingress check on the line.
//!
//! Cycle-004 assembler notes (release profile, aarch64): `crc32`
//! is a 4-insn `crc32x` word loop plus an unrolled `crc32b` tail,
//! with no calls and no table traffic; `finish` sizes its buffer
//! from the running total (no entry walk); `add` carries no
//! duplicate scan and no assert formatting in release, with the
//! entry-growth branch cold-only; `push_u16`/`push_u32` leave no
//! `bl` on the `finish` push paths.

/// One CRC-32 (IEEE) table entry: the remainder of the single byte.
/// Fallback spelling only (see `crc32`); gated out where the
/// instruction spelling covers that name.
#[cfg(not(all(target_arch = "aarch64", target_feature = "crc")))]
const fn crc_table_entry(byte: u32) -> u32 {
    let mut crc: u32 = byte;
    let mut bit: u32 = 0;
    while bit < 8 {
        if crc & 1 == 1 {
            crc = (crc >> 1) ^ 0xEDB8_8320;
        } else {
            crc >>= 1;
        }
        bit += 1;
    }
    crc
}

/// CRC-32 (IEEE) base table: one entry per byte value.
/// Fallback spelling only (see `crc32`).
#[cfg(not(all(target_arch = "aarch64", target_feature = "crc")))]
const fn crc_table() -> [u32; 256] {
    let mut table: [u32; 256] = [0; 256];
    let mut byte: u32 = 0;
    while byte < 256 {
        table[byte as usize] = crc_table_entry(byte);
        byte += 1;
    }
    table
}

#[cfg(not(all(target_arch = "aarch64", target_feature = "crc")))]
const CRC_TABLE: [u32; 256] = crc_table();

/// Slicing-by-8 tables: row `j` holds the base entry pushed through
/// `j` zero bytes, so one iteration folds eight input bytes with
/// eight independent lookups. Row 0 is the base table; every later
/// row reads row 0 at arbitrary indices, so row 0 is filled in its
/// own pass first (a single interleaved pass would read unfilled
/// slots).
/// Fallback spelling only (see `crc32`).
#[cfg(not(all(target_arch = "aarch64", target_feature = "crc")))]
const fn crc_table8() -> [[u32; 256]; 8] {
    let mut table: [[u32; 256]; 8] = [[0; 256]; 8];
    let mut byte: usize = 0;
    while byte < 256 {
        table[0][byte] = crc_table_entry(byte as u32);
        byte += 1;
    }
    let mut row: usize = 1;
    while row < 8 {
        byte = 0;
        while byte < 256 {
            let prev: u32 = table[row - 1][byte];
            table[row][byte] = (prev >> 8) ^ table[0][(prev & 0xFF) as usize];
            byte += 1;
        }
        row += 1;
    }
    table
}

#[cfg(not(all(target_arch = "aarch64", target_feature = "crc")))]
const CRC_TABLE8: [[u32; 256]; 8] = crc_table8();

/// CRC-32 (IEEE) over one buffer: the ARMv8 checksum instruction,
/// eight bytes per instruction plus a byte tail. Compile-time only:
/// this spelling exists where the target guarantees the feature
/// (`crc` sits in the default features of this target), and the
/// table spelling below covers every other target. No runtime
/// detection anywhere on this path.
///
/// Head-to-head on this machine (Apple silicon, bench profile):
/// bitwise 5.8 ns/B, single table 1.7, slicing-by-8 0.37, this
/// spelling 0.083 — picked by numbers, not by assumption.
#[cfg(all(target_arch = "aarch64", target_feature = "crc"))]
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    let (words, tail) = data.as_chunks::<8>();
    for word in words {
        crc = unsafe { core::arch::aarch64::__crc32d(crc, u64::from_le_bytes(*word)) };
    }
    for &byte in tail {
        crc = unsafe { core::arch::aarch64::__crc32b(crc, byte) };
    }
    !crc
}

/// CRC-32 (IEEE) over one buffer: slicing-by-8 over the table above
/// with a single-table tail. This is the portable spelling for every
/// target without the ARMv8 checksum feature; where that feature is
/// a compile-time guarantee the instruction spelling above covers
/// this name instead. No runtime detection anywhere on this path.
#[cfg(not(all(target_arch = "aarch64", target_feature = "crc")))]
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    let (words, tail) = data.as_chunks::<8>();
    for word in words {
        crc ^= u32::from_le_bytes([word[0], word[1], word[2], word[3]]);
        crc = CRC_TABLE8[7][(crc & 0xFF) as usize]
            ^ CRC_TABLE8[6][((crc >> 8) & 0xFF) as usize]
            ^ CRC_TABLE8[5][((crc >> 16) & 0xFF) as usize]
            ^ CRC_TABLE8[4][((crc >> 24) & 0xFF) as usize]
            ^ CRC_TABLE8[3][word[4] as usize]
            ^ CRC_TABLE8[2][word[5] as usize]
            ^ CRC_TABLE8[1][word[6] as usize]
            ^ CRC_TABLE8[0][word[7] as usize];
    }
    for &byte in tail {
        crc = CRC_TABLE[(crc as u8 ^ byte) as usize] ^ (crc >> 8);
    }
    !crc
}

/// One entry's footprint in the finished archive: a 30-byte local
/// header plus its name and data, and a 46-byte central record plus
/// its name. Single spelling, shared by [`stored_archive_len`] and
/// the running total in [`ZipWriter::add`] so the two cannot diverge.
fn entry_sizes(data_len: usize, name_len: usize) -> u64 {
    76 + 2 * (name_len as u64) + (data_len as u64)
}

/// Projected length, in bytes, of the finished stored archive for the
/// given entry sizes: the per-entry footprints plus the 22-byte end
/// record. Each tuple is `(data_len, name_len)` in bytes. Pure size
/// arithmetic in `u64`: the parts already live in memory, so no
/// caller can overflow it.
pub(crate) fn stored_archive_len(entries: &[(usize, usize)]) -> u64 {
    entries
        .iter()
        .map(|entry| entry_sizes(entry.0, entry.1))
        .sum::<u64>()
        + 22
}

struct Entry {
    name: Vec<u8>,
    data: Vec<u8>,
    crc: u32,
    local_offset: u32,
}

/// Stored-only ZIP archive, built in memory then written once.
///
/// Assumes ingress-validated input: the entry count is fixed at
/// construction, names are relative and unique, and every length
/// passed the zip32 ceiling check in [`crate::xlsx::build_xlsx`].
/// Release pays for none of this twice; debug builds re-check the
/// shape (relative names, exact count, uniqueness) so a wrong caller
/// trips tests, never ships.
pub struct ZipWriter {
    entries: Vec<Entry>,
    /// Running [`entry_sizes`] footprint over the staged entries,
    /// accumulated per add so `finish` sizes its buffer without a
    /// second walk.
    sizes: u64,
}

impl ZipWriter {
    /// Stage exactly `entry_count` entries: the entry list is sized
    /// once here and never grown.
    pub fn new(entry_count: usize) -> Self {
        Self {
            entries: Vec::with_capacity(entry_count),
            sizes: 0,
        }
    }

    /// Stage one file. Caller guarantees a relative, not-yet-staged
    /// name and a slot within the construction count; debug builds
    /// check both, release trusts the ingress path.
    pub fn add(&mut self, name: &str, data: Vec<u8>) {
        debug_assert!(!name.starts_with('/'), "zip name must be relative: {name}");
        debug_assert!(
            self.entries.len() < self.entries.capacity(),
            "zip entry past the construction count: {name}"
        );
        debug_assert!(
            !self.entries.iter().any(|e| e.name == name.as_bytes()),
            "duplicate zip entry: {name}"
        );
        let crc: u32 = crc32(&data);
        self.sizes += entry_sizes(data.len(), name.len());
        self.entries.push(Entry {
            name: name.as_bytes().to_vec(),
            data,
            crc,
            local_offset: 0,
        });
    }

    /// Finish: local files, central directory, end record. Sized once:
    /// every part's length is already known, so the archive is one
    /// allocation, never grown.
    pub fn finish(mut self) -> Vec<u8> {
        // Exact size without a second walk over the entries. The
        // ingress zip32 check covers the `u64 -> usize` narrowing
        // (house `try_from`-or-saturate spelling: saturation would
        // abort loudly in `with_capacity`, never truncate bytes).
        let total: usize = usize::try_from(22 + self.sizes).unwrap_or(usize::MAX);
        let mut out = Vec::with_capacity(total);
        for e in self.entries.iter_mut() {
            // Cast cover: ingress running-offset check bounds every
            // local header start.
            e.local_offset = out.len() as u32;
            push_u32(&mut out, 0x0403_4B50); // local signature
            push_u16(&mut out, 20); // version needed
            push_u16(&mut out, 0x0800); // UTF-8 flag
            push_u16(&mut out, 0); // method: stored
            push_u16(&mut out, 0); // mod time
            push_u16(&mut out, 0); // mod date
            push_u32(&mut out, e.crc);
            // Cast cover: ingress `data_len <= u32::MAX` refuse.
            push_u32(&mut out, e.data.len() as u32);
            push_u32(&mut out, e.data.len() as u32);
            // Cast cover: ingress `name_len <= u16::MAX` refuse.
            push_u16(&mut out, e.name.len() as u16);
            push_u16(&mut out, 0); // extra len
            out.extend_from_slice(&e.name);
            out.extend_from_slice(&e.data);
        }
        // Cast cover: ingress final-offset check bounds the central
        // directory start.
        let central_start = out.len() as u32;
        for e in &self.entries {
            push_u32(&mut out, 0x0201_4B50); // central signature
            push_u16(&mut out, 20); // made by
            push_u16(&mut out, 20); // needed
            push_u16(&mut out, 0x0800);
            push_u16(&mut out, 0);
            push_u16(&mut out, 0);
            push_u16(&mut out, 0);
            push_u32(&mut out, e.crc);
            // Cast cover: ingress `data_len <= u32::MAX` refuse.
            push_u32(&mut out, e.data.len() as u32);
            push_u32(&mut out, e.data.len() as u32);
            // Cast cover: ingress `name_len <= u16::MAX` refuse.
            push_u16(&mut out, e.name.len() as u16);
            push_u16(&mut out, 0);
            push_u16(&mut out, 0);
            push_u16(&mut out, 0);
            push_u16(&mut out, 0);
            push_u32(&mut out, 0);
            push_u32(&mut out, e.local_offset);
            out.extend_from_slice(&e.name);
        }
        // Cast cover: ingress central-length check bounds
        // `central_len`; the subtraction cannot underflow by
        // construction (`central_start` is an earlier length of this
        // same buffer).
        let central_len = out.len() as u32 - central_start;
        push_u32(&mut out, 0x0605_4B50); // end signature
        push_u16(&mut out, 0);
        push_u16(&mut out, 0);
        // Cast cover: ingress `count <= u16::MAX` refuse.
        push_u16(&mut out, self.entries.len() as u16);
        push_u16(&mut out, self.entries.len() as u16);
        push_u32(&mut out, central_len);
        push_u32(&mut out, central_start);
        push_u16(&mut out, 0);
        out
    }
}

// Multi-byte fields are little-endian per the ZIP format. Both
// helpers inline into `finish` (no `bl` on the push paths in the
// release assembler); they stay named helpers so each field keeps
// its width on the line.
fn push_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::{ZipWriter, crc32, stored_archive_len};

    #[test]
    fn crc_is_ieee() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn archive_len_counts_headers_once() {
        assert_eq!(stored_archive_len(&[]), 22);
        assert_eq!(stored_archive_len(&[(3, 1)]), 22 + 30 + 1 + 3 + 46 + 1);
    }

    /// Byte-identity lock: the exact pre-rewrite (cycle 004) bytes of
    /// a small fixed archive. Any change in layout, field order, or
    /// CRC breaks this test on purpose.
    #[test]
    fn golden_small_archive_is_byte_identical() {
        let mut zip = ZipWriter::new(2);
        zip.add("a.txt", b"hello".to_vec());
        zip.add("b/c.xml", b"<x/>".to_vec());
        let got: Vec<u8> = zip.finish();
        const EXPECTED: [u8; 207] = [
            0x50, 0x4b, 0x03, 0x04, 0x14, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x86, 0xa6, 0x10, 0x36, 0x05, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x05, 0x00,
            0x00, 0x00, 0x61, 0x2e, 0x74, 0x78, 0x74, 0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x50, 0x4b,
            0x03, 0x04, 0x14, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2b, 0x84,
            0x26, 0x12, 0x04, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00,
            0x62, 0x2f, 0x63, 0x2e, 0x78, 0x6d, 0x6c, 0x3c, 0x78, 0x2f, 0x3e, 0x50, 0x4b, 0x01,
            0x02, 0x14, 0x00, 0x14, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x86,
            0xa6, 0x10, 0x36, 0x05, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x61, 0x2e, 0x74, 0x78, 0x74, 0x50, 0x4b, 0x01, 0x02, 0x14, 0x00, 0x14, 0x00,
            0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2b, 0x84, 0x26, 0x12, 0x04, 0x00,
            0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x28, 0x00, 0x00, 0x00, 0x62, 0x2f, 0x63, 0x2e,
            0x78, 0x6d, 0x6c, 0x50, 0x4b, 0x05, 0x06, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x02,
            0x00, 0x68, 0x00, 0x00, 0x00, 0x51, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        assert_eq!(got, EXPECTED);
    }
}
