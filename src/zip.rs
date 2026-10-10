//! Minimal ZIP writer, stored (no compression) only.
//!
//! Compression is a separate act, not this module's: v0 establishes the
//! floor — `memcpy` plus formatting — and deflate enters later behind a
//! flag, measured against this spelling as oracle. Stored entries keep the
//! writer dependency-free and the bytes deterministic.

/// CRC-32 (IEEE) over one buffer, bitwise: tables enter when a caller
/// measures them faster, not before.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Projected length, in bytes, of the finished stored archive for the
/// given entry sizes: each entry contributes a 30-byte local header plus
/// its name and data, a 46-byte central record plus its name, and the
/// archive closes with the 22-byte end record. Each tuple is
/// `(data_len, name_len)` in bytes. Pure size arithmetic in `u64`: the
/// parts already live in memory, so no caller can overflow it.
pub(crate) fn stored_archive_len(entries: &[(usize, usize)]) -> u64 {
    let mut total: u64 = 22;
    for (data_len, name_len) in entries.iter().copied() {
        let data_len: u64 = data_len as u64;
        let name_len: u64 = name_len as u64;
        total += 30 + name_len + data_len;
        total += 46 + name_len;
    }
    total
}

/// Whether an archive with the given entry sizes fits the 32-bit zip
/// fields without zip64: every part length, every local header offset,
/// and the central directory offset and length must each fit `u32`,
/// and the entry count and every name length must each fit `u16`.
/// Each tuple is `(data_len, name_len)` in bytes, as in
/// `stored_archive_len`. Pure size arithmetic, no allocation: the build
/// path calls it before assembly, so no narrowing cast in
/// `ZipWriter::finish` below ever truncates.
///
/// A fitting total alone would imply the `u32` fields fit (each is
/// bounded by it), but the count and name checks have no such cover:
/// thousands of tiny entries, or one long name, overflow their `u16`
/// fields under a small total. Every category is checked explicitly so
/// the reviewer can match each check to its cast.
pub(crate) fn stored_archive_fits(entries: &[(usize, usize)]) -> bool {
    const U32_MAX: u64 = u32::MAX as u64;
    const U16_MAX: u64 = u16::MAX as u64;
    if entries.len() as u64 > U16_MAX {
        return false;
    }
    let mut offset: u64 = 0;
    let mut central_len: u64 = 0;
    for (data_len, name_len) in entries.iter().copied() {
        let data_len: u64 = data_len as u64;
        let name_len: u64 = name_len as u64;
        if data_len > U32_MAX || name_len > U16_MAX {
            return false;
        }
        if offset > U32_MAX {
            return false;
        }
        offset += 30 + name_len + data_len;
        central_len += 46 + name_len;
    }
    offset <= U32_MAX && central_len <= U32_MAX
}

struct Entry {
    name: Vec<u8>,
    data: Vec<u8>,
    crc: u32,
    local_offset: u32,
}

/// Stored-only ZIP archive, built in memory then written once.
#[derive(Default)]
pub struct ZipWriter {
    entries: Vec<Entry>,
}

