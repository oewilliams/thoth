//! The workbook assembly: frozen static parts plus named sheets.
//!
//! Static XML is golden bytes (see `tests/roundtrip.rs`): the writer owns
//! the layout, the data owns only `sheetData`. v0 carries two cell styles
//! (0 default, 1 bold header); the fixed colour palette and the remaining
//! primitives wait for a caller. Ingress is borrowed columnar slices,
//! checked rectangular once at the boundary.
//!

use std::path::{Path, PathBuf};

use crate::contract::Column;
use crate::emit::{
    col_letters, f64_cell, f64_cell_bytes, header_cell, header_cell_bytes, i64_cell,
    i64_cell_bytes, push_usize, push_usize_bytes,
};
use crate::static_parts::{RELS, STYLES};
use crate::workbook_parts::{
    content_types_xml, escape_sheet_name, sheet_part_name, validate_sheet_names, workbook_rels_xml,
    workbook_xml,
};
use crate::zip::{
    CRC_INIT, begin_entry, crc32, crc32_finish, crc32_update, patch_local_header,
    stored_archive_len, write_central_record, write_end_record,
};

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

/// One named sheet: a name plus one borrowed column set. Each sheet
/// carries its own columns (its own template shape, in the sense
/// `DESIGN.md` Templates allows); the Rust shape of a template is
/// deliberately not locked by this type.
#[derive(Clone, Debug)]
pub struct Sheet<'a> {
    pub name: String,
    pub columns: &'a [(Column, ColumnData<'a>)],
}

