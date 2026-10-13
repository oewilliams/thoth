//! The workbook assembly: frozen static parts plus named sheets.
//!
//! Static XML is golden bytes (see `tests/roundtrip.rs`): the writer owns
//! the layout, the data owns only `sheetData`. Ingress is borrowed
//! columnar slices, checked rectangular once at the boundary.
//! Every sheet reaches emission through the template contract in
//! `crate::contract`: the builder consumes the declared shape for
//! validation and the reservation, and the arena emission is driven
//! by the contract's generic drivers. Sheet-dependent ceiling
//! verdicts are taken on witnessed lengths at the narrowing casts.

use std::path::{Path, PathBuf};

use crate::contract::{
    ARENA_BLOCK, Column, DynamicSheet, Kind, SheetTemplate, emit_sheet, emit_sheet_arena,
    sheet_hint_witnessed, sheet_letters, validate_template, validate_template_witnessed,
};
use crate::static_parts::{RELS, STYLES};
use crate::workbook_parts::{
    content_types_xml, escape_sheet_name, sheet_part_name, validate_sheet_names, workbook_rels_xml,
    workbook_xml,
};
use crate::zip::{
    begin_entry, checked_u32, crc32, entry_sizes, patch_local_header, stored_archive_len,
    write_central_record, write_end_record,
};

pub use crate::contract::ColumnData;

/// The dynamic template impl, under its pre-015 name: `Sheet`
/// becomes the dynamic impl of the contract (see
/// `crate::contract::DynamicSheet`). Construct it through
/// `DynamicSheet::known` / `unknown_rows`, never a struct literal.
pub type Sheet<'a> = DynamicSheet<'a>;

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
    /// `bytes` is a projected length computed from witnessed
    /// lengths (see `crate::zip` `stored_archive_len`), scoped by
    /// refusal site. Before emission it is `stored_archive_len`
    /// over the known sizes alone (sheets priced at zero data
    /// length, so it excludes sheet payload). At a sheet refusal
    /// it is `stored_archive_len` over the actual entries up to
    /// and including the refusing entry, a lower bound on the
    /// finished archive. In `finish_archive` it is
    /// `stored_archive_len` over every witnessed entry, the
    /// projected finished length.
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
    /// A column's declared kind disagrees with its data variant:
    /// kind is declared once, at the column (RC1).
    KindMismatch {
        name: String,
        declared: Kind,
        actual: Kind,
    },
    /// A string cell passes Excel's 32,767-character cell limit.
    TextTooLong {
        column: String,
        /// 1-based Excel row (header is row 1).
        row: usize,
        len: usize,
    },
    /// A string cell holds a character Excel cannot hold.
    InvalidTextChar {
        column: String,
        /// 1-based Excel row (header is row 1).
        row: usize,
        ch: char,
    },
}

