//! The workbook assembly: frozen static parts plus one sheet.
//!
//! Static XML is golden bytes (see `tests/roundtrip.rs`): the writer owns
//! the layout, the data owns only `sheetData`. v0 carries two cell styles
//! (0 default, 1 bold header) and one sheet; the fixed colour palette and
//! the remaining primitives wait for a caller. Ingress is borrowed
//! columnar slices, checked rectangular once at the boundary.

use std::path::{Path, PathBuf};

use crate::contract::Column;
use crate::emit::{col_letters, f64_cell, header_cell, i64_cell};
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
                write!(f, "too many rows {n}: excel stops at 1048576")
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
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Rename {
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
            WriteError::Io { path, source } => {
                write!(f, "{}: {source}", path.display())
            }
            WriteError::Rename { source } => write!(f, "rename: {source}"),
        }
    }
}

impl std::error::Error for WriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WriteError::Xlsx(e) => Some(e),
            WriteError::Io { source, .. } => Some(source),
            WriteError::Rename { source } => Some(source),
            WriteError::Exists(_) | WriteError::TempExists(_) => None,
        }
    }
}

impl From<XlsxError> for WriteError {
    fn from(e: XlsxError) -> Self {
        WriteError::Xlsx(e)
    }
}

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/><Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/></Types>"#;

const RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#;

const WORKBOOK: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="data" sheetId="1" r:id="rId1"/></sheets></workbook>"#;

const WORKBOOK_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"#;

const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><fonts><font><sz val="11"/><name val="Calibri"/></font><font><b/><sz val="11"/><name val="Calibri"/></font></fonts><fills><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills><borders><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="2"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/><xf numFmtId="0" fontId="1" fillId="0" borderId="0" xfId="0" applyFont="1"/></cellXfs></styleSheet>"#;

/// Excel's hard limits, refused loudly rather than truncated.
pub const MAX_ROWS: usize = 1_048_576;
pub const MAX_COLS: usize = 16_384;

/// Zip32 ceilings, module scope: every part length, every local header
/// offset, and the central directory offset and length must each fit
/// `u32`, and the entry count and every name length must each fit
/// `u16`. (Hoisted from the check below; function scope hid what the
/// whole build path shares.)
const U32_MAX: u64 = u32::MAX as u64;
const U16_MAX: u64 = u16::MAX as u64;

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
/// - `data_len <= u32::MAX` covers each `data.len() as u32`;
/// - `name_len <= u16::MAX` covers each `name.len() as u16`;
/// - `len <= u16::MAX` covers each `entries.len() as u16`;
/// - the running-offset checks cover each `out.len() as u32` (local
///   offsets and the central directory start);
/// - the central-length check covers `central_len`.
fn archive_fits(sizes: &[(usize, usize)]) -> bool {
    if sizes.len() as u64 > U16_MAX {
        return false;
    }
    let mut offset: u64 = 0;
    let mut central_len: u64 = 0;
    for (data_len, name_len) in sizes.iter().copied() {
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
    // Infinities have no Excel spelling: refuse with position.
    for (col, data) in cols {
        if let ColumnData::F64(v) = data {
            for (i, &x) in v.iter().enumerate() {
                if x.is_infinite() {
                    return Err(XlsxError::Infinite {
                        column: col.name.clone(),
                        row: i,
                    });
                }
            }
        }
    }

    let letters: Vec<String> = (0..cols.len()).map(col_letters).collect();
    // Upper-bound capacity, sized once so the buffer never reallocates:
    // fixed tags, headers, then per row the tags plus the widest value
    // spelling (blanks and short values only come in under it). Row
    // digits cap at 7: the limit check above refused anything bigger.
    let mut cap: usize = 256 + 12;
    for (i, (col, _)) in cols.iter().enumerate() {
        cap += letters[i].len() + col.name.len() + 64;
    }
    let mut row_cap: usize = 16 + 7;
    for l in &letters {
        row_cap += l.len() + 7 + 48;
    }
    cap += nrows * row_cap;
    let mut out = String::with_capacity(cap);
    out.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    out.push_str(
        r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>"#,
    );
    out.push_str("<row r=\"1\">");
    for (i, (col, _)) in cols.iter().enumerate() {
        header_cell(&mut out, i, &col.name);
    }
    out.push_str("</row>");
    for r in 0..nrows {
        let row1 = r + 2;
        out.push_str("<row r=\"");
        out.push_str(&row1.to_string());
        out.push_str("\">");
        for (i, (_, data)) in cols.iter().enumerate() {
            match data {
                ColumnData::I64(v) => i64_cell(&mut out, &letters[i], row1, v[r]),
                ColumnData::F64(v) => f64_cell(&mut out, &letters[i], row1, v[r]),
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
/// cleared.
pub fn write_xlsx_file(out: &Path, cols: &[(Column, ColumnData<'_>)]) -> Result<(), WriteError> {
    let bytes = build_xlsx(cols)?;
    if out.exists() {
        return Err(WriteError::Exists(out.to_path_buf()));
    }
    let parent = out.parent().unwrap_or(Path::new("."));
    let file = out
        .file_name()
        .ok_or_else(|| WriteError::Exists(out.to_path_buf()))?;
    let tmp = parent.join(format!(".tmp.thoth.{}", file.to_string_lossy()));
    if tmp.exists() {
        return Err(WriteError::TempExists(tmp));
    }
    std::fs::write(&tmp, &bytes).map_err(|source| WriteError::Io {
        path: tmp.clone(),
        source,
    })?;
    std::fs::rename(&tmp, out).map_err(|source| WriteError::Rename { source })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{XlsxError, archive_fits};

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
}
