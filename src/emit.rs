//! Typed cell emitters: one monomorphic loop per kind, no per-cell
//! branch on type. Nulls (`i64::MIN`, NaN) emit a blank `<c/>` —
//! absence, not a number.

use std::fmt::Write as _;

use crate::contract::NULL_I64;

/// Excel column letters: 0 -> A, 25 -> Z, 26 -> AA.
pub fn col_letters(mut n: usize) -> String {
    let mut s = Vec::new();
    loop {
        s.push(b'A' + (n % 26) as u8);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    s.iter().rev().map(|&b| b as char).collect()
}

/// Minimal XML escape for header text.
pub fn escape_into(out: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
}

/// Header cell: bold (`s="1"`) inline string.
pub fn header_cell(out: &mut String, col: usize, name: &str) {
    let letters = col_letters(col);
    out.push_str("<c r=\"");
    out.push_str(&letters);
    out.push_str("1\" t=\"inlineStr\" s=\"1\"><is><t>");
    escape_into(out, name);
    out.push_str("</t></is></c>");
}

/// Decimal digits of one integer, appended without allocating: the
/// same bytes as `to_string` (`int_spelling_matches`).
#[inline]
pub fn push_i64(out: &mut String, v: i64) {
    if v < 0 {
        out.push('-');
    }
    push_magnitude(out, v.unsigned_abs());
}

/// Decimal digits of one `usize`, appended without allocating.
#[inline]
pub fn push_usize(out: &mut String, v: usize) {
    push_magnitude(out, v as u64);
}

/// Two-digit table: entry `k` spells `k` zero-padded. Built at
/// compile time, so this runs zero times and ships as rodata.
const DEC_PAIRS: [u8; 200] = {
    let mut table: [u8; 200] = [0; 200];
    let mut k: usize = 0;
    while k < 100 {
        table[2 * k] = b'0' + (k / 10) as u8;
        table[2 * k + 1] = b'0' + (k % 10) as u8;
        k += 1;
    }
    table
};

#[inline]
fn push_magnitude(out: &mut String, mut mag: u64) {
    let mut buf: [u8; 20] = [0; 20];
    let mut i: usize = 20;
    while mag >= 100 {
        let pair: usize = (mag % 100) as usize;
        mag /= 100;
        i -= 2;
        buf[i] = DEC_PAIRS[2 * pair];
        buf[i + 1] = DEC_PAIRS[2 * pair + 1];
    }
    if mag >= 10 {
        let pair: usize = mag as usize;
        i -= 2;
        buf[i] = DEC_PAIRS[2 * pair];
        buf[i + 1] = DEC_PAIRS[2 * pair + 1];
    } else {
        i -= 1;
        buf[i] = b'0' + mag as u8;
    }
    // Digits only, pushed as chars: valid UTF-8 by construction.
    for &b in &buf[i..] {
        out.push(b as char);
    }
}

/// Shared cell-tag skeleton (open, blank-close, value-wrap); values and null tests stay per-kind.
#[inline]
fn cell_open(out: &mut String, col_letters: &str, row: &str) {
    out.push_str("<c r=\"");
    out.push_str(col_letters);
    out.push_str(row);
}

#[inline]
fn blank_close(out: &mut String) {
    out.push_str("\"/>");
}

#[inline]
fn value_open(out: &mut String) {
    out.push_str("\"><v>");
}

#[inline]
fn value_close(out: &mut String) {
    out.push_str("</v></c>");
}

/// One numeric i64 cell at (col, row). Null sentinel -> blank cell.
/// `row` is the row digits, formatted once and shared across the
/// row's cells.
#[inline]
pub fn i64_cell(out: &mut String, col_letters: &str, row: &str, v: i64) {
    if v == NULL_I64 {
        cell_open(out, col_letters, row);
        blank_close(out);
    } else {
        cell_open(out, col_letters, row);
        value_open(out);
        push_i64(out, v);
        value_close(out);
    }
}

/// One numeric f64 cell. NaN -> blank; infinite -> caller refuses.
/// `row` is the row digits, formatted once and shared across the
/// row's cells.
#[inline]
pub fn f64_cell(out: &mut String, col_letters: &str, row: &str, v: f64) {
    if v.is_nan() {
        cell_open(out, col_letters, row);
        blank_close(out);
    } else {
        cell_open(out, col_letters, row);
        value_open(out);
        if write!(out, "{v}").is_err() {
            // `String` never fails `fmt::Write`: unreachable by construction.
            debug_assert!(false);
        }
        value_close(out);
    }
}

/// One bool cell. `None` -> blank; `Some` spells `t="b"` with
/// `1`/`0`. `row` is the row digits, shared across the row's cells.
pub fn bool_cell(out: &mut String, col_letters: &str, row: &str, v: Option<bool>) {
    match v {
        None => {
            cell_open(out, col_letters, row);
            blank_close(out);
        }
        Some(b) => {
            cell_open(out, col_letters, row);
            out.push_str("\" t=\"b\"><v>");
            out.push(if b { '1' } else { '0' });
            value_close(out);
        }
    }
}

/// Whether inline text needs `xml:space="preserve"` on its `<t>`
/// element: Excel trims leading/trailing whitespace without it.
fn needs_preserve(text: &str) -> bool {
    text.chars().next().is_some_and(|c| c.is_whitespace())
        || text.chars().last().is_some_and(|c| c.is_whitespace())
}

/// One inline-string cell. `None` -> blank; `Some("")` is a real
/// empty string, spelled with empty `<t></t>`, never blank.
/// `row` is the row digits, shared across the row's cells.
pub fn str_cell(out: &mut String, col_letters: &str, row: &str, v: Option<&str>) {
    match v {
        None => {
            cell_open(out, col_letters, row);
            blank_close(out);
        }
        Some(text) => {
            cell_open(out, col_letters, row);
            out.push_str("\" t=\"inlineStr\"><is><t");
            if needs_preserve(text) {
                out.push_str(" xml:space=\"preserve\"");
            }
            out.push('>');
            escape_into(out, text);
            out.push_str("</t></is></c>");
        }
    }
}

/// `fmt::Write` over a byte buffer, so the `f64` spelling writes
/// straight into the arena with the same bytes as the `String`
/// oracle (`write!(out, "{v}")` in [`f64_cell`]).
struct VecFmt<'a>(&'a mut Vec<u8>);

