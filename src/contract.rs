//! The template contract: a template declares its columns, their
//! data, and its shape knowledge once; the drivers below consume
//! that declaration without re-deriving shape. Impls are
//! monomorphised; the row loop never branches on impl.

use crate::emit::{
    col_letters, f64_cell, f64_cell_bytes, header_cell, header_cell_bytes, i64_cell,
    i64_cell_bytes, push_usize, push_usize_bytes,
};
use crate::xlsx::{MAX_COLS, MAX_ROWS, XlsxError};
use crate::zip::{CRC_INIT, crc32_finish, crc32_update};

/// Column kind: the one decision made per column, never per cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    I64,
    F64,
}

/// One declared column: Excel order is declaration order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Column {
    pub name: String,
    pub kind: Kind,
}

/// The i64 null: absence lives in the value's own bits.
pub const NULL_I64: i64 = i64::MIN;

/// One column's payload: one pointer plus one length.
#[derive(Clone, Copy, Debug)]
pub enum ColumnData<'a> {
    I64(&'a [i64]),
    F64(&'a [f64]),
}

impl<'a> ColumnData<'a> {
    pub fn len(&self) -> usize {
        match self {
            ColumnData::I64(v) => v.len(),
            ColumnData::F64(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn kind(&self) -> Kind {
        match self {
            ColumnData::I64(_) => Kind::I64,
            ColumnData::F64(_) => Kind::F64,
        }
    }

    pub fn from_static_i64<const N: usize>(data: &'a [i64; N]) -> Self {
        ColumnData::I64(data.as_slice())
    }

    pub fn from_static_f64<const N: usize>(data: &'a [f64; N]) -> Self {
        ColumnData::F64(data.as_slice())
    }
}

/// Which dimension a template does not know, named exactly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnknownDimension {
    Rows,
    Columns,
    RowsAndColumns,
}

/// Declared shape: each dimension known, or named unknown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShapeKnowledge {
    rows: Option<usize>,
    columns: Option<usize>,
}

impl ShapeKnowledge {
    pub const fn known(rows: usize, columns: usize) -> Self {
        ShapeKnowledge {
            rows: Some(rows),
            columns: Some(columns),
        }
    }

    pub const fn unknown_rows(columns: usize) -> Self {
        ShapeKnowledge {
            rows: None,
            columns: Some(columns),
        }
    }

    pub const fn unknown_columns(rows: usize) -> Self {
        ShapeKnowledge {
            rows: Some(rows),
            columns: None,
        }
    }

    pub const fn unknown_both() -> Self {
        ShapeKnowledge {
            rows: None,
            columns: None,
        }
    }

    pub fn rows(&self) -> Option<usize> {
        self.rows
    }

    pub fn columns(&self) -> Option<usize> {
        self.columns
    }

    pub fn unknown_dimension(&self) -> Option<UnknownDimension> {
        match (self.rows, self.columns) {
            (Some(_), Some(_)) => None,
            (None, Some(_)) => Some(UnknownDimension::Rows),
            (Some(_), None) => Some(UnknownDimension::Columns),
            (None, None) => Some(UnknownDimension::RowsAndColumns),
        }
    }
}

/// First arena block when no hint is known; the arena grows from here.
pub const ARENA_BLOCK: usize = 8192;

/// One column for the drivers: name, declared kind, data.
/// Gathered once per sheet before the row loop.
#[derive(Clone, Copy, Debug)]
pub struct ColumnView<'a> {
    pub name: &'a str,
    pub kind: Kind,
    pub data: ColumnData<'a>,
}

/// The template contract: declare columns, data, and shape once.
pub trait SheetTemplate {
    fn sheet_name(&self) -> &str;
    fn shape(&self) -> ShapeKnowledge;
    /// The declared columns in order, gathered once per sheet.
    fn columns_view(&self) -> Vec<ColumnView<'_>>;

    fn column_count(&self) -> usize {
        self.columns_view().len()
    }
}

/// Dynamic impl: runtime columns over borrowed slices.
/// `Sheet` in `crate::xlsx` is this type.
#[derive(Clone, Debug)]
pub struct DynamicSheet<'a> {
    pub name: String,
    pub columns: &'a [(Column, ColumnData<'a>)],
    shape: ShapeKnowledge,
}

