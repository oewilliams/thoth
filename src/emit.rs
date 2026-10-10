//! Typed cell emitters: one monomorphic loop per kind, no per-cell
//! branch on type. Nulls (`i64::MIN`, NaN) emit a blank `<c/>` —
//! absence, not a number.

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

/// Format one f64 for Excel: shortest round-trip text. NaN is handled by
/// the caller (blank cell); infinities are refused there, never written.
pub fn format_f64(v: f64) -> String {
    // `to_string` is `ryu`-shortest in current Rust; the shortest
    // correct text is the contract, the backend is an implementation pick.
    v.to_string()
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

/// One numeric i64 cell at (col, row1). Null sentinel -> blank cell.
pub fn i64_cell(out: &mut String, col_letters: &str, row1: usize, v: i64) {
    if v == NULL_I64 {
        out.push_str("<c r=\"");
        out.push_str(col_letters);
        out.push_str(&row1.to_string());
        out.push_str("\"/>");
    } else {
        out.push_str("<c r=\"");
        out.push_str(col_letters);
        out.push_str(&row1.to_string());
        out.push_str("\"><v>");
        out.push_str(&v.to_string());
        out.push_str("</v></c>");
    }
}

/// One numeric f64 cell. NaN -> blank; infinite -> caller refuses.
pub fn f64_cell(out: &mut String, col_letters: &str, row1: usize, v: f64) {
    if v.is_nan() {
        out.push_str("<c r=\"");
        out.push_str(col_letters);
        out.push_str(&row1.to_string());
        out.push_str("\"/>");
    } else {
        out.push_str("<c r=\"");
        out.push_str(col_letters);
        out.push_str(&row1.to_string());
        out.push_str("\"><v>");
        out.push_str(&format_f64(v));
        out.push_str("</v></c>");
    }
}

#[cfg(test)]
mod tests {
    use super::{NULL_I64, col_letters, f64_cell, i64_cell};

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
        i64_cell(&mut s, "A", 2, NULL_I64);
        assert_eq!(s, "<c r=\"A2\"/>");
        s.clear();
        f64_cell(&mut s, "B", 2, f64::NAN);
        assert_eq!(s, "<c r=\"B2\"/>");
    }
}
