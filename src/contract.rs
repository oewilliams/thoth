//! The template contract: a template declares its columns, their
//! data, and its shape knowledge once; the drivers below consume
//! that declaration without re-deriving shape. Impls are
//! monomorphised; the row loop never branches on impl.
//!
//! STYLE rule 1 tripwire: this file passes 700 lines in cycle 018
//! because `Colour`/`Style` are declared beside `Kind`/`Column` —
//! style is gathered once per sheet with kind, and splitting the
//! declaration from its gathering would scatter one fact. Cycle
//! 020 grew it further (951 lines) with the witnessed scan.

use crate::emit::{
    bool_cell, bool_cell_bytes, col_letters, f64_cell, f64_cell_bytes, header_cell,
    header_cell_bytes, i64_cell, i64_cell_bytes, push_usize, push_usize_bytes, str_cell,
    str_cell_bytes, swar_count_byte, swar_word, swar_word_is_plain_ascii,
};
use crate::xlsx::{MAX_COLS, MAX_ROWS, XlsxError};
use crate::zip::{CRC_INIT, crc32_finish, crc32_update};

/// Column kind: the one decision made per column, never per cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    I64,
    F64,
    Str,
    Bool,
}

/// Fill colour from the fixed palette, in palette order. Colour is
/// the fill axis; bold is the font axis; the two combine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Colour {
    Red,
    Green,
    Amber,
    Blue,
    Purple,
    Grey,
}

impl Colour {
    /// 0-based palette position, the arithmetic basis of the
    /// `cellXfs` index in `xl/styles.xml`.
    pub fn index(self) -> usize {
        match self {
            Colour::Red => 0,
            Colour::Green => 1,
            Colour::Amber => 2,
            Colour::Blue => 3,
            Colour::Purple => 4,
            Colour::Grey => 5,
        }
    }
}

/// Per-column style, declared once like kind: a bold flag plus an
/// optional fill colour. Plain is `Style::default()`. Every value
/// is representable in the styles part by construction, so style
/// needs no validation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Style {
    pub bold: bool,
    pub colour: Option<Colour>,
}

impl Style {
    /// The `cellXfs` index for this style, or `None` for plain
    /// (no `s` attribute): bold only is 1, colour `i` (0-based)
    /// is `2 + 2i`, colour `i` bold is `3 + 2i`.
    pub fn index(self) -> Option<usize> {
        match (self.bold, self.colour) {
            (false, None) => None,
            (true, None) => Some(1),
            (false, Some(colour)) => Some(2 + 2 * colour.index()),
            (true, Some(colour)) => Some(3 + 2 * colour.index()),
        }
    }
}

/// Pre-spelled `s` attribute fragment for one column (` s="5"`,
/// empty for plain), selected once per sheet per column from the
/// fourteen spellings the styles part admits.
pub fn style_fragment(style: Style) -> &'static str {
    match (style.bold, style.colour) {
        (false, None) => "",
        (true, None) => " s=\"1\"",
        (false, Some(Colour::Red)) => " s=\"2\"",
        (true, Some(Colour::Red)) => " s=\"3\"",
        (false, Some(Colour::Green)) => " s=\"4\"",
        (true, Some(Colour::Green)) => " s=\"5\"",
        (false, Some(Colour::Amber)) => " s=\"6\"",
        (true, Some(Colour::Amber)) => " s=\"7\"",
        (false, Some(Colour::Blue)) => " s=\"8\"",
        (true, Some(Colour::Blue)) => " s=\"9\"",
        (false, Some(Colour::Purple)) => " s=\"10\"",
        (true, Some(Colour::Purple)) => " s=\"11\"",
        (false, Some(Colour::Grey)) => " s=\"12\"",
        (true, Some(Colour::Grey)) => " s=\"13\"",
    }
}

/// One declared column: Excel order is declaration order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Column {
    pub name: String,
    pub kind: Kind,
    pub style: Style,
}

/// The i64 null: absence lives in the value's own bits.
pub const NULL_I64: i64 = i64::MIN;

