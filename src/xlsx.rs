//! The workbook assembly: frozen static parts plus one sheet.
//!
//! Static XML is golden bytes (see `tests/roundtrip.rs`): the writer owns
//! the layout, the data owns only `sheetData`. v0 carries two cell styles
//! (0 default, 1 bold header) and one sheet; the fixed colour palette and
//! the remaining primitives wait for a caller. Ingress is borrowed
//! columnar slices, checked rectangular once at the boundary.

use std::path::{Path, PathBuf};

use crate::contract::Column;
use crate::emit::{col_letters, f64_cell, header_cell, i64_cell, push_usize};
use crate::static_parts::{CONTENT_TYPES, RELS, STYLES, WORKBOOK, WORKBOOK_RELS};
use crate::zip::ZipWriter;

/// One column's payload, borrowed. Contiguous slices only: one pointer
/// plus one length, no per-cell indirection.
#[derive(Clone, Copy, Debug)]
pub enum ColumnData<'a> {
    I64(&'a [i64]),
    F64(&'a [f64]),
}

impl<'a> ColumnData<'a> {
    fn len(&self) -> usize {
        match self {
            ColumnData::I64(v) => v.len(),
            ColumnData::F64(v) => v.len(),
        }
    }
}

#[derive(Debug)]
pub enum XlsxError {
    EmptyColumns,
    /// Data rows (the sheet adds the header row).
    TooManyRows(usize),
    TooManyColumns(usize),
    /// The finished archive would not fit the 32-bit zip fields:
    /// `bytes` is its projected length (see `crate::zip`
    /// `stored_archive_len`).
    TooLarge {
        bytes: usize,
    },
    Infinite {
        column: String,
        /// 1-based Excel row (header is row 1).
        row: usize,
    },
    Ragged {
        name: String,
        len: usize,
        expected: usize,
    },
}

impl std::fmt::Display for XlsxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XlsxError::EmptyColumns => write!(f, "no columns declared"),
            XlsxError::TooManyRows(n) => {
                write!(
                    f,
                    "too many data rows {n}: sheet would have {} rows, excel stops at {}",
                    n.saturating_add(1),
                    MAX_ROWS
                )
            }
            XlsxError::TooManyColumns(n) => {
                write!(f, "too many columns {n}: excel stops at 16384")
            }
            XlsxError::TooLarge { bytes } => {
                write!(f, "archive too large {bytes}: zip stops at {}", u32::MAX)
            }
            XlsxError::Infinite { column, row } => {
                write!(
                    f,
                    "column {column:?} row {row}: infinite is not an excel value"
                )
            }
            XlsxError::Ragged {
                name,
                len,
                expected,
            } => {
                write!(f, "column {name:?}: ragged length {len}, want {expected}")
            }
        }
    }
}

impl std::error::Error for XlsxError {}

#[derive(Debug)]
pub enum WriteError {
    Xlsx(XlsxError),
    Exists(PathBuf),
    TempExists(PathBuf),
    /// The output path names no file: nothing was written.
    NoFileName(PathBuf),
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Rename {
        tmp: PathBuf,
        out: PathBuf,
        source: std::io::Error,
    },
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriteError::Xlsx(e) => write!(f, "{e}"),
            WriteError::Exists(p) => write!(
                f,
                "{} exists and a sealed object is never overwritten",
                p.display()
            ),
            WriteError::TempExists(p) => write!(
                f,
                "{} exists from a crashed run; clear it by hand",
                p.display()
            ),
            WriteError::NoFileName(p) => write!(f, "{}: no file name", p.display()),
            WriteError::Io { path, source } => {
                write!(f, "{}: {source}", path.display())
            }
            WriteError::Rename { tmp, out, source } => {
                write!(f, "rename {} to {}: {source}", tmp.display(), out.display())
            }
        }
    }
}

impl std::error::Error for WriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WriteError::Xlsx(e) => Some(e),
            WriteError::Io { source, .. } => Some(source),
            WriteError::Rename { source, .. } => Some(source),
            WriteError::Exists(_) | WriteError::TempExists(_) | WriteError::NoFileName(_) => None,
        }
    }
}

impl From<XlsxError> for WriteError {
    fn from(e: XlsxError) -> Self {
        WriteError::Xlsx(e)
    }
}

/// Excel's hard limits, refused loudly rather than truncated.
pub const MAX_ROWS: usize = 1_048_576;
pub const MAX_COLS: usize = 16_384;

