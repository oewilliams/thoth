//! Minimal stored-ZIP machinery, internal to the crate.
//!
//! Compression is a separate act, not this module's: v0 establishes the
//! floor — `memcpy` plus formatting — and deflate enters later behind a
//! flag, measured against this spelling as oracle. Stored entries keep
//! the writer dependency-free and the bytes deterministic.
//!
//! This module is plumbing, not a user surface: the arena builder in
//! [`crate::xlsx::build_workbook`] is the single archive spelling. It
//! reserves each local header in the output buffer, the part's bytes
//! are emitted directly after it, and the CRC and stored sizes are
//! patched back into the header once the part's length is known. This
//! module owns only the format primitives (header, central, and end
//! records), the CRC spellings, and the projected archive length.
//!
//! This module performs no validation: names, count, and every size
//! already passed the ingress check in [`crate::xlsx::build_workbook`],
//! which owns the entry set. Every narrowing cast below names its
//! covering ingress check on the line.
//!
//! Cycle-004 assembler notes (release profile, aarch64): `crc32`
//! is a 4-insn `crc32x` word loop plus an unrolled `crc32b` tail,
//! with no calls and no table traffic.

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

/// Working CRC state before the first chunk: the caller folds each
/// chunk with [`crc32_update`] and applies [`crc32_finish`] once.
pub(crate) const CRC_INIT: u32 = 0xFFFF_FFFF;

/// Fold one chunk into a working CRC state (see [`CRC_INIT`]).
/// Any split of a buffer folds to the one-shot result: chunked
/// update is concatenation, pinned by `crc_update_any_split`.
#[cfg(all(target_arch = "aarch64", target_feature = "crc"))]
pub(crate) fn crc32_update(mut crc: u32, data: &[u8]) -> u32 {
    let (words, tail) = data.as_chunks::<8>();
    for word in words {
        crc = unsafe { core::arch::aarch64::__crc32d(crc, u64::from_le_bytes(*word)) };
    }
    for &byte in tail {
        crc = unsafe { core::arch::aarch64::__crc32b(crc, byte) };
    }
    crc
}

/// Fold one chunk into a working CRC state: the portable spelling
/// for every target without the ARMv8 checksum feature.
#[cfg(not(all(target_arch = "aarch64", target_feature = "crc")))]
pub(crate) fn crc32_update(mut crc: u32, data: &[u8]) -> u32 {
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
    crc
}

/// Finish a working CRC state: the final complement, applied once.
pub(crate) fn crc32_finish(crc: u32) -> u32 {
    !crc
}

/// CRC-32 (IEEE) over one buffer: the ARMv8 checksum instruction,
/// eight bytes per instruction plus a byte tail, where the target
/// guarantees the feature at compile time; the slicing-by-8 table
/// spelling covers every other target. No runtime detection
/// anywhere on this path. One-shot spelling of the chunked update
/// above: `crc32_finish(crc32_update(CRC_INIT, data))`.
///
/// Head-to-head on this machine (Apple silicon, bench profile):
/// bitwise 5.8 ns/B, single table 1.7, slicing-by-8 0.37, the
/// instruction spelling 0.083 — picked by numbers, not by assumption.
pub(crate) fn crc32(data: &[u8]) -> u32 {
    crc32_finish(crc32_update(CRC_INIT, data))
}