impl<'a> DynamicSheet<'a> {
    /// Known shape, derived once here from the slices.
    pub fn known(name: String, columns: &'a [(Column, ColumnData<'a>)]) -> Self {
        let rows: usize = columns.first().map_or(0, |(_, data)| data.len());
        DynamicSheet {
            name,
            columns,
            shape: ShapeKnowledge::known(rows, columns.len()),
        }
    }

    /// Unknown rows; the builder grows in blocks.
    pub fn unknown_rows(name: String, columns: &'a [(Column, ColumnData<'a>)]) -> Self {
        DynamicSheet {
            name,
            columns,
            shape: ShapeKnowledge::unknown_rows(columns.len()),
        }
    }

    /// Unknown columns; the builder grows in blocks.
    pub fn unknown_columns(name: String, columns: &'a [(Column, ColumnData<'a>)]) -> Self {
        let rows: usize = columns.first().map_or(0, |(_, data)| data.len());
        DynamicSheet {
            name,
            columns,
            shape: ShapeKnowledge::unknown_columns(rows),
        }
    }
}

impl<'a> SheetTemplate for DynamicSheet<'a> {
    fn sheet_name(&self) -> &str {
        &self.name
    }

    fn shape(&self) -> ShapeKnowledge {
        self.shape
    }

    fn columns_view(&self) -> Vec<ColumnView<'_>> {
        self.columns
            .iter()
            .map(|(col, data)| ColumnView {
                name: &col.name,
                kind: col.kind,
                data: *data,
            })
            .collect()
    }

    fn column_count(&self) -> usize {
        self.columns.len()
    }
}

/// One column of a [`StaticSheet`]: fixed `[T; R]` data.
#[derive(Clone, Copy, Debug)]
pub enum StaticColumn<const R: usize> {
    I64 { name: &'static str, data: [i64; R] },
    F64 { name: &'static str, data: [f64; R] },
}

impl<const R: usize> StaticColumn<R> {
    pub fn name(&self) -> &'static str {
        match self {
            StaticColumn::I64 { name, .. } => name,
            StaticColumn::F64 { name, .. } => name,
        }
    }

    pub fn kind(&self) -> Kind {
        match self {
            StaticColumn::I64 { .. } => Kind::I64,
            StaticColumn::F64 { .. } => Kind::F64,
        }
    }

    pub fn data(&self) -> ColumnData<'_> {
        match self {
            StaticColumn::I64 { data, .. } => ColumnData::I64(data.as_slice()),
            StaticColumn::F64 { data, .. } => ColumnData::F64(data.as_slice()),
        }
    }
}

/// Static impl: column set and row count known at compile time.
#[derive(Clone, Copy, Debug)]
pub struct StaticSheet<const C: usize, const R: usize> {
    pub name: &'static str,
    pub columns: [StaticColumn<R>; C],
}

impl<const C: usize, const R: usize> StaticSheet<C, R> {
    pub const fn new(name: &'static str, columns: [StaticColumn<R>; C]) -> Self {
        StaticSheet { name, columns }
    }
}

impl<const C: usize, const R: usize> SheetTemplate for StaticSheet<C, R> {
    fn sheet_name(&self) -> &str {
        self.name
    }

    fn shape(&self) -> ShapeKnowledge {
        ShapeKnowledge::known(R, C)
    }

    fn columns_view(&self) -> Vec<ColumnView<'_>> {
        self.columns
            .iter()
            .map(|col| ColumnView {
                name: col.name(),
                kind: col.kind(),
                data: col.data(),
            })
            .collect()
    }

    fn column_count(&self) -> usize {
        C
    }
}

/// Column letters, one per column, in declaration order.
pub fn sheet_letters<T: SheetTemplate>(template: &T) -> Vec<String> {
    (0..template.column_count()).map(col_letters).collect()
}

/// Boundary check before any emission: non-empty, within Excel's
/// limits, rectangular, declared kinds matching their data.
/// Returns the data row count.
pub fn validate_template<T: SheetTemplate>(template: &T) -> Result<usize, XlsxError> {
    let views: Vec<ColumnView<'_>> = template.columns_view();
    if views.is_empty() {
        return Err(XlsxError::EmptyColumns);
    }
    for view in &views {
        let actual: Kind = view.data.kind();
        if view.kind != actual {
            return Err(XlsxError::KindMismatch {
                name: view.name.to_string(),
                declared: view.kind,
                actual,
            });
        }
    }
    if views.len() > MAX_COLS {
        return Err(XlsxError::TooManyColumns(views.len()));
    }
    let nrows: usize = views[0].data.len();
    for view in &views[1..] {
        let len: usize = view.data.len();
        if len != nrows {
            return Err(XlsxError::Ragged {
                name: view.name.to_string(),
                len,
                expected: nrows,
            });
        }
    }
    if nrows + 1 > MAX_ROWS {
        return Err(XlsxError::TooManyRows(nrows));
    }
    Ok(nrows)
}