/// One column's payload: one pointer plus one length.
#[derive(Clone, Copy, Debug)]
pub enum ColumnData<'a> {
    I64(&'a [i64]),
    F64(&'a [f64]),
    Str(&'a [Option<&'a str>]),
    Bool(&'a [Option<bool>]),
}

impl<'a> ColumnData<'a> {
    pub fn len(&self) -> usize {
        match self {
            ColumnData::I64(v) => v.len(),
            ColumnData::F64(v) => v.len(),
            ColumnData::Str(v) => v.len(),
            ColumnData::Bool(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn kind(&self) -> Kind {
        match self {
            ColumnData::I64(_) => Kind::I64,
            ColumnData::F64(_) => Kind::F64,
            ColumnData::Str(_) => Kind::Str,
            ColumnData::Bool(_) => Kind::Bool,
        }
    }

    pub fn from_static_i64<const N: usize>(data: &'a [i64; N]) -> Self {
        ColumnData::I64(data.as_slice())
    }

    pub fn from_static_f64<const N: usize>(data: &'a [f64; N]) -> Self {
        ColumnData::F64(data.as_slice())
    }

    pub fn from_static_str<const N: usize>(data: &'a [Option<&'a str>; N]) -> Self {
        ColumnData::Str(data.as_slice())
    }

    pub fn from_static_bool<const N: usize>(data: &'a [Option<bool>; N]) -> Self {
        ColumnData::Bool(data.as_slice())
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

/// One column for the drivers: name, declared kind, style, data.
/// Gathered once per sheet before the row loop.
#[derive(Clone, Copy, Debug)]
pub struct ColumnView<'a> {
    pub name: &'a str,
    pub kind: Kind,
    pub style: Style,
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
                style: col.style,
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
    I64 {
        name: &'static str,
        style: Style,
        data: [i64; R],
    },
    F64 {
        name: &'static str,
        style: Style,
        data: [f64; R],
    },
    Str {
        name: &'static str,
        style: Style,
        data: [Option<&'static str>; R],
    },
    Bool {
        name: &'static str,
        style: Style,
        data: [Option<bool>; R],
    },
}

impl<const R: usize> StaticColumn<R> {
    pub fn name(&self) -> &'static str {
        match self {
            StaticColumn::I64 { name, .. } => name,
            StaticColumn::F64 { name, .. } => name,
            StaticColumn::Str { name, .. } => name,
            StaticColumn::Bool { name, .. } => name,
        }
    }

    pub fn kind(&self) -> Kind {
        match self {
            StaticColumn::I64 { .. } => Kind::I64,
            StaticColumn::F64 { .. } => Kind::F64,
            StaticColumn::Str { .. } => Kind::Str,
            StaticColumn::Bool { .. } => Kind::Bool,
        }
    }

    pub fn style(&self) -> Style {
        match self {
            StaticColumn::I64 { style, .. } => *style,
            StaticColumn::F64 { style, .. } => *style,
            StaticColumn::Str { style, .. } => *style,
            StaticColumn::Bool { style, .. } => *style,
        }
    }

    pub fn data(&self) -> ColumnData<'_> {
        match self {
            StaticColumn::I64 { data, .. } => ColumnData::I64(data.as_slice()),
            StaticColumn::F64 { data, .. } => ColumnData::F64(data.as_slice()),
            StaticColumn::Str { data, .. } => ColumnData::Str(data.as_slice()),
            StaticColumn::Bool { data, .. } => ColumnData::Bool(data.as_slice()),
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
                style: col.style(),
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
/// limits, rectangular, declared kinds matching their data, and
/// no infinite floats. Returns the data row count.
pub fn validate_template<T: SheetTemplate>(template: &T) -> Result<usize, XlsxError> {
    let (nrows, _escaped): (usize, usize) = validate_template_witnessed(template)?;
    Ok(nrows)
}

/// The boundary check plus its witnessed escaped byte total:
/// the exact escaped length of every string cell's payload,
/// accumulated during the text scan that already visits those
/// chars (never a second pass), for the arena reservation in
/// [`sheet_hint_witnessed`]. On a refusal the total is
/// discarded and unused: no reservation is taken from a
/// refused sheet.
pub fn validate_template_witnessed<T: SheetTemplate>(
    template: &T,
) -> Result<(usize, usize), XlsxError> {
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
    // Values are checked here, at the boundary; past it the data
    // is bits. Each float column is scanned contiguously (benched
    // faster than the row-major stride at every shape); the first
    // infinite in row-major order is rebuilt from the per-column
    // firsts, earliest column winning a tied row.
    let mut first: Option<(usize, usize)> = None;
    for (c, view) in views.iter().enumerate() {
        if let ColumnData::F64(v) = view.data
            && let Some(r) = v.iter().position(|x| x.is_infinite())
            && first.is_none_or(|(fr, _)| r < fr)
        {
            first = Some((r, c));
        }
    }
    if let Some((r, c)) = first {
        return Err(XlsxError::Infinite {
            column: views[c].name.to_string(),
            row: r + 2,
        });
    }
    let (text_err, escaped): (Option<XlsxError>, usize) = scan_text_cells(&views);
    if let Some(err) = text_err {
        return Err(err);
    }
    Ok((nrows, escaped))
}

/// First string refusal in row-major order, or `None`: Excel's
/// 32,767-char cap and the XML 1.0 valid range, checked per column
/// contiguously like the float scan in `validate_template`; the
/// first refusal in row-major order is rebuilt from the per-column
/// firsts, earliest column winning a tied row. The valid set is
/// spelled out here where the check lives: tab (0x09), LF (0x0A),
/// and CR (0x0D) — the whitespace Excel keeps — plus
/// 0x20..=0xD7FF, 0xE000..=0xFFFD, and 0x10000..=0x10FFFF.
/// Everything else (other controls below 0x20, 0xFFFE, 0xFFFF)
/// is refused. `char` in a `&str` is already a valid Unicode
/// scalar, so surrogates and values past 0x10FFFF cannot occur.
fn scan_text_cells(views: &[ColumnView<'_>]) -> (Option<XlsxError>, usize) {
    let mut text_first: Option<(usize, usize, XlsxError)> = None;
    let mut escaped_total: usize = 0;
    for (c, view) in views.iter().enumerate() {
        let ColumnData::Str(v) = view.data else {
            continue;
        };
        for (r, cell) in v.iter().enumerate() {
            let Some(text) = cell else { continue };
            let (len, bad, escaped): (usize, Option<char>, usize) = scan_one_text(text);
            escaped_total += escaped;
            let err: Option<XlsxError> = if len > 32_767 {
                Some(XlsxError::TextTooLong {
                    column: view.name.to_string(),
                    row: r + 2,
                    len,
                })
            } else {
                bad.map(|ch| XlsxError::InvalidTextChar {
                    column: view.name.to_string(),
                    row: r + 2,
                    ch,
                })
            };
            if let Some(err) = err {
                let earlier: bool = text_first.as_ref().is_none_or(|(fr, _, _)| r < *fr);
                if earlier {
                    text_first = Some((r, c, err));
                }
                break;
            }
        }
    }
    (text_first.map(|(_, _, err)| err), escaped_total)
}

/// One string cell under the boundary scan: its char count, its
/// first invalid char, and its exact escaped byte length (the
/// E1 witness, accumulated here where the chars are already
/// visited). Plain-ASCII words ride the SWAR path (eight
/// one-byte valid chars; escapable lanes counted in SWAR for
/// the escaped length); every other char takes the spelled-out
/// valid-range check above, and its escaped length is its
/// entity length or its UTF-8 length.
fn scan_one_text(text: &str) -> (usize, Option<char>, usize) {
    let bytes: &[u8] = text.as_bytes();
    let mut len: usize = 0;
    let mut bad: Option<char> = None;
    let mut escaped: usize = 0;
    let mut i: usize = 0;
    while i < bytes.len() {
        // Plain-ASCII words need no per-char walk at all: eight
        // one-byte valid chars, and the escaped length is eight
        // plus each escapable lane's entity growth (`&` +4,
        // `<`/`>` +3, `"`/`'` +5), counted per lane in SWAR.
        if let Some(word) = swar_word(bytes, i)
            && swar_word_is_plain_ascii(word)
        {
            len += 8;
            escaped += 8
                + 4 * swar_count_byte(word, b'&')
                + 3 * (swar_count_byte(word, b'<') + swar_count_byte(word, b'>'))
                + 5 * (swar_count_byte(word, b'"') + swar_count_byte(word, b'\''));
            i += 8;
            continue;
        }
        // A word the fast path refuses (or the short tail) is
        // walked char by char exactly once, never retested.
        let end: usize = usize::min(i + 8, bytes.len());
        while i < end {
            // One-byte chars spell themselves: valid exactly when
            // tab, LF, CR, or at least 0x20 (0x7F included, inside
            // 0x20..=0xD7FF), with the entity length on the byte.
            if bytes[i] < 0x80 {
                let byte: u8 = bytes[i];
                len += 1;
                escaped += match byte {
                    b'&' => 5,
                    b'<' | b'>' => 4,
                    b'"' | b'\'' => 6,
                    _ => 1,
                };
                if bad.is_none() && !(byte == 0x09 || byte == 0x0A || byte == 0x0D || byte >= 0x20)
                {
                    bad = Some(char::from(byte));
                }
                i += 1;
                continue;
            }
            let Some(ch) = text[i..].chars().next() else {
                break;
            };
            // (multi-byte char: spelled-out range check below)
            len += 1;
            escaped += match ch {
                '&' => 5,
                '<' | '>' => 4,
                '"' | '\'' => 6,
                _ => ch.len_utf8(),
            };
            if bad.is_none() {
                let u: u32 = u32::from(ch);
                let valid: bool = u == 0x09
                    || u == 0x0A
                    || u == 0x0D
                    || (0x20..=0xD7FF).contains(&u)
                    || (0xE000..=0xFFFD).contains(&u)
                    || (0x10000..=0x10FFFF).contains(&u);
                if !valid {
                    bad = Some(ch);
                }
            }
            i += ch.len_utf8();
        }
    }
    (len, bad, escaped)
}

/// Worst-case sheet bytes (20 per `I64`, 327 per `F64`, the fixed
/// bool spelling, and per string cell the escaped upper bound of
/// the actual text at 6x per char plus tags; header names priced
/// at the same escaped bound): the oracle's `String` capacity in
/// `emit_sheet` and the `sheet_cap_holds` proof that the actual
/// never exceeds it. Never the reservation and never a refusal:
/// the ceiling verdict is taken on witnessed lengths at the
/// narrowing casts in `crate::xlsx`.
pub fn sheet_cap<T: SheetTemplate>(template: &T, nrows: usize, letters: &[String]) -> usize {
    let cell_tags: usize = "<c r=\"".len() + "\"><v>".len() + "</v></c>".len();
    let mut cap: usize = 256 + 12;
    let views: Vec<ColumnView<'_>> = template.columns_view();
    for (view, letter) in views.iter().zip(letters.iter()) {
        // Header names escape through the same five-entity table
        // as string cells, so price the escaped bound (6x per
        // char, the widest `&quot;`/`&apos;` spelling) plus the
        // fixed header tags the 64 covers.
        cap += letter.len() + 6 * view.name.chars().count() + 64;
    }
    cap += nrows * (16 + 7);
    for (view, letter) in views.iter().zip(letters.iter()) {
        // Widest `s` spelling (` s="13"`, seven bytes with the
        // leading space) per data cell in a styled column, blanks
        // included; plain columns price nothing.
        let style_widest: usize = if view.style.index().is_some() { 7 } else { 0 };
        match view.data {
            ColumnData::Str(v) => {
                // Per cell: `<c r="` + letter + row digits (<=7) +
                // `" t="inlineStr">` + `<is><t xml:space="preserve">`
                // + escaped text (<=6 bytes per char, the widest
                // five-entity spelling) + `</t></is></c>`; blank is
                // `<c r="` + letter + digits + `"/>`.
                for cell in v.iter() {
                    match cell {
                        Some(text) => {
                            cap += letter.len()
                                + 7
                                + style_widest
                                + 6
                                + 16
                                + 28
                                + 13
                                + 6 * text.chars().count();
                        }
                        None => {
                            cap += letter.len() + 7 + style_widest + 6 + 3;
                        }
                    }
                }
            }
            ColumnData::I64(_) | ColumnData::F64(_) | ColumnData::Bool(_) => {
                let widest: usize = match view.data.kind() {
                    Kind::I64 => 20,
                    Kind::F64 => 327,
                    // Fixed spelling: ` t="b"` (6) plus one digit.
                    Kind::Bool => 7,
                    Kind::Str => 0,
                };
                cap += nrows * (letter.len() + 7 + style_widest + cell_tags + widest);
            }
        }
    }
    cap
}

/// Arena reservation hint from the declared shape (expected
/// widths 10 `I64`, 12 `F64`; strings priced from witnessed
/// lengths — text bytes plus tags); `None` when a dimension
/// is unknown.
pub fn sheet_hint<T: SheetTemplate>(template: &T, letters: &[String]) -> Option<usize> {
    sheet_hint_inner(template, letters, None)
}

/// The reservation with the string payload priced at its
/// witnessed escaped length (from
/// [`validate_template_witnessed`]) in place of the raw text
/// bytes: escapable-heavy sheets reserve their actual size
/// instead of growing into it. Numeric kinds keep their
/// expected widths: no boundary scan visits their spelling,
/// and witnessing one would add the pass this spelling rides.
/// `None` when a dimension is unknown, like [`sheet_hint`].
pub fn sheet_hint_witnessed<T: SheetTemplate>(
    template: &T,
    letters: &[String],
    escaped_total: usize,
) -> Option<usize> {
    sheet_hint_inner(template, letters, Some(escaped_total))
}

fn sheet_hint_inner<T: SheetTemplate>(
    template: &T,
    letters: &[String],
    witnessed: Option<usize>,
) -> Option<usize> {
    let nrows: usize = template.shape().rows()?;
    template.shape().columns()?;
    let views: Vec<ColumnView<'_>> = template.columns_view();
    let mut hint: usize = 256 + 12;
    for (view, letter) in views.iter().zip(letters.iter()) {
        hint += letter.len() + view.name.len() + 64;
    }
    hint += nrows * (16 + 7);
    for (view, letter) in views.iter().zip(letters.iter()) {
        // The column's exact `s` spelling per data cell, so styled
        // sheets reserve near their actual size.
        let style_len: usize = style_fragment(view.style).len();
        match view.data {
            ColumnData::Str(v) => {
                // Witnessed lengths: actual text bytes plus the
                // inline-string tags, per cell, so string sheets
                // reserve near their actual size. With a witnessed
                // escaped total the payload bytes arrive as that
                // one total instead of the raw per-cell lengths.
                for cell in v.iter() {
                    match cell {
                        Some(text) => {
                            hint += letter.len() + 7 + style_len + 6 + 16 + 7 + 13;
                            if witnessed.is_none() {
                                hint += text.len();
                            }
                        }
                        None => {
                            hint += letter.len() + 7 + style_len + 6 + 3;
                        }
                    }
                }
            }
            ColumnData::I64(_) | ColumnData::F64(_) | ColumnData::Bool(_) => {
                let expected: usize = match view.data.kind() {
                    Kind::I64 => 10,
                    Kind::F64 => 12,
                    // Fixed bool spelling past the shared tag allowance.
                    Kind::Bool => 6,
                    Kind::Str => 0,
                };
                hint += nrows * (letter.len() + 7 + style_len + 20 + expected);
            }
        }
    }
    if let Some(escaped_total) = witnessed {
        hint += escaped_total;
    }
    Some(hint)
}

/// Emit one sheet's XML (the oracle spelling). Assumes
/// `validate_template` passed; writes bits, checks nothing.
pub fn emit_sheet<T: SheetTemplate>(template: &T, nrows: usize) -> Result<Vec<u8>, XlsxError> {
    let letters: Vec<String> = sheet_letters(template);
    let mut out = String::with_capacity(sheet_cap(template, nrows, &letters));
    out.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    out.push_str(
        r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>"#,
    );
    let views: Vec<ColumnView<'_>> = template.columns_view();
    let gathered: Vec<(&ColumnView<'_>, &String, &'static str)> = views
        .iter()
        .zip(letters.iter())
        .map(|(view, letter)| (view, letter, style_fragment(view.style)))
        .collect();
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
        for (view, letter, fragment) in &gathered {
            match view.data {
                ColumnData::I64(v) => i64_cell(&mut out, letter, &row_digits, fragment, v[r]),
                ColumnData::F64(v) => f64_cell(&mut out, letter, &row_digits, fragment, v[r]),
                ColumnData::Str(v) => str_cell(&mut out, letter, &row_digits, fragment, v[r]),
                ColumnData::Bool(v) => bool_cell(&mut out, letter, &row_digits, fragment, v[r]),
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
    let gathered: Vec<(&ColumnView<'_>, &String, &'static str)> = views
        .iter()
        .zip(letters.iter())
        .map(|(view, letter)| (view, letter, style_fragment(view.style)))
        .collect();
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
        for (view, letter, fragment) in &gathered {
            match view.data {
                ColumnData::I64(v) => i64_cell_bytes(out, letter, &row_digits, fragment, v[r]),
                ColumnData::F64(v) => f64_cell_bytes(out, letter, &row_digits, fragment, v[r]),
                ColumnData::Str(v) => str_cell_bytes(out, letter, &row_digits, fragment, v[r]),
                ColumnData::Bool(v) => bool_cell_bytes(out, letter, &row_digits, fragment, v[r]),
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

#[cfg(test)]
mod tests {
    use super::{Column, ColumnData, DynamicSheet, Kind, Style, validate_template_witnessed};
    use crate::xlsx::XlsxError;

    #[test]
    fn witnessed_escaped_total_is_exact() {
        let plain: String = "abcdefgh".repeat(10);
        let cells: [Option<&str>; 3] = [Some("a&<b>"), Some(plain.as_str()), None];
        let cols: Vec<(Column, ColumnData<'_>)> = vec![(
            Column {
                name: "s".to_string(),
                kind: Kind::Str,
                style: Style::default(),
            },
            ColumnData::Str(&cells),
        )];
        let sheet: DynamicSheet<'_> = DynamicSheet::known("data".to_string(), &cols);
        let (nrows, escaped): (usize, usize) =
            validate_template_witnessed(&sheet).expect("valid sheet");
        assert_eq!(nrows, 3);
        // "a&<b>" escapes to 1 + 5 + 4 + 1 + 4 = 15; the plain
        // 80 chars witness 80; blank witnesses nothing.
        assert_eq!(escaped, 15 + 80);
        // Adjacent `target ^ 1` pairs must not double-count:
        // each word below is one full plain-ASCII word, and
        // its witnessed length is the per-char escaped length.
        for (text, want) in [
            ("&'&'&'&'", 44usize),
            ("'&'&'&'&", 44),
            ("&<>\"'&<>", 38),
            ("aaa&'aaa", 17),
            ("<=<=<=<=", 20),
        ] {
            let cells: [Option<&str>; 1] = [Some(text)];
            let cols: Vec<(Column, ColumnData<'_>)> = vec![(
                Column {
                    name: "s".to_string(),
                    kind: Kind::Str,
                    style: Style::default(),
                },
                ColumnData::Str(&cells),
            )];
            let sheet: DynamicSheet<'_> = DynamicSheet::known("data".to_string(), &cols);
            let (_, got): (usize, usize) =
                validate_template_witnessed(&sheet).expect("valid sheet");
            assert_eq!(got, want, "witness for {text:?}");
        }
    }

    #[test]
    fn swar_scan_still_refuses_past_plain_words() {
        let mut text: String = "abcdefgh".repeat(20);
        text.push('\u{1}');
        let cells: [Option<&str>; 1] = [Some(text.as_str())];
        let cols: Vec<(Column, ColumnData<'_>)> = vec![(
            Column {
                name: "s".to_string(),
                kind: Kind::Str,
                style: Style::default(),
            },
            ColumnData::Str(&cells),
        )];
        let sheet: DynamicSheet<'_> = DynamicSheet::known("data".to_string(), &cols);
        match validate_template_witnessed(&sheet) {
            Err(XlsxError::InvalidTextChar { row, .. }) => assert_eq!(row, 2),
            other => panic!("want InvalidTextChar, got {other:?}"),
        }
        let long: String = "abcdefgh".repeat(5000);
        let cells: [Option<&str>; 1] = [Some(long.as_str())];
        let cols: Vec<(Column, ColumnData<'_>)> = vec![(
            Column {
                name: "s".to_string(),
                kind: Kind::Str,
                style: Style::default(),
            },
            ColumnData::Str(&cells),
        )];
        let sheet: DynamicSheet<'_> = DynamicSheet::known("data".to_string(), &cols);
        match validate_template_witnessed(&sheet) {
            Err(XlsxError::TextTooLong { len, .. }) => assert_eq!(len, 40_000),
            other => panic!("want TextTooLong, got {other:?}"),
        }
    }
}