/// One entry's footprint in the finished archive: a 30-byte local
/// header plus its name and data, and a 46-byte central record plus
/// its name. Single spelling, shared by [`stored_archive_len`] and
/// the arena reservation in [`crate::xlsx::build_workbook`] so the
/// two cannot diverge.
pub(crate) fn entry_sizes(data_len: usize, name_len: usize) -> u64 {
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

// Multi-byte fields are little-endian per the ZIP format. Both
// helpers stay named so each field keeps its width on the line.
pub(crate) fn push_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

pub(crate) fn push_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// Reserve one entry's 30-byte local header plus its name at the
/// end of `out` and return the header's offset. The CRC and the
/// two stored sizes are written as zeros here and patched by
/// [`patch_local_header`] once the part has been emitted after the
/// header; every other field is final.
pub(crate) fn begin_entry(out: &mut Vec<u8>, name: &[u8]) -> usize {
    let offset: usize = out.len();
    push_u32(out, 0x0403_4B50); // local signature
    push_u16(out, 20); // version needed
    push_u16(out, 0x0800); // UTF-8 flag
    push_u16(out, 0); // method: stored
    push_u16(out, 0); // mod time
    push_u16(out, 0); // mod date
    push_u32(out, 0); // crc, patched
    push_u32(out, 0); // compressed size, patched
    push_u32(out, 0); // uncompressed size, patched
    // Cast cover: ingress `name_len <= u16::MAX` refuse in
    // `crate::xlsx::archive_fits`.
    push_u16(out, name.len() as u16);
    push_u16(out, 0); // extra len
    out.extend_from_slice(name);
    offset
}

/// Patch the CRC and the equal stored sizes into the local header
/// at `offset` (header offsets +14, +18, +22). The caller has
/// already narrowed `len` under its covering ingress check.
pub(crate) fn patch_local_header(out: &mut [u8], offset: usize, crc: u32, len: u32) {
    out[offset + 14..offset + 18].copy_from_slice(&crc.to_le_bytes());
    out[offset + 18..offset + 22].copy_from_slice(&len.to_le_bytes());
    out[offset + 22..offset + 26].copy_from_slice(&len.to_le_bytes());
}

/// Append one central-directory record for an entry already
/// emitted. `len` and `local_offset` arrive narrowed under their
/// covering ingress checks in `crate::xlsx::archive_fits`.
pub(crate) fn write_central_record(
    out: &mut Vec<u8>,
    name: &[u8],
    crc: u32,
    len: u32,
    local_offset: u32,
) {
    push_u32(out, 0x0201_4B50); // central signature
    push_u16(out, 20); // made by
    push_u16(out, 20); // needed
    push_u16(out, 0x0800); // UTF-8 flag
    push_u16(out, 0); // method: stored
    push_u16(out, 0); // mod time
    push_u16(out, 0); // mod date
    push_u32(out, crc);
    push_u32(out, len);
    push_u32(out, len);
    // Cast cover: ingress `name_len <= u16::MAX` refuse in
    // `crate::xlsx::archive_fits`.
    push_u16(out, name.len() as u16);
    push_u16(out, 0); // extra len
    push_u16(out, 0); // comment len
    push_u16(out, 0); // disk start
    push_u16(out, 0); // internal attrs
    push_u32(out, 0); // external attrs
    push_u32(out, local_offset);
    out.extend_from_slice(name);
}

/// Append the end record. `central_len` and `central_start`
/// arrive narrowed under their covering ingress checks.
pub(crate) fn write_end_record(
    out: &mut Vec<u8>,
    count: usize,
    central_len: u32,
    central_start: u32,
) {
    push_u32(out, 0x0605_4B50); // end signature
    push_u16(out, 0); // disk
    push_u16(out, 0); // central disk
    // Cast cover: ingress `count <= u16::MAX` refuse in
    // `crate::xlsx::archive_fits`.
    push_u16(out, count as u16);
    push_u16(out, count as u16);
    push_u32(out, central_len);
    push_u32(out, central_start);
    push_u16(out, 0); // comment len
}

#[cfg(test)]
mod tests {
    use super::{
        CRC_INIT, begin_entry, crc32, crc32_finish, crc32_update, patch_local_header,
        stored_archive_len, write_central_record, write_end_record,
    };

    #[test]
    fn crc_is_ieee() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    /// Chunked update is concatenation: every two-chunk split of
    /// the probe buffer, empty chunks at either end, and a
    /// byte-at-a-time fold all equal the one-shot result.
    #[test]
    fn crc_update_any_split_matches_one_shot() {
        let data: Vec<u8> = (0..1024u32).map(|i| (i * 31 + 7) as u8).collect();
        let want: u32 = crc32(&data);
        for split in 0..=data.len() {
            let state: u32 = crc32_update(CRC_INIT, &data[..split]);
            let state: u32 = crc32_update(state, &data[split..]);
            assert_eq!(crc32_finish(state), want, "split {split}");
        }
        let state: u32 = crc32_update(CRC_INIT, &[]);
        let state: u32 = crc32_update(state, &data);
        let state: u32 = crc32_update(state, &[]);
        assert_eq!(crc32_finish(state), want, "empty chunks");
        let mut state: u32 = CRC_INIT;
        for &byte in &data {
            state = crc32_update(state, &[byte]);
        }
        assert_eq!(crc32_finish(state), want, "byte at a time");
        assert_eq!(crc32_update(CRC_INIT, &[]), CRC_INIT);
    }

    #[test]
    fn archive_len_counts_headers_once() {
        assert_eq!(stored_archive_len(&[]), 22);
        assert_eq!(stored_archive_len(&[(3, 1)]), 22 + 30 + 1 + 3 + 46 + 1);
    }

    /// Byte-identity lock: the exact pre-rewrite (cycle 004) bytes of
    /// a small fixed archive, now spelled through the arena
    /// primitives (reserve header, emit, patch, central, end). Any
    /// change in layout, field order, or CRC breaks this test on
    /// purpose. The expected bytes are unchanged from the retired
    /// staged writer's lock.
    #[test]
    fn golden_small_archive_is_byte_identical() {
        let parts: [(&str, &[u8]); 2] = [("a.txt", b"hello"), ("b/c.xml", b"<x/>")];
        let mut out: Vec<u8> = Vec::new();
        let mut entries: Vec<(Vec<u8>, u32, usize, usize)> = Vec::new();
        for (name, data) in parts {
            let offset: usize = begin_entry(&mut out, name.as_bytes());
            let start: usize = out.len();
            out.extend_from_slice(data);
            let crc: u32 = crc32(&out[start..]);
            patch_local_header(&mut out, offset, crc, data.len() as u32);
            entries.push((name.as_bytes().to_vec(), crc, data.len(), offset));
        }
        let central_start: usize = out.len();
        for (name, crc, len, offset) in &entries {
            write_central_record(&mut out, name, *crc, *len as u32, *offset as u32);
        }
        let central_len: u32 = (out.len() - central_start) as u32;
        write_end_record(&mut out, entries.len(), central_len, central_start as u32);
        let got: Vec<u8> = out;
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