/// Zip32 ceilings in the address width: compile-time values, so the
/// check below compares without casting. Every part length, every
/// local header offset, and the central directory offset and length
/// must each fit `u32`; the entry count and every name length must
/// each fit `u16`.
const DATA_MAX: usize = u32::MAX as usize;
const NAME_MAX: usize = u16::MAX as usize;
const COUNT_MAX: usize = u16::MAX as usize;
/// Section-total ceiling: the sums below accumulate in `u64` (a
/// `usize` sum could wrap on narrower targets, where this build
/// still refuses correctly), so the totals compare up here.
const TOTAL_MAX: u64 = u32::MAX as u64;

/// Whether an archive with the given entry sizes fits the 32-bit zip
/// fields without zip64. Each tuple is `(data_len, name_len)` in
/// bytes, as in `crate::zip::stored_archive_len`. Pure size
/// arithmetic, no allocation: the build path calls it before
/// assembly, so no narrowing cast in `ZipWriter::finish` ever
/// truncates. This check lives at the ingress boundary
/// ([`build_xlsx`], which owns the entry set), not in the zip module.
///
/// A fitting total alone would imply the `u32` fields fit (each is
/// bounded by it), but the count and name checks have no such cover:
/// thousands of tiny entries, or one long name, overflow their `u16`
/// fields under a small total. Every category is checked explicitly
/// so the reviewer can match each check to its cast, cast for cast:
///
/// - `data_len <= DATA_MAX` covers each `data.len() as u32`;
/// - `name_len <= NAME_MAX` covers each `name.len() as u16`;
/// - `len <= COUNT_MAX` covers each `entries.len() as u16`;
/// - the local-section total covers each `out.len() as u32`
///   (every local start precedes it, including the central start);
/// - the central-section total covers `central_len`.
fn archive_fits(sizes: &[(usize, usize)]) -> bool {
    if sizes.len() > COUNT_MAX {
        return false;
    }
    let mut datas: u64 = 0;
    let mut names: u64 = 0;
    for &(data_len, name_len) in sizes.iter() {
        if data_len > DATA_MAX || name_len > NAME_MAX {
            return false;
        }
        datas += data_len as u64;
        names += name_len as u64;
    }
    // Closed form: 30 bytes per local header, 46 per central record.
    let n: u64 = sizes.len() as u64;
    datas + names + 30 * n <= TOTAL_MAX && names + 46 * n <= TOTAL_MAX
}