fn kind_label(kind: Kind) -> &'static str {
    match kind {
        Kind::I64 => "i64",
        Kind::F64 => "f64",
        Kind::Str => "str",
        Kind::Bool => "bool",
    }
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
            XlsxError::KindMismatch {
                name,
                declared,
                actual,
            } => {
                write!(
                    f,
                    "column {name:?}: declared kind {} does not match data kind {}",
                    kind_label(*declared),
                    kind_label(*actual)
                )
            }
            XlsxError::TextTooLong { column, row, len } => {
                write!(
                    f,
                    "column {column:?} row {row}: text is {len} characters, excel stops at 32767"
                )
            }
            XlsxError::InvalidTextChar { column, row, ch } => {
                write!(
                    f,
                    "column {column:?} row {row}: character {ch:?} is not a valid excel character"
                )
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
/// check below compares without casting. What is known exactly
/// before emission must each fit its field: the entry count and
/// every name length in `u16`, and the small parts' lengths and
/// section totals over known entries in `u32`. Sheet-dependent
/// lengths fit only when witnessed, at their narrowing casts
/// through `crate::zip::checked_u32`.
const DATA_MAX: usize = u32::MAX as usize;
const NAME_MAX: usize = u16::MAX as usize;
const COUNT_MAX: usize = u16::MAX as usize;
/// Section-total ceiling: the sums below accumulate in `u64` (a
/// `usize` sum could wrap on narrower targets, where this build
/// still refuses correctly), so the totals compare up here.
const TOTAL_MAX: u64 = u32::MAX as u64;

/// Whether the given known entry sizes fit the 32-bit zip fields
/// without zip64. Each tuple is `(data_len, name_len)`, as in
/// `crate::zip::stored_archive_len`. Pure size arithmetic, no
/// allocation: [`build_workbook`] calls it before any emission
/// with what is exact at the boundary — entry count, every entry
/// name length, the small parts at actual length, and sheets at
/// zero data length (their names alone price the central
/// directory, which depends on names and count only). A sheet's
/// actual length is not an input here: it is checked through
/// `checked_u32` where it is patched, as are each entry's local
/// offset and the central start and length where they are
/// written. Over the sizes given, every category is checked:
///
/// - `data_len <= DATA_MAX` for each known length;
/// - `name_len <= NAME_MAX` covers each `name.len() as u16`
///   (local header, central record);
/// - `len <= COUNT_MAX` covers the count narrowed to `u16`
///   (end record);
/// - the local-section total over the known sizes;
/// - the central-section total covers `central_len` over known
///   entries (names and count).
fn archive_fits(sizes: &[(usize, usize)]) -> bool {
    if sizes.len() > COUNT_MAX {
        return false;
    }
    // One fold, no early exit. Exactness: the count check above
    // bounds legal entries to 65535 x u32::MAX, far inside u64, and
    // any illegal entry sets `fits_each`, which is consulted before
    // the sums — so the wrapping sums can never produce a wrong
    // verdict.
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
    let n: u64 = sizes.len() as u64;
    datas + names + 30 * n <= TOTAL_MAX && names + 46 * n <= TOTAL_MAX
}

/// Emit one small part into the arena after its reserved header
/// and patch the header with the CRC of the just-emitted range.
fn emit_small_part(
    out: &mut Vec<u8>,
    entries: &mut Vec<(String, u32, usize, usize)>,
    projected: &mut u64,
    name: &str,
    data: &[u8],
) -> Result<(), XlsxError> {
    let offset: usize = begin_entry(out, name.as_bytes());
    let start: usize = out.len();
    out.extend_from_slice(data);
    let crc: u32 = crc32(&out[start..]);
    // Witnessed length, narrowed through `checked_u32` at the patch;
    // `projected` is `stored_archive_len` over the entries so far.
    let with_this: u64 = *projected + entry_sizes(data.len(), name.len());
    patch_local_header(out, offset, crc, checked_u32(data.len(), with_this)?);
    *projected = with_this;
    entries.push((name.to_string(), crc, data.len(), offset));
    Ok(())
}

/// Build the sheet XML for the given columns (declaration order).
/// This standalone spelling is its own boundary and the permanent
/// oracle; it is implemented through the template contract as a
/// known-shape [`DynamicSheet`].
pub fn sheet_xml(cols: &[(Column, ColumnData<'_>)]) -> Result<Vec<u8>, XlsxError> {
    let template: DynamicSheet<'_> = DynamicSheet::known("data".to_string(), cols);
    sheet_xml_template(&template)
}

/// Build the sheet XML for one template, through the contract.
/// The oracle spelling: generic over the impl, so both impls are
/// checked against the same bytes.
pub fn sheet_xml_template<T: SheetTemplate>(template: &T) -> Result<Vec<u8>, XlsxError> {
    emit_sheet(template, validate_template(template)?)
}

/// Emit every part into the arena after its reserved header,
/// then the central directory and end record.
fn emit_parts<T: SheetTemplate>(
    out: &mut Vec<u8>,
    sheets: &[T],
    row_counts: &[usize],
    letters: &[Vec<String>],
    small_parts: [(&str, &[u8]); 5],
) -> Result<(), XlsxError> {
    let mut entries: Vec<(String, u32, usize, usize)> = Vec::with_capacity(5 + sheets.len());
    // Running `stored_archive_len` over the witnessed entries
    // (the 22-byte end record, then one footprint per entry).
    let mut projected: u64 = 22;
    for (name, data) in small_parts {
        emit_small_part(out, &mut entries, &mut projected, name, data)?;
    }
    for (i, sheet) in sheets.iter().enumerate() {
        let name: String = sheet_part_name(i + 1);
        let offset: usize = begin_entry(out, name.as_bytes());
        let start: usize = out.len();
        let crc: u32 =
            emit_sheet_arena(sheet, row_counts[i], &letters[i], out).map_err(|inner| {
                XlsxError::InSheet {
                    sheet: sheet.sheet_name().to_string(),
                    inner: Box::new(inner),
                }
            })?;
        let len: usize = out.len() - start;
        // Witnessed sheet length, narrowed through `checked_u32`
        // at the patch into the local header. At this refusal
        // `with_this` is `stored_archive_len` over the actual
        // entries up to and including this sheet.
        let with_this: u64 = projected + entry_sizes(len, name.len());
        patch_local_header(out, offset, crc, checked_u32(len, with_this)?);
        projected = with_this;
        entries.push((name, crc, len, offset));
    }
    finish_archive(out, &entries, projected)
}

/// Write the central directory and end record over the witnessed
/// entries. `projected` is the finished length over every entry.
fn finish_archive(
    out: &mut Vec<u8>,
    entries: &[(String, u32, usize, usize)],
    projected: u64,
) -> Result<(), XlsxError> {
    // Witnessed central start, narrowed through `checked_u32`
    // before the central records are written; the value is
    // written into the end record below.
    let central_start_len: usize = out.len();
    let central_start: u32 = checked_u32(central_start_len, projected)?;
    for (name, crc, len, offset) in entries {
        // Witnessed entry length and local offset, each narrowed
        // through `checked_u32` where it is written.
        write_central_record(
            out,
            name.as_bytes(),
            *crc,
            checked_u32(*len, projected)?,
            checked_u32(*offset, projected)?,
        );
    }
    // Witnessed central length, narrowed through `checked_u32`
    // where it is written; the subtraction cannot underflow
    // (`central_start_len` is an earlier length of this buffer).
    let central_len: u32 = checked_u32(out.len() - central_start_len, projected)?;
    write_end_record(out, entries.len(), central_len, central_start);
    Ok(())
}

/// Assemble the whole `.xlsx` (a stored zip) from templates:
/// static parts plus the sheets, emitted once into the one
/// returned buffer. Generic over the template impl, resolved by
/// monomorphisation. Names and shapes are validated before any
/// emission; per-sheet errors name the sheet. The reservation is
/// produced through the contract: the declared hint when the
/// shape is known, or [`ARENA_BLOCK`] plus block growth when a
/// dimension is named unknown.
pub fn build_workbook<T: SheetTemplate>(sheets: &[T]) -> Result<Vec<u8>, XlsxError> {
    let names: Vec<&str> = sheets.iter().map(|sheet| sheet.sheet_name()).collect();
    validate_sheet_names(&names)?;
    let mut row_counts: Vec<usize> = Vec::with_capacity(sheets.len());
    let mut escaped_totals: Vec<usize> = Vec::with_capacity(sheets.len());
    for sheet in sheets {
        let (nrows, escaped): (usize, usize) =
            validate_template_witnessed(sheet).map_err(|inner| XlsxError::InSheet {
                sheet: sheet.sheet_name().to_string(),
                inner: Box::new(inner),
            })?;
        row_counts.push(nrows);
        escaped_totals.push(escaped);
    }
    let escaped: Vec<String> = sheets
        .iter()
        .map(|sheet| escape_sheet_name(sheet.sheet_name()))
        .collect();
    let content_types: String = content_types_xml(sheets.len());
    let workbook: String = workbook_xml(&escaped);
    let workbook_rels: String = workbook_rels_xml(sheets.len());
    let letters: Vec<Vec<String>> = sheets.iter().map(sheet_letters).collect();
    let statics: [(&str, usize); 5] = [
        ("[Content_Types].xml", content_types.len()),
        ("_rels/.rels", RELS.len()),
        ("xl/workbook.xml", workbook.len()),
        ("xl/_rels/workbook.xml.rels", workbook_rels.len()),
        ("xl/styles.xml", STYLES.len()),
    ];
    debug_assert!(statics.iter().all(|(name, _)| !name.starts_with('/')));
    let mut sizes: Vec<(usize, usize)> = Vec::with_capacity(5 + sheets.len());
    let mut hint_sizes: Vec<(usize, usize)> = Vec::with_capacity(5 + sheets.len());
    for (name, len) in statics {
        sizes.push((len, name.len()));
        hint_sizes.push((len, name.len()));
    }
    for (i, sheet) in sheets.iter().enumerate() {
        let name_len: usize = sheet_part_name(i + 1).len();
        // Known before emission: the sheet name alone. Its data
        // length is witnessed only after emission and is checked
        // at the narrowing casts in `emit_parts`.
        sizes.push((0, name_len));
        let hint: usize =
            sheet_hint_witnessed(sheet, &letters[i], escaped_totals[i]).unwrap_or(ARENA_BLOCK);
        hint_sizes.push((hint, name_len));
    }
    // What is known exactly before emission is checked before
    // emission: entry count, entry names, small-part lengths and
    // totals, and central length over the known entries. No
    // estimate refuses here: sheet lengths are witnessed later.
    if !archive_fits(&sizes) {
        let total: u64 = stored_archive_len(&sizes);
        // Error payload only: saturates where `u64` exceeds `usize`.
        let bytes: usize = usize::try_from(total).unwrap_or(usize::MAX);
        return Err(XlsxError::TooLarge { bytes });
    }
    // Reservation: one allocation from the declared shapes, so the
    // known-shape path does not reallocate; an unknown dimension
    // spells `ARENA_BLOCK` here and grows in blocks. The hint is
    // the reservation, never a refusal. The conversion saturates
    // only past `usize`, where `with_capacity` aborts loudly
    // rather than truncating.
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
/// the one-sheet spelling kept for every existing caller,
/// implemented through the contract as a known-shape
/// [`DynamicSheet`].
pub fn build_xlsx(cols: &[(Column, ColumnData<'_>)]) -> Result<Vec<u8>, XlsxError> {
    let sheets: [Sheet<'_>; 1] = [DynamicSheet::known("data".to_string(), cols)];
    build_workbook(&sheets)
}

/// Build the workbook and move it to disk sealed.
pub fn write_workbook_file<T: SheetTemplate>(out: &Path, sheets: &[T]) -> Result<(), WriteError> {
    let bytes: Vec<u8> = build_workbook(sheets)?;
    if out.exists() {
        return Err(WriteError::Exists(out.to_path_buf()));
    }
    let parent: &Path = out.parent().unwrap_or(Path::new("."));
    let file: &std::ffi::OsStr = out
        .file_name()
        .ok_or_else(|| WriteError::NoFileName(out.to_path_buf()))?;
    let tmp: PathBuf = parent.join(format!(".tmp.thoth.{}", file.to_string_lossy()));
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
/// the one-sheet spelling kept for every existing caller,
/// implemented through the contract.
pub fn write_xlsx_file(out: &Path, cols: &[(Column, ColumnData<'_>)]) -> Result<(), WriteError> {
    let sheets: [Sheet<'_>; 1] = [DynamicSheet::known("data".to_string(), cols)];
    write_workbook_file(out, &sheets)
}

#[cfg(test)]
mod tests;