/// Worst-case sheet bytes (20 per `I64`, 327 per `F64`): the
/// ceiling proof, never the reservation.
pub fn sheet_cap<T: SheetTemplate>(template: &T, nrows: usize, letters: &[String]) -> usize {
    let cell_tags: usize = "<c r=\"".len() + "\"><v>".len() + "</v></c>".len();
    let mut header: usize = 256 + 12;
    let mut row: usize = 16 + 7;
    let views: Vec<ColumnView<'_>> = template.columns_view();
    for (view, letter) in views.iter().zip(letters.iter()) {
        let widest: usize = match view.data.kind() {
            Kind::I64 => 20,
            Kind::F64 => 327,
        };
        header += letter.len() + view.name.len() + 64;
        row += letter.len() + 7 + cell_tags + widest;
    }
    header + nrows * row
}

/// Arena reservation hint from the declared shape (expected
/// widths 10 `I64`, 12 `F64`); `None` when a dimension is unknown.
pub fn sheet_hint<T: SheetTemplate>(template: &T, letters: &[String]) -> Option<usize> {
    let nrows: usize = template.shape().rows()?;
    template.shape().columns()?;
    let mut header: usize = 256 + 12;
    let mut row: usize = 16 + 7;
    let views: Vec<ColumnView<'_>> = template.columns_view();
    for (view, letter) in views.iter().zip(letters.iter()) {
        let expected: usize = match view.data.kind() {
            Kind::I64 => 10,
            Kind::F64 => 12,
        };
        header += letter.len() + view.name.len() + 64;
        row += letter.len() + 7 + 20 + expected;
    }
    Some(header + nrows * row)
}

/// Emit one sheet's XML (the oracle spelling). Assumes
/// `validate_template` passed; only an infinite float can refuse.
pub fn emit_sheet<T: SheetTemplate>(template: &T, nrows: usize) -> Result<Vec<u8>, XlsxError> {
    let letters: Vec<String> = sheet_letters(template);
    let mut out = String::with_capacity(sheet_cap(template, nrows, &letters));
    out.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    out.push_str(
        r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>"#,
    );
    let views: Vec<ColumnView<'_>> = template.columns_view();
    out.push_str("<row r=\"1\">");
    for (column, view) in views.iter().enumerate() {
        header_cell(&mut out, column, view.name);
    }
    out.push_str("</row>");
    let mut row_digits = String::new();
    for r in 0..nrows {
        let row1: usize = r + 2;
        row_digits.clear();
        push_usize(&mut row_digits, row1);
        out.push_str("<row r=\"");
        out.push_str(&row_digits);
        out.push_str("\">");
        for (view, letter) in views.iter().zip(letters.iter()) {
            match view.data {
                ColumnData::I64(v) => i64_cell(&mut out, letter, &row_digits, v[r]),
                ColumnData::F64(v) => {
                    let x: f64 = v[r];
                    if x.is_infinite() {
                        return Err(XlsxError::Infinite {
                            column: view.name.to_string(),
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

/// Emit one sheet into the arena, returning its CRC, folded
/// once per row over that row's range.
pub fn emit_sheet_arena<T: SheetTemplate>(
    template: &T,
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
    let views: Vec<ColumnView<'_>> = template.columns_view();
    out.extend_from_slice(b"<row r=\"1\">");
    for (column, view) in views.iter().enumerate() {
        header_cell_bytes(out, column, view.name);
    }
    out.extend_from_slice(b"</row>");
    crc_state = crc32_update(crc_state, &out[chunk_start..]);
    chunk_start = out.len();
    let mut row_digits: Vec<u8> = Vec::new();
    for r in 0..nrows {
        let row1: usize = r + 2;
        row_digits.clear();
        push_usize_bytes(&mut row_digits, row1);
        out.extend_from_slice(b"<row r=\"");
        out.extend_from_slice(&row_digits);
        out.extend_from_slice(b"\">");
        for (view, letter) in views.iter().zip(letters.iter()) {
            match view.data {
                ColumnData::I64(v) => i64_cell_bytes(out, letter, &row_digits, v[r]),
                ColumnData::F64(v) => {
                    let x: f64 = v[r];
                    if x.is_infinite() {
                        return Err(XlsxError::Infinite {
                            column: view.name.to_string(),
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
