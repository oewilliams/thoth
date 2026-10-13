//! Typed cell emitters: one monomorphic loop per kind, no per-cell
//! branch on type. Nulls (`i64::MIN`, NaN) emit a blank `<c/>` —
//! absence, not a number.
//!
//! STYLE rule 1 tripwire: this file passes 700 lines in cycle 018
//! because each kind keeps a plain body (verbatim, so the plain
//! path pays nothing measurable) beside an outlined styled body;
//! the duplication is the measured price of that split. Cycle 020
//! grew it further (993 lines) with the shared SWAR escape scan.

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

/// SWAR word constants: one `0x01` / high bit per byte lane.
const SWAR_ONES: u64 = 0x0101_0101_0101_0101;
const SWAR_HIGHS: u64 = 0x8080_8080_8080_8080;

/// Whether any byte lane of `word` equals `byte` (SWAR zero-byte
/// detection: xor to zero the matching lane, then the classic
/// `(x - ONES) & !x & HIGHS` test for a zero lane).
fn swar_has_byte(word: u64, byte: u8) -> bool {
    let x: u64 = word ^ (SWAR_ONES * u64::from(byte));
    (x.wrapping_sub(SWAR_ONES) & !x & SWAR_HIGHS) != 0
}

/// How many lanes of `word` equal `byte`, exactly: the count
/// uses a borrow-free zero-lane test (clear each lane's high
/// bit, add `0x7F` per lane — no carry can escape a lane —
/// then or-in the original high bit), so a lane is flagged
/// zero exactly when it is zero. The classic
/// `(x - ONES) & !x & HIGHS` mask is sound for existence
/// ([`swar_has_byte`]) but not for counting: a borrow out of
/// a true zero lane also flags a following lane holding
/// `target ^ 1`, double-counting adjacent pairs such as
/// `&'` for the first byte's target.
pub(crate) fn swar_count_byte(word: u64, byte: u8) -> usize {
    let x: u64 = word ^ (SWAR_ONES * u64::from(byte));
    let nonzero: u64 =
        ((x & 0x7F7F_7F7F_7F7F_7F7F).wrapping_add(0x7F7F_7F7F_7F7F_7F7F) | x) & SWAR_HIGHS;
    ((!nonzero & SWAR_HIGHS).count_ones()) as usize
}

/// Whether `word` holds one of the five escapable bytes
/// (`&`, `<`, `>`, `"`, `'` — all ASCII, so no lane of a
/// multi-byte UTF-8 sequence can false-positive: continuation
/// and lead bytes are all `>= 0x80`).
pub(crate) fn swar_has_escapable(word: u64) -> bool {
    swar_has_byte(word, b'&')
        || swar_has_byte(word, b'<')
        || swar_has_byte(word, b'>')
        || swar_has_byte(word, b'"')
        || swar_has_byte(word, b'\'')
}

/// The eight bytes starting at `i` as one little-endian word,
/// or `None` when fewer than eight remain.
pub(crate) fn swar_word(bytes: &[u8], i: usize) -> Option<u64> {
    let chunk: &[u8; 8] = bytes.get(i..)?.first_chunk::<8>()?;
    Some(u64::from_le_bytes(*chunk))
}

/// Whether `byte` is one of the five escapable bytes.
fn is_escapable(byte: u8) -> bool {
    matches!(byte, b'&' | b'<' | b'>' | b'"' | b'\'')
}

/// The entity spelling of one escapable byte.
fn escape_entity(byte: u8) -> &'static str {
    match byte {
        b'&' => "&amp;",
        b'<' => "&lt;",
        b'>' => "&gt;",
        b'"' => "&quot;",
        _ => "&apos;",
    }
}

