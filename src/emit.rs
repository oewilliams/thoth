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

/// One numeric i64 cell at (col, row). Null sentinel -> blank cell.
/// `row` is the row digits, formatted once and shared across the
/// row's cells.
#[inline]
pub fn i64_cell(out: &mut String, col_letters: &str, row: &str, v: i64) {
    if v == NULL_I64 {
        out.push_str("<c r=\"");
        out.push_str(col_letters);
        out.push_str(row);
        out.push_str("\"/>");
    } else {
        out.push_str("<c r=\"");
        out.push_str(col_letters);
        out.push_str(row);
        out.push_str("\"><v>");
        push_i64(out, v);
        out.push_str("</v></c>");
    }
}

/// One numeric f64 cell. NaN -> blank; infinite -> caller refuses.
/// `row` is the row digits, formatted once and shared across the
/// row's cells.
#[inline]
pub fn f64_cell(out: &mut String, col_letters: &str, row: &str, v: f64) {
    if v.is_nan() {
        out.push_str("<c r=\"");
        out.push_str(col_letters);
        out.push_str(row);
        out.push_str("\"/>");
    } else {
        out.push_str("<c r=\"");
        out.push_str(col_letters);
        out.push_str(row);
        out.push_str("\"><v>");
        if write!(out, "{v}").is_err() {
            // `String` never fails `fmt::Write`: unreachable by construction.
            debug_assert!(false);
        }
        out.push_str("</v></c>");
    }
}

#[cfg(test)]
mod tests {
    use super::{NULL_I64, col_letters, f64_cell, i64_cell, push_i64};

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
}