impl std::fmt::Write for VecFmt<'_> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.extend_from_slice(s.as_bytes());
        Ok(())
    }
}

/// Decimal digits of one integer into a byte buffer: the same
/// `DEC_PAIRS` spelling as [`push_i64`], one slice append.
#[inline]
pub(crate) fn push_i64_bytes(out: &mut Vec<u8>, v: i64) {
    if v < 0 {
        out.push(b'-');
    }
    push_magnitude_bytes(out, v.unsigned_abs());
}

/// Decimal digits of one `usize` into a byte buffer.
#[inline]
pub(crate) fn push_usize_bytes(out: &mut Vec<u8>, v: usize) {
    push_magnitude_bytes(out, v as u64);
}

#[inline]
fn push_magnitude_bytes(out: &mut Vec<u8>, mut mag: u64) {
    let mut buf: [u8; 20] = [0; 20];
    let mut i: usize = 20;
    while mag >= 100 {
        let pair: usize = (mag % 100) as usize;
        mag /= 100;
        i -= 2;
        buf[i] = DEC_PAIRS[2 * pair];
        buf[i + 1] = DEC_PAIRS[2 * pair + 1];
    }
    if mag >= 10 {
        let pair: usize = mag as usize;
        i -= 2;
        buf[i] = DEC_PAIRS[2 * pair];
        buf[i + 1] = DEC_PAIRS[2 * pair + 1];
    } else {
        i -= 1;
        buf[i] = b'0' + mag as u8;
    }
    out.extend_from_slice(&buf[i..]);
}