#[derive(Debug)]
pub enum XlsxError {
    NoSheets,
    SheetNameEmpty {
        name: String,
    },
    SheetNameTooLong {
        name: String,
    },
    SheetNameInvalidChar {
        name: String,
        ch: char,
    },
    SheetNameApostrophe {
        name: String,
    },
    SheetNameReserved {
        name: String,
    },
    DuplicateSheetName {
        name: String,
    },
    InSheet {
        sheet: String,
        inner: Box<XlsxError>,
    },
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
            XlsxError::NoSheets => write!(f, "no sheets declared"),
            XlsxError::SheetNameEmpty { name } => {
                write!(f, "sheet name \"{name}\": sheet name must not be empty")
            }
            XlsxError::SheetNameTooLong { name } => {
                write!(
                    f,
                    "sheet name \"{name}\": sheet name is {} characters, excel stops at 31",
                    name.chars().count()
                )
            }
            XlsxError::SheetNameInvalidChar { name, ch } => {
                write!(
                    f,
                    "sheet name \"{name}\": character {ch:?} is not allowed in a sheet name"
                )
            }
            XlsxError::SheetNameApostrophe { name } => {
                write!(
                    f,
                    "sheet name \"{name}\": sheet name must not begin or end with an apostrophe"
                )
            }
            XlsxError::SheetNameReserved { name } => {
                write!(f, "sheet name \"{name}\": sheet name is reserved")
            }
            XlsxError::DuplicateSheetName { name } => {
                write!(f, "sheet name \"{name}\": duplicate sheet name")
            }
            XlsxError::InSheet { sheet, inner } => {
                write!(f, "sheet \"{sheet}\": {inner}")
            }
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

/// Whether an archive with the given entry sizes fits the 32-bit
/// zip fields without zip64. Each tuple is `(data_len, name_len)`,
/// as in `crate::zip::stored_archive_len`. Pure size arithmetic,
/// no allocation: the build path calls it before any emission, so
/// no narrowing cast on the arena path ever truncates. This check
/// lives at the ingress boundary ([`build_workbook`], which owns
/// the entry set), not in the zip module. Arena-path sizes are the
/// small parts' actual lengths where known and, for sheets, the
/// `sheet_cap` worst-case projection — conservative, named here
/// because a sheet's actual length exists only after emission;
/// `sheet_cap_holds` proves the actual never exceeds it, so a cast
/// covered for the projection is covered for the actual. Every
/// category is checked explicitly, cast for cast:
///
/// - `data_len <= DATA_MAX` covers each entry length narrowed to
///   `u32` (patched local sizes +18/+22, central record);
/// - `name_len <= NAME_MAX` covers each `name.len() as u16`
///   (local header, central record);
/// - `len <= COUNT_MAX` covers the count narrowed to `u16`
///   (end record);
/// - the local-section total covers each local-header offset
///   narrowed to `u32` (central record) and the central start;
/// - the central-section total covers `central_len` as `u32`.
fn archive_fits(sizes: &[(usize, usize)]) -> bool {
    if sizes.len() > COUNT_MAX {
        return false;
    }
    // One fold, no early exit (benched against the branching loop and
    // a multi-loop spelling: the fold wins, multi-loop loses at every
    // entry count). Exactness: the count check above bounds legal
    // entries to 65535 x u32::MAX, far inside u64, and any illegal
    // entry sets `fits_each`, which is consulted before the sums —
    // so the wrapping sums can never produce a wrong verdict.
    let (datas, names, fits_each) = sizes.iter().fold(
        (0u64, 0u64, true),
        |(datas, names, fits_each), &(data_len, name_len)| {
            (
                datas.wrapping_add(data_len as u64),
                names.wrapping_add(name_len as u64),
                fits_each && data_len <= DATA_MAX && name_len <= NAME_MAX,
            )
        },
    );
    if !fits_each {
        return false;
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
    // Two plain accumulators in one loop (benched against the tuple
    // fold and a counts-and-multiply spelling: the loop ties or wins
    // at realistic column counts; counts wins only near 500 columns).
    let mut header: usize = 256 + 12;
    let mut row: usize = 16 + 7;
    for ((col, data), letter) in cols.iter().zip(letters.iter()) {
        let widest: usize = match data {
            ColumnData::I64(_) => 20,
            ColumnData::F64(_) => 327,
        };
        header += letter.len() + col.name.len() + 64;
        row += letter.len() + 7 + cell_tags + widest;
    }
    header + nrows * row
}

/// Expected sheet size from the declared shape alone: the arena
/// reservation hint. Per-kind expected widths (10 `I64`, 12
/// `F64`) spell a typical value, not the widest — a reservation,
/// never a proof: an under-estimate grows the buffer, an
/// over-estimate is retained capacity measured in TESTING.md
/// (Post-014). The unknown-dimension growth path arrives with
/// the template types, not here.
fn sheet_hint(nrows: usize, cols: &[(Column, ColumnData<'_>)], letters: &[String]) -> usize {
    let mut header: usize = 256 + 12;
    let mut row: usize = 16 + 7;
    for ((col, data), letter) in cols.iter().zip(letters.iter()) {
        let expected: usize = match data {
            ColumnData::I64(_) => 10,
            ColumnData::F64(_) => 12,
        };
        header += letter.len() + col.name.len() + 64;
        row += letter.len() + 7 + 20 + expected;
    }
    header + nrows * row
}

/// Boundary shape check for one sheet's columns, run before any
/// emission: non-empty, within Excel's limits, and rectangular.
/// Returns the data row count; `build_workbook` runs it for every
/// sheet up front, `sheet_xml` as its own boundary.
fn validate_columns(cols: &[(Column, ColumnData<'_>)]) -> Result<usize, XlsxError> {
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
    Ok(nrows)
}

/// Emit one sheet's XML. Assumes `validate_columns` has passed (the
/// row count arrives with that proof); the only refusal left is a
/// value concern — an infinite float, found in row order as the
/// cells are spelled.
fn emit_sheet(cols: &[(Column, ColumnData<'_>)], nrows: usize) -> Result<Vec<u8>, XlsxError> {
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

/// Emit one sheet's XML as bytes into the arena `out`, returning
/// its finished CRC. Assumes `validate_columns` has passed; the
/// only refusal left is an infinite float, in row order (the arena
/// drops with the failed build). The CRC folds once per emitted
/// row over that row's range, so bytes are checksummed while hot,
/// never re-read whole. Byte-identical to [`emit_sheet`].
fn emit_sheet_arena(
    cols: &[(Column, ColumnData<'_>)],
    nrows: usize,
    letters: &[String],
    out: &mut Vec<u8>,
) -> Result<u32, XlsxError> {
    let mut crc_state: u32 = CRC_INIT;
    let mut chunk_start: usize = out.len();
    out.extend_from_slice(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#.as_bytes());
    out.extend_from_slice(
        r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>"#
            .as_bytes(),
    );
    out.extend_from_slice(b"<row r=\"1\">");
    for (i, (col, _)) in cols.iter().enumerate() {
        header_cell_bytes(out, i, &col.name);
    }
    out.extend_from_slice(b"</row>");
    crc_state = crc32_update(crc_state, &out[chunk_start..]);
    chunk_start = out.len();
    // Row digits formatted once per row into a reused buffer: the
    // cells share the spelling, so no cell allocates for them.
    let mut row_digits: Vec<u8> = Vec::new();
    for r in 0..nrows {
        let row1: usize = r + 2;
        row_digits.clear();
        push_usize_bytes(&mut row_digits, row1);
        out.extend_from_slice(b"<row r=\"");
        out.extend_from_slice(&row_digits);
        out.extend_from_slice(b"\">");
        for ((col, data), letter) in cols.iter().zip(letters.iter()) {
            match data {
                ColumnData::I64(v) => i64_cell_bytes(out, letter, &row_digits, v[r]),
                ColumnData::F64(v) => {
                    // Infinities have no Excel spelling: refuse at
                    // the first one in row-major order, exactly as
                    // the oracle spelling does.
                    let x: f64 = v[r];
                    if x.is_infinite() {
                        return Err(XlsxError::Infinite {
                            column: col.name.clone(),
                            row: row1,
                        });
                    }
                    f64_cell_bytes(out, letter, &row_digits, x);
                }
            }
        }
        out.extend_from_slice(b"</row>");
        crc_state = crc32_update(crc_state, &out[chunk_start..]);
        chunk_start = out.len();
    }
    out.extend_from_slice(b"</sheetData></worksheet>");
    crc_state = crc32_update(crc_state, &out[chunk_start..]);
    Ok(crc32_finish(crc_state))
}

/// Emit one small part into the arena after its reserved header
/// and patch the header with the CRC of the just-emitted range:
/// its one copy, in place; entries own no data buffer.
fn emit_small_part(
    out: &mut Vec<u8>,
    entries: &mut Vec<(String, u32, usize, usize)>,
    name: &str,
    data: &[u8],
) {
    let offset: usize = begin_entry(out, name.as_bytes());
    let start: usize = out.len();
    out.extend_from_slice(data);
    let crc: u32 = crc32(&out[start..]);
    // Cast cover: ingress `data_len <= DATA_MAX` refuse in
    // `archive_fits` (the small parts enter it at actual length).
    patch_local_header(out, offset, crc, data.len() as u32);
    entries.push((name.to_string(), crc, data.len(), offset));
}

/// Build the sheet XML for the given columns (declaration order). Row 1
/// is the header; data starts at row 2. This standalone spelling is
/// its own boundary (shape checked here) and the permanent oracle
/// the fused/arena spellings are checked against.
pub fn sheet_xml(cols: &[(Column, ColumnData<'_>)]) -> Result<Vec<u8>, XlsxError> {
    emit_sheet(cols, validate_columns(cols)?)
}

/// Emit every part into the arena after its reserved header,
/// then the central directory and end record.
fn emit_parts(
    out: &mut Vec<u8>,
    sheets: &[Sheet<'_>],
    row_counts: &[usize],
    letters: &[Vec<String>],
    small_parts: [(&str, &[u8]); 5],
) -> Result<(), XlsxError> {
    let mut entries: Vec<(String, u32, usize, usize)> = Vec::with_capacity(5 + sheets.len());
    for (name, data) in small_parts {
        emit_small_part(out, &mut entries, name, data);
    }
    for (i, sheet) in sheets.iter().enumerate() {
        let name: String = sheet_part_name(i + 1);
        let offset: usize = begin_entry(out, name.as_bytes());
        let start: usize = out.len();
        let crc: u32 =
            emit_sheet_arena(sheet.columns, row_counts[i], &letters[i], out).map_err(|inner| {
                XlsxError::InSheet {
                    sheet: sheet.name.clone(),
                    inner: Box::new(inner),
                }
            })?;
        let len: usize = out.len() - start;
        // Cast cover: the sheet's checked `sheet_cap` projection in
        // `archive_fits` (`data_len <= DATA_MAX`), which the actual
        // length never exceeds (`sheet_cap_holds`).
        patch_local_header(out, offset, crc, len as u32);
        entries.push((name, crc, len, offset));
    }
    // Cast cover: the local-section total in `archive_fits` bounds
    // the central start; every local offset precedes it.
    let central_start: u32 = out.len() as u32;
    for (name, crc, len, offset) in &entries {
        // Cast covers: entry lengths as above (actual for small
        // parts, projection-bounded for sheets); local offsets
        // bounded by the same local-section total.
        write_central_record(out, name.as_bytes(), *crc, *len as u32, *offset as u32);
    }
    // Cast cover: the central-section total in `archive_fits`
    // bounds `central_len`; the subtraction cannot underflow by
    // construction (`central_start` is an earlier length of this
    // same buffer, widened back losslessly here).
    let central_len: u32 = (out.len() - central_start as usize) as u32;
    write_end_record(out, entries.len(), central_len, central_start);
    Ok(())
}

/// Assemble the whole `.xlsx` (a stored zip): static parts plus
/// the sheets, emitted once into the one returned buffer. Per
/// entry a 30-byte local header plus name is reserved, the part's
/// bytes emit after it, and CRC and equal stored sizes are patched
/// at +14/+18/+22; entries are `(name, crc, len, offset)` records
/// only — no per-sheet buffer, staged list, or assembly copy.
/// Names and shapes are validated before any emission; per-sheet
/// errors name the sheet. Emission itself is `emit_parts`.
pub fn build_workbook(sheets: &[Sheet<'_>]) -> Result<Vec<u8>, XlsxError> {
    // Boundary pre-pass: names and shapes are refused before any
    // emission, so emission meets only value refusals (infinite).
    validate_sheet_names(sheets)?;
    let mut row_counts: Vec<usize> = Vec::with_capacity(sheets.len());
    for sheet in sheets {
        let nrows = validate_columns(sheet.columns).map_err(|inner| XlsxError::InSheet {
            sheet: sheet.name.clone(),
            inner: Box::new(inner),
        })?;
        row_counts.push(nrows);
    }
    let escaped: Vec<String> = sheets
        .iter()
        .map(|sheet| escape_sheet_name(&sheet.name))
        .collect();
    let content_types: String = content_types_xml(sheets.len());
    let workbook: String = workbook_xml(&escaped);
    let workbook_rels: String = workbook_rels_xml(sheets.len());
    let letters: Vec<Vec<String>> = sheets
        .iter()
        .map(|sheet| (0..sheet.columns.len()).map(col_letters).collect())
        .collect();
    // Entry sizes for the ceiling (`sheet_cap` projection) and
    // the reservation (`sheet_hint`); statics at actual length.
    let statics: [(&str, usize); 5] = [
        ("[Content_Types].xml", content_types.len()),
        ("_rels/.rels", RELS.len()),
        ("xl/workbook.xml", workbook.len()),
        ("xl/_rels/workbook.xml.rels", workbook_rels.len()),
        ("xl/styles.xml", STYLES.len()),
    ];
    // Name shape holds by construction (fixed literals above,
    // `sheet{index}.xml` per sheet); this pins a wrong new literal
    // in debug and test builds. The zip primitives trust them.
    debug_assert!(statics.iter().all(|(name, _)| !name.starts_with('/')));
    let mut sizes: Vec<(usize, usize)> = Vec::with_capacity(5 + sheets.len());
    let mut hint_sizes: Vec<(usize, usize)> = Vec::with_capacity(5 + sheets.len());
    for (name, len) in statics {
        sizes.push((len, name.len()));
        hint_sizes.push((len, name.len()));
    }
    for (i, sheet) in sheets.iter().enumerate() {
        let name_len: usize = sheet_part_name(i + 1).len();
        sizes.push((
            sheet_cap(row_counts[i], sheet.columns, &letters[i]),
            name_len,
        ));
        hint_sizes.push((
            sheet_hint(row_counts[i], sheet.columns, &letters[i]),
            name_len,
        ));
    }
    // Zip32 ceiling, before any arena byte is written: refuse past
    // it rather than truncate (cast-for-cast in `archive_fits`).
    if !archive_fits(&sizes) {
        let total: u64 = stored_archive_len(&sizes);
        // Error payload only: saturates where `u64` exceeds `usize`.
        let bytes: usize = usize::try_from(total).unwrap_or(usize::MAX);
        return Err(XlsxError::TooLarge { bytes });
    }
    // Reservation: one allocation from the declared shapes, so the
    // known-shape path does not reallocate. `sheet_cap` stays the
    // worst-case proof, not the reservation. The conversion
    // saturates only past `usize`, where `with_capacity` aborts
    // loudly rather than truncating.
    let reservation: usize = usize::try_from(stored_archive_len(&hint_sizes)).unwrap_or(usize::MAX);
    let mut out: Vec<u8> = Vec::with_capacity(reservation);
    let small_parts: [(&str, &[u8]); 5] = [
        ("[Content_Types].xml", content_types.as_bytes()),
        ("_rels/.rels", RELS.as_bytes()),
        ("xl/workbook.xml", workbook.as_bytes()),
        ("xl/_rels/workbook.xml.rels", workbook_rels.as_bytes()),
        ("xl/styles.xml", STYLES.as_bytes()),
    ];
    emit_parts(&mut out, sheets, &row_counts, &letters, small_parts)?;
    Ok(out)
}

/// Assemble the whole `.xlsx` with the single sheet named `data`:
/// the one-sheet spelling kept for every existing caller; its bytes
/// are identical to the pre-011 single-sheet build.
pub fn build_xlsx(cols: &[(Column, ColumnData<'_>)]) -> Result<Vec<u8>, XlsxError> {
    let sheets: [Sheet<'_>; 1] = [Sheet {
        name: "data".to_string(),
        columns: cols,
    }];
    build_workbook(&sheets)
}

/// Build the workbook and move it to disk sealed: an existing
/// destination is refused, bytes land in a dot-prefixed temp file beside
/// it and rename into place. Debris is left for a human, never silently
/// cleared. Durability is caller-owned: write plus rename with no sync.
pub fn write_workbook_file(out: &Path, sheets: &[Sheet<'_>]) -> Result<(), WriteError> {
    let bytes = build_workbook(sheets)?;
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

/// Build the single sheet named `data` and move it to disk sealed:
/// the one-sheet spelling kept for every existing caller.
pub fn write_xlsx_file(out: &Path, cols: &[(Column, ColumnData<'_>)]) -> Result<(), WriteError> {
    let sheets: [Sheet<'_>; 1] = [Sheet {
        name: "data".to_string(),
        columns: cols,
    }];
    write_workbook_file(out, &sheets)
}

#[cfg(test)]
mod tests;
