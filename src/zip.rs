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
    use super::crc32;

    #[test]
    fn crc_is_ieee() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }
}