/// Header-text escape into a byte buffer: the same five spellings
/// as [`escape_into`], fragments as byte constants.
pub(crate) fn escape_into_bytes(out: &mut Vec<u8>, text: &str) {
    for c in text.chars() {
        match c {
            '&' => out.extend_from_slice(b"&amp;"),
            '<' => out.extend_from_slice(b"&lt;"),
            '>' => out.extend_from_slice(b"&gt;"),
            '"' => out.extend_from_slice(b"&quot;"),
            '\'' => out.extend_from_slice(b"&apos;"),
            _ => {
                let mut buf: [u8; 4] = [0; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
        }
    }
}

/// Header cell into a byte buffer: byte-identical to
/// [`header_cell`].
pub(crate) fn header_cell_bytes(out: &mut Vec<u8>, col: usize, name: &str) {
    let letters: String = col_letters(col);
    out.extend_from_slice(b"<c r=\"");
    out.extend_from_slice(letters.as_bytes());
    out.extend_from_slice(b"1\" t=\"inlineStr\" s=\"1\"><is><t>");
    escape_into_bytes(out, name);
    out.extend_from_slice(b"</t></is></c>");
}

/// One numeric i64 cell into a byte buffer: byte-identical to
/// [`i64_cell`], with the oracle's append shape (open, digits or
/// blank close, close). `row` is the row digits as bytes,
/// formatted once and shared across the row's cells. A
/// precomputed per-column prefix spelling was measured slower
/// (runtime-length appends lose to constant-length ones), so the
/// open stays two constant appends plus the letters.
#[inline]
pub(crate) fn i64_cell_bytes(out: &mut Vec<u8>, col_letters: &str, row: &[u8], v: i64) {
    out.extend_from_slice(b"<c r=\"");
    out.extend_from_slice(col_letters.as_bytes());
    out.extend_from_slice(row);
    if v == NULL_I64 {
        out.extend_from_slice(b"\"/>");
    } else {
        out.extend_from_slice(b"\"><v>");
        push_i64_bytes(out, v);
        out.extend_from_slice(b"</v></c>");
    }
}

/// One numeric f64 cell into a byte buffer: byte-identical to
/// [`f64_cell`] (NaN blank, the `Display` spelling write-through).
#[inline]
pub(crate) fn f64_cell_bytes(out: &mut Vec<u8>, col_letters: &str, row: &[u8], v: f64) {
    out.extend_from_slice(b"<c r=\"");
    out.extend_from_slice(col_letters.as_bytes());
    out.extend_from_slice(row);
    if v.is_nan() {
        out.extend_from_slice(b"\"/>");
    } else {
        out.extend_from_slice(b"\"><v>");
        if write!(VecFmt(out), "{v}").is_err() {
            // `Vec<u8>` never fails `fmt::Write`: unreachable by construction.
            debug_assert!(false);
        }
        out.extend_from_slice(b"</v></c>");
    }
}

/// One bool cell into a byte buffer: byte-identical to
/// [`bool_cell`].
pub(crate) fn bool_cell_bytes(out: &mut Vec<u8>, col_letters: &str, row: &[u8], v: Option<bool>) {
    out.extend_from_slice(b"<c r=\"");
    out.extend_from_slice(col_letters.as_bytes());
    out.extend_from_slice(row);
    match v {
        None => out.extend_from_slice(b"\"/>"),
        Some(b) => {
            out.extend_from_slice(b"\" t=\"b\"><v>");
            out.push(if b { b'1' } else { b'0' });
            out.extend_from_slice(b"</v></c>");
        }
    }
}

/// One inline-string cell into a byte buffer: byte-identical to
/// [`str_cell`].
pub(crate) fn str_cell_bytes(out: &mut Vec<u8>, col_letters: &str, row: &[u8], v: Option<&str>) {
    out.extend_from_slice(b"<c r=\"");
    out.extend_from_slice(col_letters.as_bytes());
    out.extend_from_slice(row);
    match v {
        None => out.extend_from_slice(b"\"/>"),
        Some(text) => {
            out.extend_from_slice(b"\" t=\"inlineStr\"><is><t");
            if needs_preserve(text) {
                out.extend_from_slice(b" xml:space=\"preserve\"");
            }
            out.push(b'>');
            escape_into_bytes(out, text);
            out.extend_from_slice(b"</t></is></c>");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NULL_I64, bool_cell, bool_cell_bytes, col_letters, f64_cell, f64_cell_bytes, header_cell,
        header_cell_bytes, i64_cell, i64_cell_bytes, push_i64, push_i64_bytes, str_cell,
        str_cell_bytes,
    };

    #[test]
    fn letters_roll_over_z() {
        assert_eq!(col_letters(0), "A");
        assert_eq!(col_letters(25), "Z");
        assert_eq!(col_letters(26), "AA");
        assert_eq!(col_letters(27), "AB");
    }

    #[test]
    fn null_is_blank() {
        let mut s = String::new();
        i64_cell(&mut s, "A", "2", NULL_I64);
        assert_eq!(s, "<c r=\"A2\"/>");
        s.clear();
        f64_cell(&mut s, "B", "2", f64::NAN);
        assert_eq!(s, "<c r=\"B2\"/>");
    }

    /// Direct integer spelling matches `to_string` exactly: a full
    /// sweep around zero plus every decimal-width extreme.
    #[test]
    fn int_spelling_matches() {
        let mut buf = String::new();
        let mut check = |v: i64| {
            buf.clear();
            push_i64(&mut buf, v);
            assert_eq!(buf, v.to_string(), "{v}");
        };
        for v in -100_000i64..100_000i64 {
            check(v);
        }
        for v in [
            i64::MAX,
            i64::MIN + 1,
            999_999_999_999_999_999,
            -999_999_999_999_999_999,
            1_000_000_000_000_000_000,
            -1_000_000_000_000_000_000,
            10,
            100,
            1_000_000_000,
        ] {
            check(v);
        }
    }

    /// Byte sink equals the `String` oracle, cell for cell: integers
    /// over the direct-spelling sweep, floats over specials plus a
    /// quarter-step sweep, headers with escapable names.
    #[test]
    fn byte_sink_matches_string_oracle() {
        let mut oracle = String::new();
        let mut bytes: Vec<u8> = Vec::new();
        for v in [-100_000i64, -1, 0, 1, 99, 100, 100_000, i64::MAX, NULL_I64] {
            oracle.clear();
            bytes.clear();
            i64_cell(&mut oracle, "A", "2", v);
            i64_cell_bytes(&mut bytes, "A", b"2", v);
            assert_eq!(bytes, oracle.as_bytes(), "i64 {v}");
            oracle.clear();
            bytes.clear();
            push_i64(&mut oracle, v);
            push_i64_bytes(&mut bytes, v);
            assert_eq!(bytes, oracle.as_bytes(), "digits {v}");
        }
        let mut floats: Vec<f64> = vec![
            0.0,
            -0.0,
            1.0,
            -1.5,
            0.5,
            1e300,
            -1e300,
            5e-324,
            -5e-324,
            f64::MAX,
            -f64::MAX,
            f64::MIN_POSITIVE,
            f64::NAN,
            // 301-digit positional extremes and the subnormal floor:
            // the widest `Display` spellings, pinned byte for byte.
            1e308,
            -1e308,
            1.5e300,
            2.225073858507201e-308,
            1e-310,
            123_456_789.123_456_79,
        ];
        for i in -1000i64..1000 {
            floats.push(i as f64 * 0.25);
        }
        for v in floats {
            oracle.clear();
            bytes.clear();
            f64_cell(&mut oracle, "B", "7", v);
            f64_cell_bytes(&mut bytes, "B", b"7", v);
            assert_eq!(bytes, oracle.as_bytes(), "f64 {v}");
        }
        for (col, name) in [(0usize, "t"), (27, "a & <b> \"q\"")] {
            oracle.clear();
            bytes.clear();
            header_cell(&mut oracle, col, name);
            header_cell_bytes(&mut bytes, col, name);
            assert_eq!(bytes, oracle.as_bytes(), "header {name}");
        }
        for v in [Some(true), Some(false), None] {
            oracle.clear();
            bytes.clear();
            bool_cell(&mut oracle, "C", "3", v);
            bool_cell_bytes(&mut bytes, "C", b"3", v);
            assert_eq!(bytes, oracle.as_bytes(), "bool {v:?}");
        }
        assert_eq!(
            {
                let mut s = String::new();
                bool_cell(&mut s, "C", "3", Some(true));
                s
            },
            "<c r=\"C3\" t=\"b\"><v>1</v></c>"
        );
        for v in [
            Some("plain"),
            Some("a & <b> \"q\" 'x'"),
            Some(" lead"),
            Some("trail "),
            Some(" both "),
            Some("\ttab\n"),
            Some(""),
            None,
        ] {
            oracle.clear();
            bytes.clear();
            str_cell(&mut oracle, "D", "4", v);
            str_cell_bytes(&mut bytes, "D", b"4", v);
            assert_eq!(bytes, oracle.as_bytes(), "str {v:?}");
        }
        // Empty string is a real value, distinct from blank.
        let mut empty = String::new();
        str_cell(&mut empty, "D", "4", Some(""));
        assert_eq!(empty, "<c r=\"D4\" t=\"inlineStr\"><is><t></t></is></c>");
        let mut blank = String::new();
        str_cell(&mut blank, "D", "4", None);
        assert_eq!(blank, "<c r=\"D4\"/>");
        assert_ne!(empty, blank);
        let mut preserved = String::new();
        str_cell(&mut preserved, "D", "4", Some(" x"));
        assert_eq!(
            preserved,
            "<c r=\"D4\" t=\"inlineStr\"><is><t xml:space=\"preserve\"> x</t></is></c>"
        );
    }
}