/// Minimal XML escape for header text: SWAR scan over the byte
/// payload (eight bytes per word, safe Rust) skips clean words,
/// and clean spans land with one append each; only the bytes of
/// a word holding an escapable are walked one by one. A word is
/// skipped only when it is plain ASCII (in addition to holding
/// no escapable), so the skip's end is always a char boundary
/// and every span sliced from `text` is a valid `str` slice;
/// a word holding a multi-byte char is walked by whole chars.
pub fn escape_into(out: &mut String, text: &str) {
    let bytes: &[u8] = text.as_bytes();
    let mut start: usize = 0;
    let mut i: usize = 0;
    while i < bytes.len() {
        if let Some(word) = swar_word(bytes, i)
            && swar_word_is_plain_ascii(word)
            && !swar_has_escapable(word)
        {
            i += 8;
            continue;
        }
        // A word holding an escapable (or the short tail) is
        // walked exactly once, never retested: flush the
        // clean span before it, then one append per char
        // (spans inside a dirty word are short by
        // construction, so slice appends lose to char appends).
        out.push_str(&text[start..i]);
        let end: usize = usize::min(i + 8, bytes.len());
        while i < end {
            if bytes[i] < 0x80 {
                if is_escapable(bytes[i]) {
                    out.push_str(escape_entity(bytes[i]));
                } else {
                    out.push(char::from(bytes[i]));
                }
                i += 1;
            } else {
                let Some(ch) = text[i..].chars().next() else {
                    break;
                };
                out.push(ch);
                i += ch.len_utf8();
            }
        }
        start = i;
    }
    out.push_str(&text[start..]);
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

/// Two-digit table: entry `k` spells `k` zero-padded, packed as
/// one little-endian `u16` so a pair is one load and one store.
/// Built at compile time; ships as rodata.
const DEC_PAIRS: [u16; 100] = {
    let mut table: [u16; 100] = [0; 100];
    let mut k: usize = 0;
    while k < 100 {
        table[k] = u16::from_le_bytes([b'0' + (k / 10) as u8, b'0' + (k % 10) as u8]);
        k += 1;
    }
    table
};

/// The digits of `mag` at the tail of a 20-byte buffer, with the
/// start index. One spelling behind both magnitude sinks.
#[inline]
fn magnitude_digits(mut mag: u64) -> ([u8; 20], usize) {
    let mut buf: [u8; 20] = [0; 20];
    let mut i: usize = 20;
    while mag >= 100 {
        let pair: usize = (mag % 100) as usize;
        mag /= 100;
        i -= 2;
        buf[i..i + 2].copy_from_slice(&DEC_PAIRS[pair].to_le_bytes());
    }
    if mag >= 10 {
        i -= 2;
        buf[i..i + 2].copy_from_slice(&DEC_PAIRS[mag as usize].to_le_bytes());
    } else {
        i -= 1;
        buf[i] = b'0' + mag as u8;
    }
    (buf, i)
}

#[inline]
fn push_magnitude(out: &mut String, mag: u64) {
    let (buf, i) = magnitude_digits(mag);
    // Digits only, pushed as chars: valid UTF-8 by construction.
    for &b in &buf[i..] {
        out.push(b as char);
    }
}

/// Shared cell-tag skeleton (open, blank-close, value-wrap); values and null tests stay per-kind.
/// `style` is the column's pre-spelled fragment (` s="5"`, empty
/// for plain), placed after the `r` attribute and before any `t`.
#[inline]
fn cell_open(out: &mut String, col_letters: &str, row: &str) {
    out.push_str("<c r=\"");
    out.push_str(col_letters);
    out.push_str(row);
}

#[inline]
fn style_close(out: &mut String, style: &str) {
    out.push('"');
    out.push_str(style);
}

#[inline]
fn blank_close(out: &mut String) {
    out.push_str("\"/>");
}

#[inline]
fn blank_close_styled(out: &mut String) {
    out.push_str("/>");
}

#[inline]
fn value_open(out: &mut String) {
    out.push_str("\"><v>");
}

#[inline]
fn value_open_styled(out: &mut String) {
    out.push_str("><v>");
}

#[inline]
fn value_close(out: &mut String) {
    out.push_str("</v></c>");
}

/// One numeric i64 cell at (col, row). Null sentinel -> blank cell.
/// `row` is the row digits, formatted once and shared across the
/// row's cells.
#[inline]
pub fn i64_cell(out: &mut String, col_letters: &str, row: &str, style: &str, v: i64) {
    if !style.is_empty() {
        i64_cell_styled(out, col_letters, row, style, v);
        return;
    }
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

fn i64_cell_styled(out: &mut String, col_letters: &str, row: &str, style: &str, v: i64) {
    cell_open(out, col_letters, row);
    style_close(out, style);
    if v == NULL_I64 {
        blank_close_styled(out);
    } else {
        value_open_styled(out);
        push_i64(out, v);
        value_close(out);
    }
}

/// One numeric f64 cell. NaN -> blank; infinite -> caller refuses.
/// `row` is the row digits, formatted once and shared across the
/// row's cells.
#[inline]
pub fn f64_cell(out: &mut String, col_letters: &str, row: &str, style: &str, v: f64) {
    if !style.is_empty() {
        f64_cell_styled(out, col_letters, row, style, v);
        return;
    }
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

fn f64_cell_styled(out: &mut String, col_letters: &str, row: &str, style: &str, v: f64) {
    cell_open(out, col_letters, row);
    style_close(out, style);
    if v.is_nan() {
        blank_close_styled(out);
    } else {
        value_open_styled(out);
        if write!(out, "{v}").is_err() {
            debug_assert!(false);
        }
        value_close(out);
    }
}

/// One bool cell. `None` -> blank; `Some` spells `t="b"` with
/// `1`/`0`. `row` is the row digits, shared across the row's cells.
pub fn bool_cell(out: &mut String, col_letters: &str, row: &str, style: &str, v: Option<bool>) {
    if !style.is_empty() {
        bool_cell_styled(out, col_letters, row, style, v);
        return;
    }
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

fn bool_cell_styled(out: &mut String, col_letters: &str, row: &str, style: &str, v: Option<bool>) {
    match v {
        None => {
            cell_open(out, col_letters, row);
            style_close(out, style);
            blank_close_styled(out);
        }
        Some(b) => {
            cell_open(out, col_letters, row);
            style_close(out, style);
            out.push_str(" t=\"b\"><v>");
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
pub fn str_cell(out: &mut String, col_letters: &str, row: &str, style: &str, v: Option<&str>) {
    if !style.is_empty() {
        str_cell_styled(out, col_letters, row, style, v);
        return;
    }
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

fn str_cell_styled(out: &mut String, col_letters: &str, row: &str, style: &str, v: Option<&str>) {
    match v {
        None => {
            cell_open(out, col_letters, row);
            style_close(out, style);
            blank_close_styled(out);
        }
        Some(text) => {
            cell_open(out, col_letters, row);
            style_close(out, style);
            out.push_str(" t=\"inlineStr\"><is><t");
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
fn push_magnitude_bytes(out: &mut Vec<u8>, mag: u64) {
    let (buf, i) = magnitude_digits(mag);
    out.extend_from_slice(&buf[i..]);
}

/// Whether every lane of `word` is a plain ASCII byte
/// (`0x20..=0x7F`: valid one-byte chars, tab/LF/CR and every
/// control below `0x20` excluded, so those fall back to the
/// per-char walk). No lane has its high bit set, and adding
/// `0x60` per lane sets the high bit exactly where the lane is
/// at least `0x20` (no carry escapes a lane: `0x7F + 0x60`
/// still fits one byte).
pub(crate) fn swar_word_is_plain_ascii(word: u64) -> bool {
    (word & SWAR_HIGHS) == 0 && ((word + 0x6060_6060_6060_6060) & SWAR_HIGHS) == SWAR_HIGHS
}

/// Header-text escape into a byte buffer: the same five spellings
/// and SWAR span scan as [`escape_into`], fragments as byte
/// constants and clean spans as byte slices.
pub(crate) fn escape_into_bytes(out: &mut Vec<u8>, text: &str) {
    let bytes: &[u8] = text.as_bytes();
    let mut start: usize = 0;
    let mut i: usize = 0;
    while i < bytes.len() {
        if let Some(word) = swar_word(bytes, i)
            && !swar_has_escapable(word)
        {
            i += 8;
            continue;
        }
        // A word holding an escapable (or the short tail) is
        // walked byte by byte exactly once, never retested:
        // flush the clean span before it, then one append per
        // byte (spans inside a dirty word are short by
        // construction, so slice appends lose to byte appends).
        out.extend_from_slice(&bytes[start..i]);
        let end: usize = usize::min(i + 8, bytes.len());
        while i < end {
            if is_escapable(bytes[i]) {
                out.extend_from_slice(escape_entity(bytes[i]).as_bytes());
            } else {
                out.push(bytes[i]);
            }
            i += 1;
        }
        start = i;
    }
    out.extend_from_slice(&bytes[start..]);
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
pub(crate) fn i64_cell_bytes(
    out: &mut Vec<u8>,
    col_letters: &str,
    row: &[u8],
    style: &str,
    v: i64,
) {
    if !style.is_empty() {
        i64_cell_bytes_styled(out, col_letters, row, style, v);
        return;
    }
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

fn i64_cell_bytes_styled(out: &mut Vec<u8>, col_letters: &str, row: &[u8], style: &str, v: i64) {
    out.extend_from_slice(b"<c r=\"");
    out.extend_from_slice(col_letters.as_bytes());
    out.extend_from_slice(row);
    out.extend_from_slice(b"\"");
    out.extend_from_slice(style.as_bytes());
    if v == NULL_I64 {
        out.extend_from_slice(b"/>");
    } else {
        out.extend_from_slice(b"><v>");
        push_i64_bytes(out, v);
        out.extend_from_slice(b"</v></c>");
    }
}

/// One numeric f64 cell into a byte buffer: byte-identical to
/// [`f64_cell`] (NaN blank, the `Display` spelling write-through).
#[inline]
pub(crate) fn f64_cell_bytes(
    out: &mut Vec<u8>,
    col_letters: &str,
    row: &[u8],
    style: &str,
    v: f64,
) {
    if !style.is_empty() {
        f64_cell_bytes_styled(out, col_letters, row, style, v);
        return;
    }
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

fn f64_cell_bytes_styled(out: &mut Vec<u8>, col_letters: &str, row: &[u8], style: &str, v: f64) {
    out.extend_from_slice(b"<c r=\"");
    out.extend_from_slice(col_letters.as_bytes());
    out.extend_from_slice(row);
    out.extend_from_slice(b"\"");
    out.extend_from_slice(style.as_bytes());
    if v.is_nan() {
        out.extend_from_slice(b"/>");
    } else {
        out.extend_from_slice(b"><v>");
        if write!(VecFmt(out), "{v}").is_err() {
            debug_assert!(false);
        }
        out.extend_from_slice(b"</v></c>");
    }
}

/// One bool cell into a byte buffer: byte-identical to
/// [`bool_cell`].
pub(crate) fn bool_cell_bytes(
    out: &mut Vec<u8>,
    col_letters: &str,
    row: &[u8],
    style: &str,
    v: Option<bool>,
) {
    if !style.is_empty() {
        bool_cell_bytes_styled(out, col_letters, row, style, v);
        return;
    }
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

fn bool_cell_bytes_styled(
    out: &mut Vec<u8>,
    col_letters: &str,
    row: &[u8],
    style: &str,
    v: Option<bool>,
) {
    out.extend_from_slice(b"<c r=\"");
    out.extend_from_slice(col_letters.as_bytes());
    out.extend_from_slice(row);
    out.extend_from_slice(b"\"");
    out.extend_from_slice(style.as_bytes());
    match v {
        None => out.extend_from_slice(b"/>"),
        Some(b) => {
            out.extend_from_slice(b" t=\"b\"><v>");
            out.push(if b { b'1' } else { b'0' });
            out.extend_from_slice(b"</v></c>");
        }
    }
}

/// One inline-string cell into a byte buffer: byte-identical to
/// [`str_cell`].
pub(crate) fn str_cell_bytes(
    out: &mut Vec<u8>,
    col_letters: &str,
    row: &[u8],
    style: &str,
    v: Option<&str>,
) {
    if !style.is_empty() {
        str_cell_bytes_styled(out, col_letters, row, style, v);
        return;
    }
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

fn str_cell_bytes_styled(
    out: &mut Vec<u8>,
    col_letters: &str,
    row: &[u8],
    style: &str,
    v: Option<&str>,
) {
    out.extend_from_slice(b"<c r=\"");
    out.extend_from_slice(col_letters.as_bytes());
    out.extend_from_slice(row);
    out.extend_from_slice(b"\"");
    out.extend_from_slice(style.as_bytes());
    match v {
        None => out.extend_from_slice(b"/>"),
        Some(text) => {
            out.extend_from_slice(b" t=\"inlineStr\"><is><t");
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
        NULL_I64, bool_cell, bool_cell_bytes, col_letters, escape_into, escape_into_bytes,
        f64_cell, f64_cell_bytes, header_cell, header_cell_bytes, i64_cell, i64_cell_bytes,
        push_i64, push_i64_bytes, str_cell, str_cell_bytes,
    };

    /// The SWAR escape spelling pinned literally (both sinks
    /// now share the scan, so the oracle comparison below
    /// cannot pin it alone): escapables on and across word
    /// boundaries, multi-byte text, and the empty text.
    #[test]
    fn escape_spelling_is_pinned() {
        let cases: [(&str, &str); 7] = [
            ("", ""),
            ("plain", "plain"),
            ("1234567&", "1234567&amp;"),
            ("12345678&", "12345678&amp;"),
            ("12345678<abcdefg>", "12345678&lt;abcdefg&gt;"),
            (
                "abcdefgh&ijklmnop<qrstuvwx>yz\"'£&end",
                "abcdefgh&amp;ijklmnop&lt;qrstuvwx&gt;yz&quot;&apos;£&amp;end",
            ),
            ("&<>\"'", "&amp;&lt;&gt;&quot;&apos;"),
        ];
        for (text, want) in cases {
            let mut s = String::new();
            escape_into(&mut s, text);
            assert_eq!(s, want, "string sink {text:?}");
            let mut b: Vec<u8> = Vec::new();
            escape_into_bytes(&mut b, text);
            assert_eq!(b, want.as_bytes(), "byte sink {text:?}");
        }
        // Multi-byte chars straddling an 8-byte word boundary
        // at every straddle offset, with and without an
        // escapable in a following word: the clean-word skip
        // must never land mid-char. Expected output is built
        // char by char here, never through the SWAR scan.
        for ch in ["\u{00A3}", "\u{20AC}", "\u{1F600}"] {
            for prefix_len in 0usize..8 {
                for tail in ["abcdefgh", "abcdefgh&", "hijklm&nop"] {
                    let text: String = format!("{}{ch}{tail}", "a".repeat(prefix_len));
                    let mut want = String::new();
                    for c in text.chars() {
                        match c {
                            '&' => want.push_str("&amp;"),
                            '<' => want.push_str("&lt;"),
                            '>' => want.push_str("&gt;"),
                            '"' => want.push_str("&quot;"),
                            '\'' => want.push_str("&apos;"),
                            _ => want.push(c),
                        }
                    }
                    let mut s = String::new();
                    escape_into(&mut s, &text);
                    assert_eq!(s, want, "string sink straddle {text:?}");
                    let mut b: Vec<u8> = Vec::new();
                    escape_into_bytes(&mut b, &text);
                    assert_eq!(b, want.as_bytes(), "byte sink straddle {text:?}");
                }
            }
        }
        let long_plain: String = "abcdefgh".repeat(40);
        let mut s = String::new();
        escape_into(&mut s, &long_plain);
        assert_eq!(s, long_plain);
    }

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
        i64_cell(&mut s, "A", "2", "", NULL_I64);
        assert_eq!(s, "<c r=\"A2\"/>");
        s.clear();
        f64_cell(&mut s, "B", "2", "", f64::NAN);
        assert_eq!(s, "<c r=\"B2\"/>");
    }

    #[test]
    fn styled_spellings_place_s_after_r_before_t() {
        let mut s = String::new();
        i64_cell(&mut s, "A", "2", " s=\"5\"", 7);
        assert_eq!(s, "<c r=\"A2\" s=\"5\"><v>7</v></c>");
        s.clear();
        i64_cell(&mut s, "A", "2", " s=\"5\"", NULL_I64);
        assert_eq!(s, "<c r=\"A2\" s=\"5\"/>");
        s.clear();
        f64_cell(&mut s, "B", "2", " s=\"5\"", f64::NAN);
        assert_eq!(s, "<c r=\"B2\" s=\"5\"/>");
        s.clear();
        bool_cell(&mut s, "C", "3", " s=\"5\"", Some(true));
        assert_eq!(s, "<c r=\"C3\" s=\"5\" t=\"b\"><v>1</v></c>");
        s.clear();
        bool_cell(&mut s, "C", "3", " s=\"5\"", None);
        assert_eq!(s, "<c r=\"C3\" s=\"5\"/>");
        s.clear();
        str_cell(&mut s, "D", "4", " s=\"5\"", Some("x"));
        assert_eq!(
            s,
            "<c r=\"D4\" s=\"5\" t=\"inlineStr\"><is><t>x</t></is></c>"
        );
        s.clear();
        str_cell(&mut s, "D", "4", " s=\"5\"", None);
        assert_eq!(s, "<c r=\"D4\" s=\"5\"/>");
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
            i64_cell(&mut oracle, "A", "2", "", v);
            i64_cell_bytes(&mut bytes, "A", b"2", "", v);
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
            f64_cell(&mut oracle, "B", "7", "", v);
            f64_cell_bytes(&mut bytes, "B", b"7", "", v);
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
            bool_cell(&mut oracle, "C", "3", "", v);
            bool_cell_bytes(&mut bytes, "C", b"3", "", v);
            assert_eq!(bytes, oracle.as_bytes(), "bool {v:?}");
        }
        assert_eq!(
            {
                let mut s = String::new();
                bool_cell(&mut s, "C", "3", "", Some(true));
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
            str_cell(&mut oracle, "D", "4", "", v);
            str_cell_bytes(&mut bytes, "D", b"4", "", v);
            assert_eq!(bytes, oracle.as_bytes(), "str {v:?}");
        }
        // Styled sweep: bold-only and every colour plain and bold,
        // every kind valued and blank, byte sink against oracle.
        let styled: [&str; 13] = [
            " s=\"1\"",
            " s=\"2\"",
            " s=\"3\"",
            " s=\"4\"",
            " s=\"5\"",
            " s=\"6\"",
            " s=\"7\"",
            " s=\"8\"",
            " s=\"9\"",
            " s=\"10\"",
            " s=\"11\"",
            " s=\"12\"",
            " s=\"13\"",
        ];
        for style in styled {
            for v in [7i64, NULL_I64] {
                oracle.clear();
                bytes.clear();
                i64_cell(&mut oracle, "A", "2", style, v);
                i64_cell_bytes(&mut bytes, "A", b"2", style, v);
                assert_eq!(bytes, oracle.as_bytes(), "styled i64 {style} {v}");
            }
            for v in [1.5f64, f64::NAN] {
                oracle.clear();
                bytes.clear();
                f64_cell(&mut oracle, "B", "7", style, v);
                f64_cell_bytes(&mut bytes, "B", b"7", style, v);
                assert_eq!(bytes, oracle.as_bytes(), "styled f64 {style} {v}");
            }
            for v in [Some(true), None] {
                oracle.clear();
                bytes.clear();
                bool_cell(&mut oracle, "C", "3", style, v);
                bool_cell_bytes(&mut bytes, "C", b"3", style, v);
                assert_eq!(bytes, oracle.as_bytes(), "styled bool {style} {v:?}");
            }
            for v in [Some("x"), None] {
                oracle.clear();
                bytes.clear();
                str_cell(&mut oracle, "D", "4", style, v);
                str_cell_bytes(&mut bytes, "D", b"4", style, v);
                assert_eq!(bytes, oracle.as_bytes(), "styled str {style} {v:?}");
            }
        }
        // Empty string is a real value, distinct from blank.
        let mut empty = String::new();
        str_cell(&mut empty, "D", "4", "", Some(""));
        assert_eq!(empty, "<c r=\"D4\" t=\"inlineStr\"><is><t></t></is></c>");
        let mut blank = String::new();
        str_cell(&mut blank, "D", "4", "", None);
        assert_eq!(blank, "<c r=\"D4\"/>");
        assert_ne!(empty, blank);
        let mut preserved = String::new();
        str_cell(&mut preserved, "D", "4", "", Some(" x"));
        assert_eq!(
            preserved,
            "<c r=\"D4\" t=\"inlineStr\"><is><t xml:space=\"preserve\"> x</t></is></c>"
        );
    }
}