impl ZipWriter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Stage one file. The name must be UTF-8 without a leading slash;
    /// duplicates are refused, never silently replaced.
    pub fn add(&mut self, name: &str, data: Vec<u8>) {
        assert!(!name.starts_with('/'), "zip name must be relative: {name}");
        assert!(
            !self.entries.iter().any(|e| e.name == name.as_bytes()),
            "duplicate zip entry: {name}"
        );
        let crc = crc32(&data);
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
        let mut total: usize = 22; // end record
        for e in &self.entries {
            total += 30 + e.name.len() + e.data.len(); // local header + body
            total += 46 + e.name.len(); // central record
        }
        let mut out = Vec::with_capacity(total);
        for e in self.entries.iter_mut() {
            e.local_offset = out.len() as u32;
            push_u32(&mut out, 0x0403_4B50); // local signature
            push_u16(&mut out, 20); // version needed
            push_u16(&mut out, 0x0800); // UTF-8 flag
            push_u16(&mut out, 0); // method: stored
            push_u16(&mut out, 0); // mod time
            push_u16(&mut out, 0); // mod date
            push_u32(&mut out, e.crc);
            push_u32(&mut out, e.data.len() as u32);
            push_u32(&mut out, e.data.len() as u32);
            push_u16(&mut out, e.name.len() as u16);
            push_u16(&mut out, 0); // extra len
            out.extend_from_slice(&e.name);
            out.extend_from_slice(&e.data);
        }
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
            push_u32(&mut out, e.data.len() as u32);
            push_u32(&mut out, e.data.len() as u32);
            push_u16(&mut out, e.name.len() as u16);
            push_u16(&mut out, 0);
            push_u16(&mut out, 0);
            push_u16(&mut out, 0);
            push_u16(&mut out, 0);
            push_u32(&mut out, 0);
            push_u32(&mut out, e.local_offset);
            out.extend_from_slice(&e.name);
        }
        let central_len = out.len() as u32 - central_start;
        push_u32(&mut out, 0x0605_4B50); // end signature
        push_u16(&mut out, 0);
        push_u16(&mut out, 0);
        push_u16(&mut out, self.entries.len() as u16);
        push_u16(&mut out, self.entries.len() as u16);
        push_u32(&mut out, central_len);
        push_u32(&mut out, central_start);
        push_u16(&mut out, 0);
        out
    }
}

// Multi-byte fields are little-endian per the ZIP format.
fn push_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::{crc32, stored_archive_fits, stored_archive_len};

    #[test]
    fn crc_is_ieee() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn archive_len_counts_headers_once() {
        assert_eq!(stored_archive_len(&[]), 22);
        assert_eq!(stored_archive_len(&[(3, 1)]), 22 + 30 + 1 + 3 + 46 + 1);
    }

    #[test]
    fn archive_fits_flips_at_the_u32_ceiling() {
        let max: usize = u32::MAX as usize;
        // One entry with an empty name carries 98 bytes of headers
        // (30 local + 46 central + 22 end), so data fills the last
        // offset exactly at `max - 30` and one byte more refuses.
        // Only sizes are decided here; no bytes are allocated.
        assert!(stored_archive_fits(&[(max - 30, 0)]));
        assert!(!stored_archive_fits(&[(max - 29, 0)]));
        assert!(!stored_archive_fits(&[(max - 1, 0)]));
        assert!(!stored_archive_fits(&[(max, 0)]));
        assert!(!stored_archive_fits(&[(max + 1, 0)]));
    }

    #[test]
    fn archive_fits_accumulates_offsets_across_entries() {
        let max: usize = u32::MAX as usize;
        // Two individually small parts whose running offset still
        // overflows the central directory start.
        assert!(!stored_archive_fits(&[(max - 30, 0), (1, 0)]));
        assert!(stored_archive_fits(&[(8, 2), (300, 5)]));
    }

    #[test]
    fn archive_fits_bounds_name_lengths_at_u16() {
        assert!(stored_archive_fits(&[(0, u16::MAX as usize)]));
        assert!(!stored_archive_fits(&[(0, u16::MAX as usize + 1)]));
    }

    #[test]
    fn archive_fits_bounds_entry_count_at_u16() {
        let over: Vec<(usize, usize)> = vec![(0, 0); u16::MAX as usize + 1];
        assert!(!stored_archive_fits(&over));
        let at_max: Vec<(usize, usize)> = vec![(0, 0); u16::MAX as usize];
        assert!(stored_archive_fits(&at_max));
    }

    #[test]
    fn archive_fits_bounds_central_directory_length() {
        // Every `u16` field fits, but the central directory itself
        // passes the `u32` ceiling, so the archive refuses.
        let entries: Vec<(usize, usize)> = vec![(0, u16::MAX as usize); u16::MAX as usize];
        assert!(!stored_archive_fits(&entries));
    }
}
