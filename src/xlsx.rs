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
    // covers every cast on the assembly path.
    let mut sizes: [(usize, usize); 6] = [(0, 0); 6];
    for (i, part) in parts.iter().enumerate() {
        sizes[i] = (part.1.len(), part.0.len());
    }
    if !crate::zip::stored_archive_fits(&sizes) {
        let total: u64 = crate::zip::stored_archive_len(&sizes);
        let bytes: usize = usize::try_from(total).unwrap_or(usize::MAX);
        return Err(XlsxError::TooLarge { bytes });
    }
    let mut zip = ZipWriter::new();
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
    use super::XlsxError;

    #[test]
    fn too_large_states_the_ceiling() {
        let err: String = XlsxError::TooLarge { bytes: usize::MAX }.to_string();
        assert!(err.contains(&u32::MAX.to_string()), "{err}");
    }
}