/// Upper-bound sheet size in bytes, the single spelling behind
/// `sheet_xml`'s one allocation: headers, then per row the tags
/// plus each kind's widest value (20 `I64` with `i64::MIN` blank,
/// 327 `F64`: Display never uses exponents, so `1e300` needs 301
/// digits; pinned by `sheet_cap_holds`). Row digits cap at 7 by
/// the limit check.
fn sheet_cap(nrows: usize, cols: &[(Column, ColumnData<'_>)], letters: &[String]) -> usize {
    let cell_tags: usize = "<c r=\"".len() + "\"><v>".len() + "</v></c>".len();
    let (header, row) = cols.iter().zip(letters.iter()).fold(
        (256 + 12, 16 + 7),
        |(header, row), ((col, data), letter)| {
            let widest: usize = match data {
                ColumnData::I64(_) => 20,
                ColumnData::F64(_) => 327,
            };
            (
                header + letter.len() + col.name.len() + 64,
                row + letter.len() + 7 + cell_tags + widest,
            )
        },
    );
    header + nrows * row
}

/// Build the sheet XML for the given columns (declaration order). Row 1
/// is the header; data starts at row 2. One monomorphic loop per column
/// kind. Lengths are checked once here: ragged columns are refused.
pub fn sheet_xml(cols: &[(Column, ColumnData<'_>)]) -> Result<Vec<u8>, XlsxError> {
    if cols.is_empty() {
        return Err(XlsxError::EmptyColumns);
    }
    if cols.len() > MAX_COLS {
        return Err(XlsxError::TooManyColumns(cols.len()));
    }
    let nrows = cols[0].1.len();
    for (col, data) in &cols[1..] {
        if data.len() != nrows {
            return Err(XlsxError::Ragged {
                name: col.name.clone(),
                len: data.len(),
                expected: nrows,
            });
        }
    }
    if nrows + 1 > MAX_ROWS {
        return Err(XlsxError::TooManyRows(nrows));
    }
    let letters: Vec<String> = (0..cols.len()).map(col_letters).collect();
    // One allocation from the shared bound (`sheet_cap_holds`).
    let mut out = String::with_capacity(sheet_cap(nrows, cols, &letters));
    out.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    out.push_str(
        r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>"#,
    );
    out.push_str("<row r=\"1\">");
    for (i, (col, _)) in cols.iter().enumerate() {
        header_cell(&mut out, i, &col.name);
    }
    out.push_str("</row>");
    // Row digits formatted once per row into a reused buffer: the
    // cells share the spelling, so no cell allocates for them.
    let mut row_digits = String::new();
    for r in 0..nrows {
        let row1 = r + 2;
        row_digits.clear();
        push_usize(&mut row_digits, row1);
        out.push_str("<row r=\"");
        out.push_str(&row_digits);
        out.push_str("\">");
        for ((col, data), letter) in cols.iter().zip(letters.iter()) {
            match data {
                ColumnData::I64(v) => i64_cell(&mut out, letter, &row_digits, v[r]),
                ColumnData::F64(v) => {
                    // Infinities have no Excel spelling: refuse at the
                    // first one in row-major order (single-infinite
                    // inputs, the only pinned case, match the old
                    // column-major scan exactly).
                    let x: f64 = v[r];
                    if x.is_infinite() {
                        return Err(XlsxError::Infinite {
                            column: col.name.clone(),
                            row: row1,
                        });
                    }
                    f64_cell(&mut out, letter, &row_digits, x);
                }
            }
        }
        out.push_str("</row>");
    }
    out.push_str("</sheetData></worksheet>");
    Ok(out.into_bytes())
}

/// Assemble the whole `.xlsx` (a zip): static parts plus the sheet.
pub fn build_xlsx(cols: &[(Column, ColumnData<'_>)]) -> Result<Vec<u8>, XlsxError> {
    let sheet: Vec<u8> = sheet_xml(cols)?;
    // Owned statics: ~117ns per build measured (TESTING.md);
    // borrowing needs `ZipWriter` churn no measurement justifies.
    let parts: [(&str, Vec<u8>); 6] = [
        ("[Content_Types].xml", CONTENT_TYPES.as_bytes().to_vec()),
        ("_rels/.rels", RELS.as_bytes().to_vec()),
        ("xl/workbook.xml", WORKBOOK.as_bytes().to_vec()),
        (
            "xl/_rels/workbook.xml.rels",
            WORKBOOK_RELS.as_bytes().to_vec(),
        ),
        ("xl/styles.xml", STYLES.as_bytes().to_vec()),
        ("xl/worksheets/sheet1.xml", sheet),
    ];
    // Zip32 ceiling, before any assembly byte is written: every length
    // and offset below narrows to `u32`/`u16` with no zip64 to catch
    // it, so refuse past the ceiling rather than truncate. Sheet
    // formatting performs no narrowing conversion, so this one check
    // covers every cast on the assembly path (see `archive_fits` for
    // the cast-for-cast mapping).
    let mut sizes: [(usize, usize); 6] = [(0, 0); 6];
    for (i, part) in parts.iter().enumerate() {
        sizes[i] = (part.1.len(), part.0.len());
    }
    if !archive_fits(&sizes) {
        let total: u64 = crate::zip::stored_archive_len(&sizes);
        // Error payload only: saturates where `u64` exceeds `usize`.
        let bytes: usize = usize::try_from(total).unwrap_or(usize::MAX);
        return Err(XlsxError::TooLarge { bytes });
    }
    // Name shape and uniqueness are ingress checks too, stated here
    // where the entry set is owned: the six names are fixed literals,
    // so validity holds by construction, and these debug checks pin
    // it (a wrong new literal trips the debug and test builds;
    // release pays nothing). The zip module trusts them outright.
    debug_assert!(parts.iter().all(|(name, _)| !name.starts_with('/')));
    debug_assert!({
        let mut unique = true;
        for (i, (name, _)) in parts.iter().enumerate() {
            unique &= !parts[..i].iter().any(|(other, _)| *other == *name);
        }
        unique
    });
    let mut zip = ZipWriter::new(parts.len());
    for (name, data) in parts {
        zip.add(name, data);
    }
    Ok(zip.finish())
}

/// Build the workbook and move it to disk sealed: an existing
/// destination is refused, bytes land in a dot-prefixed temp file beside
/// it and rename into place. Debris is left for a human, never silently
/// cleared. Durability is caller-owned: write plus rename with no sync.
pub fn write_xlsx_file(out: &Path, cols: &[(Column, ColumnData<'_>)]) -> Result<(), WriteError> {
    let bytes = build_xlsx(cols)?;
    if out.exists() {
        return Err(WriteError::Exists(out.to_path_buf()));
    }
    let parent = out.parent().unwrap_or(Path::new("."));
    let file = out
        .file_name()
        .ok_or_else(|| WriteError::NoFileName(out.to_path_buf()))?;
    let tmp = parent.join(format!(".tmp.thoth.{}", file.to_string_lossy()));
    if tmp.exists() {
        return Err(WriteError::TempExists(tmp));
    }
    std::fs::write(&tmp, &bytes).map_err(|source| WriteError::Io {
        path: tmp.clone(),
        source,
    })?;
    std::fs::rename(&tmp, out).map_err(|source| WriteError::Rename {
        tmp: tmp.clone(),
        out: out.to_path_buf(),
        source,
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ColumnData, WriteError, XlsxError, archive_fits, sheet_cap, sheet_xml};
    use crate::contract::{Column, Kind};
    use crate::emit::col_letters;
    use crate::static_parts::{CONTENT_TYPES, RELS, STYLES, WORKBOOK, WORKBOOK_RELS};

    #[test]
    fn too_large_states_the_ceiling() {
        let err: String = XlsxError::TooLarge { bytes: usize::MAX }.to_string();
        assert!(err.contains(&u32::MAX.to_string()), "{err}");
    }

    #[test]
    fn archive_fits_flips_at_the_u32_ceiling() {
        let max: usize = u32::MAX as usize;
        // One entry with an empty name carries 98 bytes of headers
        // (30 local + 46 central + 22 end), so data fills the last
        // offset exactly at `max - 30` and one byte more refuses.
        // Only sizes are decided here; no bytes are allocated.
        assert!(archive_fits(&[(max - 30, 0)]));
        assert!(!archive_fits(&[(max - 29, 0)]));
        assert!(!archive_fits(&[(max - 1, 0)]));
        assert!(!archive_fits(&[(max, 0)]));
        assert!(!archive_fits(&[(max + 1, 0)]));
    }

    #[test]
    fn archive_fits_accumulates_offsets_across_entries() {
        let max: usize = u32::MAX as usize;
        // Two individually small parts whose running offset still
        // overflows the central directory start.
        assert!(!archive_fits(&[(max - 30, 0), (1, 0)]));
        assert!(archive_fits(&[(8, 2), (300, 5)]));
    }

    #[test]
    fn archive_fits_bounds_name_lengths_at_u16() {
        assert!(archive_fits(&[(0, u16::MAX as usize)]));
        assert!(!archive_fits(&[(0, u16::MAX as usize + 1)]));
    }

    #[test]
    fn archive_fits_bounds_entry_count_at_u16() {
        let over: Vec<(usize, usize)> = vec![(0, 0); u16::MAX as usize + 1];
        assert!(!archive_fits(&over));
        let at_max: Vec<(usize, usize)> = vec![(0, 0); u16::MAX as usize];
        assert!(archive_fits(&at_max));
    }

    #[test]
    fn archive_fits_bounds_central_directory_length() {
        // Every `u16` field fits, but the central directory itself
        // passes the `u32` ceiling, so the archive refuses.
        let entries: Vec<(usize, usize)> = vec![(0, u16::MAX as usize); u16::MAX as usize];
        assert!(!archive_fits(&entries));
    }

    /// The second data value refuses at Excel row 3.
    #[test]
    fn infinite_names_the_excel_row() {
        let px: Vec<f64> = vec![1.0, f64::INFINITY];
        let cols: Vec<(Column, ColumnData<'_>)> = vec![(
            Column {
                name: "px".to_string(),
                kind: Kind::F64,
            },
            ColumnData::F64(&px),
        )];
        let err: String = sheet_xml(&cols).unwrap_err().to_string();
        assert!(err.contains("row 3"), "{err}");
    }

    /// The message states data rows and the sheet total.
    #[test]
    fn too_many_rows_states_data_and_total() {
        let err: String = XlsxError::TooManyRows(1_048_576).to_string();
        assert!(err.contains("1048576"), "{err}");
        assert!(err.contains("1048577"), "{err}");
    }

    /// Both ends of the failed move (a rename failure is inducible
    /// only by the filesystem).
    #[test]
    fn rename_names_tmp_and_out() {
        let err: WriteError = WriteError::Rename {
            tmp: std::path::PathBuf::from("/tmp/.tmp.thoth.sealed.xlsx"),
            out: std::path::PathBuf::from("/tmp/sealed.xlsx"),
            source: std::io::Error::other("boom"),
        };
        let words: String = err.to_string();
        assert!(words.contains("/tmp/.tmp.thoth.sealed.xlsx"), "{words}");
        assert!(words.contains("/tmp/sealed.xlsx"), "{words}");
        assert!(words.contains("boom"), "{words}");
    }

    /// Tripwire on the five frozen statics: pins captured
    /// pre-lock; `crc32` is a change detector, round-trip is
    /// the oracle.
    #[test]
    fn static_parts_are_pinned() {
        let pinned: [(&str, usize, u32); 5] = [
            ("[Content_Types].xml", 680, 0xCC79_080A),
            ("_rels/.rels", 296, 0x82C7_5906),
            ("xl/workbook.xml", 282, 0xD473_8FA2),
            ("xl/_rels/workbook.xml.rels", 424, 0x8A96_42E0),
            ("xl/styles.xml", 708, 0x24A3_B074),
        ];
        let parts: [&str; 5] = [CONTENT_TYPES, RELS, WORKBOOK, WORKBOOK_RELS, STYLES];
        for ((name, len, crc), part) in pinned.iter().zip(parts.iter()) {
            assert_eq!(part.len(), *len, "{name}");
            assert_eq!(crate::zip::crc32(part.as_bytes()), *crc, "{name}");
        }
    }

    /// A missing file name is its own refusal, never `Exists`.
    #[test]
    fn no_file_name_states_its_condition() {
        let err: WriteError = WriteError::NoFileName(std::path::PathBuf::from(""));
        assert!(err.to_string().contains("no file name"), "{err:?}");
        assert!(std::error::Error::source(&err).is_none());
    }

    /// `len <= cap` is the no-realloc proof; shapes are
    /// structural representatives, extremes included.
    #[test]
    fn sheet_cap_holds() {
        fn check(cols: &[(Column, ColumnData<'_>)]) {
            let nrows: usize = cols[0].1.len();
            let letters: Vec<String> = (0..cols.len()).map(col_letters).collect();
            let cap: usize = sheet_cap(nrows, cols, &letters);
            let sheet: Vec<u8> = sheet_xml(cols).unwrap();
            assert!(
                sheet.len() <= cap,
                "sheet {} bytes over cap {cap}",
                sheet.len()
            );
        }
        let empty_i: Vec<i64> = vec![];
        let empty_f: Vec<f64> = vec![];
        let header: Vec<(Column, ColumnData<'_>)> = vec![
            (
                Column {
                    name: "t".to_string(),
                    kind: Kind::I64,
                },
                ColumnData::I64(&empty_i),
            ),
            (
                Column {
                    name: "px".to_string(),
                    kind: Kind::F64,
                },
                ColumnData::F64(&empty_f),
            ),
        ];
        check(&header);
        let wide_data: Vec<Vec<i64>> = (0..30).map(|i| vec![i, -i]).collect();
        let wide: Vec<(Column, ColumnData<'_>)> = wide_data
            .iter()
            .enumerate()
            .map(|(i, d)| {
                (
                    Column {
                        name: format!("c{i}"),
                        kind: Kind::I64,
                    },
                    ColumnData::I64(d),
                )
            })
            .collect();
        check(&wide);
        // Non-null integer ends, longest positional floats (all finite).
        let extremes_i: [i64; 2] = [i64::MAX, i64::MIN + 1];
        let tall_i: Vec<i64> = (0..500).map(|r| extremes_i[r % 2]).collect();
        let extremes_f: [f64; 4] = [1e300, 5e-324, -f64::MAX, 0.5];
        let tall_f: Vec<f64> = (0..500).map(|r| extremes_f[r % 4]).collect();
        let tall: Vec<(Column, ColumnData<'_>)> = vec![
            (
                Column {
                    name: "t".to_string(),
                    kind: Kind::I64,
                },
                ColumnData::I64(&tall_i),
            ),
            (
                Column {
                    name: "px".to_string(),
                    kind: Kind::F64,
                },
                ColumnData::F64(&tall_f),
            ),
        ];
        check(&tall);
    }
}
