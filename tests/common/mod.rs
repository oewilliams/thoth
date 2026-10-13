//! Shared example store for tests and benches.
//!
//! Each example owns its data and exposes a borrowed column view for
//! the writer plus the values a reader must observe. Round-trip
//! examples carry row-major expectations (`None` is blank); negative
//! examples carry the error substring they must fail with and never
//! reach a file.

use thoth::contract::{Colour, Column, DynamicSheet, Kind, Style};
use thoth::xlsx::ColumnData;

/// One read-back cell value, by kind.
#[derive(Clone, Debug, PartialEq)]
pub enum CellValue {
    Num(f64),
    Bool(bool),
    Text(String),
}

/// One column of example data, in Excel order.
pub enum ExampleColumn {
    I64 {
        name: String,
        style: Style,
        data: Vec<i64>,
    },
    F64 {
        name: String,
        style: Style,
        data: Vec<f64>,
    },
    Str {
        name: String,
        style: Style,
        data: Vec<Option<&'static str>>,
    },
    Bool {
        name: String,
        style: Style,
        data: Vec<Option<bool>>,
    },
}

/// One example of a unit of functionality.
pub struct Example {
    pub name: &'static str,
    pub columns: Vec<ExampleColumn>,
    /// Expected cells row-major, header row excluded. Empty when
    /// [`Example::expect_error`] is set.
    pub expected: Vec<Vec<Option<CellValue>>>,
    /// Substring the writer error must contain. `None` round-trips.
    pub expect_error: Option<&'static str>,
}

impl Example {
    /// Dynamic template for this example, named `name`: the
    /// contract spelling every test and bench routes through.
    /// Borrows the column view built by [`Example::writer_columns`];
    /// the caller keeps that view alive for the template's life.
    pub fn dynamic_sheet<'a>(name: &str, cols: &'a [(Column, ColumnData<'a>)]) -> DynamicSheet<'a> {
        DynamicSheet::known(name.to_string(), cols)
    }

    /// Borrowed column view for the writer.
    pub fn writer_columns(&self) -> Vec<(Column, ColumnData<'_>)> {
        self.columns
            .iter()
            .map(|c| match c {
                ExampleColumn::I64 { name, style, data } => (
                    Column {
                        name: name.clone(),
                        kind: Kind::I64,
                        style: *style,
                    },
                    ColumnData::I64(data),
                ),
                ExampleColumn::F64 { name, style, data } => (
                    Column {
                        name: name.clone(),
                        kind: Kind::F64,
                        style: *style,
                    },
                    ColumnData::F64(data),
                ),
                ExampleColumn::Str { name, style, data } => (
                    Column {
                        name: name.clone(),
                        kind: Kind::Str,
                        style: *style,
                    },
                    ColumnData::Str(data),
                ),
                ExampleColumn::Bool { name, style, data } => (
                    Column {
                        name: name.clone(),
                        kind: Kind::Bool,
                        style: *style,
                    },
                    ColumnData::Bool(data),
                ),
            })
            .collect()
    }
}

/// Mixed integers and floats with headers.
pub fn ints_floats() -> Example {
    Example {
        name: "ints_floats",
        columns: vec![
            ExampleColumn::I64 {
                name: "t".to_string(),
                style: Style::default(),
                data: vec![100, 200],
            },
            ExampleColumn::F64 {
                name: "px".to_string(),
                style: Style::default(),
                data: vec![10.5, 11.0],
            },
        ],
        expected: vec![
            vec![Some(CellValue::Num(100.0)), Some(CellValue::Num(10.5))],
            vec![Some(CellValue::Num(200.0)), Some(CellValue::Num(11.0))],
        ],
        expect_error: None,
    }
}

/// Nulls read back blank, never as sentinels.
pub fn nulls_blank() -> Example {
    Example {
        name: "nulls_blank",
        columns: vec![
            ExampleColumn::I64 {
                name: "t".to_string(),
                style: Style::default(),
                data: vec![100, thoth::contract::NULL_I64],
            },
            ExampleColumn::F64 {
                name: "px".to_string(),
                style: Style::default(),
                data: vec![f64::NAN, 11.0],
            },
        ],
        expected: vec![
            vec![Some(CellValue::Num(100.0)), None],
            vec![None, Some(CellValue::Num(11.0))],
        ],
        expect_error: None,
    }
}

/// Zero rows: the sheet is headers alone.
pub fn header_only() -> Example {
    Example {
        name: "header_only",
        columns: vec![
            ExampleColumn::I64 {
                name: "t".to_string(),
                style: Style::default(),
                data: vec![],
            },
            ExampleColumn::F64 {
                name: "px".to_string(),
                style: Style::default(),
                data: vec![],
            },
        ],
        expected: vec![],
        expect_error: None,
    }
}

/// One column entirely blank.
pub fn all_null_column() -> Example {
    Example {
        name: "all_null_column",
        columns: vec![
            ExampleColumn::I64 {
                name: "t".to_string(),
                style: Style::default(),
                data: vec![thoth::contract::NULL_I64, thoth::contract::NULL_I64],
            },
            ExampleColumn::F64 {
                name: "px".to_string(),
                style: Style::default(),
                data: vec![1.0, 2.0],
            },
        ],
        expected: vec![
            vec![None, Some(CellValue::Num(1.0))],
            vec![None, Some(CellValue::Num(2.0))],
        ],
        expect_error: None,
    }
}

/// Many columns, few rows: exercises column letters past `Z`.
pub fn wide() -> Example {
    let columns: Vec<ExampleColumn> = (0..30)
        .map(|i| ExampleColumn::I64 {
            name: format!("c{i}"),
            style: Style::default(),
            data: vec![i as i64, -(i as i64)],
        })
        .collect();
    let expected: Vec<Vec<Option<CellValue>>> = [0, 1]
        .iter()
        .map(|&r| {
            (0..30)
                .map(|i| Some(CellValue::Num(if r == 0 { i as f64 } else { -(i as f64) })))
                .collect()
        })
        .collect();
    Example {
        name: "wide",
        columns,
        expected,
        expect_error: None,
    }
}

/// Many rows. `n` is small in tests and scaled up in benches.
pub fn tall(n: usize) -> Example {
    let t: Vec<i64> = (0..n as i64).collect();
    let px: Vec<f64> = (0..n).map(|i| i as f64 + 0.5).collect();
    let expected: Vec<Vec<Option<CellValue>>> = (0..n)
        .map(|i| {
            vec![
                Some(CellValue::Num(i as f64)),
                Some(CellValue::Num(i as f64 + 0.5)),
            ]
        })
        .collect();
    Example {
        name: "tall",
        columns: vec![
            ExampleColumn::I64 {
                name: "t".to_string(),
                style: Style::default(),
                data: t,
            },
            ExampleColumn::F64 {
                name: "px".to_string(),
                style: Style::default(),
                data: px,
            },
        ],
        expected,
        expect_error: None,
    }
}

/// Many integer rows. `n` is small in tests and scaled up in benches.
pub fn tall_ints(n: usize) -> Example {
    let t: Vec<i64> = (0..n as i64).collect();
    let expected: Vec<Vec<Option<CellValue>>> = (0..n)
        .map(|i| vec![Some(CellValue::Num(i as f64))])
        .collect();
    Example {
        name: "tall_ints",
        columns: vec![ExampleColumn::I64 {
            name: "t".to_string(),
            style: Style::default(),
            data: t,
        }],
        expected,
        expect_error: None,
    }
}

/// Many float rows. `n` is small in tests and scaled up in benches.
pub fn tall_floats(n: usize) -> Example {
    let px: Vec<f64> = (0..n).map(|i| i as f64 + 0.5).collect();
    let expected: Vec<Vec<Option<CellValue>>> = (0..n)
        .map(|i| vec![Some(CellValue::Num(i as f64 + 0.5))])
        .collect();
    Example {
        name: "tall_floats",
        columns: vec![ExampleColumn::F64 {
            name: "px".to_string(),
            style: Style::default(),
            data: px,
        }],
        expected,
        expect_error: None,
    }
}

/// Negative: unequal lengths stop the run naming the column.
pub fn ragged() -> Example {
    Example {
        name: "ragged",
        columns: vec![
            ExampleColumn::I64 {
                name: "a".to_string(),
                style: Style::default(),
                data: vec![1, 2],
            },
            ExampleColumn::F64 {
                name: "b".to_string(),
                style: Style::default(),
                data: vec![1.0],
            },
        ],
        expected: vec![],
        expect_error: Some("b"),
    }
}

/// Negative: infinities stop the run with position.
pub fn infinite() -> Example {
    Example {
        name: "infinite",
        columns: vec![ExampleColumn::F64 {
            name: "px".to_string(),
            style: Style::default(),
            data: vec![1.0, f64::INFINITY],
        }],
        expected: vec![],
        expect_error: Some("px"),
    }
}

/// Every kind in one sheet: ints, floats, strings (escapable,
/// empty, blank, whitespace-padded), and bools.
pub fn mixed_kinds() -> Example {
    Example {
        name: "mixed_kinds",
        columns: vec![
            ExampleColumn::I64 {
                name: "t".to_string(),
                style: Style::default(),
                data: vec![100, thoth::contract::NULL_I64, 300, 400],
            },
            ExampleColumn::F64 {
                name: "px".to_string(),
                style: Style::default(),
                data: vec![10.5, f64::NAN, 12.25, 0.5],
            },
            ExampleColumn::Str {
                name: "name".to_string(),
                style: Style::default(),
                data: vec![Some("a & <b> \"q\""), Some(""), None, Some(" pad ")],
            },
            ExampleColumn::Bool {
                name: "ok".to_string(),
                style: Style::default(),
                data: vec![Some(true), Some(false), None, Some(true)],
            },
        ],
        expected: vec![
            vec![
                Some(CellValue::Num(100.0)),
                Some(CellValue::Num(10.5)),
                Some(CellValue::Text("a & <b> \"q\"".to_string())),
                Some(CellValue::Bool(true)),
            ],
            vec![
                None,
                None,
                Some(CellValue::Text(String::new())),
                Some(CellValue::Bool(false)),
            ],
            vec![
                Some(CellValue::Num(300.0)),
                Some(CellValue::Num(12.25)),
                None,
                None,
            ],
            vec![
                Some(CellValue::Num(400.0)),
                Some(CellValue::Num(0.5)),
                Some(CellValue::Text(" pad ".to_string())),
                Some(CellValue::Bool(true)),
            ],
        ],
        expect_error: None,
    }
}

/// Bold, colour, and bold-colour columns across mixed kinds
/// (the reserved bold/colour example): styles ride in the column
/// declarations; values round-trip unchanged.
pub fn styled() -> Example {
    Example {
        name: "styled",
        columns: vec![
            ExampleColumn::I64 {
                name: "t".to_string(),
                style: Style {
                    bold: true,
                    colour: None,
                },
                data: vec![100, thoth::contract::NULL_I64, 300],
            },
            ExampleColumn::F64 {
                name: "px".to_string(),
                style: Style {
                    bold: false,
                    colour: Some(Colour::Red),
                },
                data: vec![10.5, f64::NAN, 12.25],
            },
            ExampleColumn::Str {
                name: "name".to_string(),
                style: Style {
                    bold: true,
                    colour: Some(Colour::Green),
                },
                data: vec![Some("a & b"), None, Some("")],
            },
            ExampleColumn::Bool {
                name: "ok".to_string(),
                style: Style {
                    bold: false,
                    colour: Some(Colour::Amber),
                },
                data: vec![Some(true), None, Some(false)],
            },
        ],
        expected: vec![
            vec![
                Some(CellValue::Num(100.0)),
                Some(CellValue::Num(10.5)),
                Some(CellValue::Text("a & b".to_string())),
                Some(CellValue::Bool(true)),
            ],
            vec![None, None, None, None],
            vec![
                Some(CellValue::Num(300.0)),
                Some(CellValue::Num(12.25)),
                Some(CellValue::Text(String::new())),
                Some(CellValue::Bool(false)),
            ],
        ],
        expect_error: None,
    }
}

/// Every example in the store, in a stable order. `tall_n` sizes the
/// tall example for the caller (tests keep it small).
pub fn all(tall_n: usize) -> Vec<Example> {
    vec![
        ints_floats(),
        nulls_blank(),
        header_only(),
        all_null_column(),
        wide(),
        tall(tall_n),
        tall_ints(tall_n),
        tall_floats(tall_n),
        mixed_kinds(),
        styled(),
        ragged(),
        infinite(),
    ]
}

/// Read back data cells from a written `.xlsx` on disk through an
/// independent path: raw stored-zip parsing plus `<v>` text extraction.
/// Header row excluded; `None` is a blank cell.
///
/// This is the untimed verification step. A third-party reader replaces
/// it as oracle when one can be installed; until then this read path
/// (byte parsing, never string building) is what stands between the
/// writer and a false green.
#[allow(dead_code)]
pub fn read_sheet_values(path: &std::path::Path) -> Vec<Vec<Option<f64>>> {
    read_sheet_values_by_index(path, 1)
}

/// Read back data cells from sheet `index` (1-based, declaration
/// order) through the same independent stored-zip path as
/// [`read_sheet_values`], which is the sheet-1 spelling of this.
#[allow(dead_code)]
pub fn read_sheet_values_by_index(path: &std::path::Path, index: usize) -> Vec<Vec<Option<f64>>> {
    read_sheet_cells_by_index(path, index)
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|cell| match cell {
                    None => None,
                    Some(CellValue::Num(v)) => Some(v),
                    Some(other) => panic!("numeric reader saw {other:?}"),
                })
                .collect()
        })
        .collect()
}

/// Read back typed data cells (numbers, bools, strings) from a
/// written `.xlsx` through the independent stored-zip path.
/// Header row excluded; `None` is a blank cell, and an empty
/// inline string reads back as `Some(Text(""))`, never blank.
#[allow(dead_code)]
pub fn read_sheet_cells(path: &std::path::Path) -> Vec<Vec<Option<CellValue>>> {
    read_sheet_cells_by_index(path, 1)
}

/// Sheet-`index` spelling of [`read_sheet_cells`].
#[allow(dead_code)]
pub fn read_sheet_cells_by_index(
    path: &std::path::Path,
    index: usize,
) -> Vec<Vec<Option<CellValue>>> {
    let bytes = std::fs::read(path).unwrap();
    let part: String = format!("xl/worksheets/sheet{index}.xml");
    let sheet = find_stored_entry(&bytes, &part).expect("sheet part missing from zip");
    parse_data_rows(sheet)
}

/// Raw bytes of one stored part (e.g. `xl/workbook.xml`) through the
/// same independent stored-zip walk.
#[allow(dead_code)]
pub fn read_part(path: &std::path::Path, name: &str) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap();
    find_stored_entry(&bytes, name)
        .expect("part missing from zip")
        .to_vec()
}

/// Walk stored local file headers for one entry's bytes. Stored only:
/// anything else is this reader's problem, never the writer's.
fn find_stored_entry<'a>(zip: &'a [u8], name: &str) -> Option<&'a [u8]> {
    let mut at: usize = 0;
    while at + 30 <= zip.len() {
        let sig = u32::from_le_bytes(zip[at..at + 4].try_into().ok()?);
        if sig != 0x0403_4B50 {
            break;
        }
        let method = u16::from_le_bytes(zip[at + 8..at + 10].try_into().ok()?);
        let comp_len = u32::from_le_bytes(zip[at + 18..at + 22].try_into().ok()?) as usize;
        let name_len = u16::from_le_bytes(zip[at + 26..at + 28].try_into().ok()?) as usize;
        let extra_len = u16::from_le_bytes(zip[at + 28..at + 30].try_into().ok()?) as usize;
        let name_at: usize = at.checked_add(30)?;
        let data_at: usize = name_at.checked_add(name_len)?.checked_add(extra_len)?;
        let end: usize = data_at.checked_add(comp_len)?;
        if end > zip.len() {
            break;
        }
        if method == 0 && zip[name_at..name_at + name_len] == *name.as_bytes() {
            return Some(&zip[data_at..end]);
        }
        at = end;
    }
    None
}

/// Reverse the writer's five-entity escaping, in one scan.
fn unescape_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some((before, after)) = rest.split_once('&') {
        out.push_str(before);
        let (entity, tail) = after.split_once(';').expect("entity close");
        match entity {
            "amp" => out.push('&'),
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "quot" => out.push('"'),
            "apos" => out.push('\''),
            other => panic!("unknown entity &{other};"),
        }
        rest = tail;
    }
    out.push_str(rest);
    out
}

/// Data rows as typed cells: self-closing `<c/>` is blank, `t="b"`
/// reads `<v>1</v>`/`<v>0</v>` back as a bool, `t="inlineStr"`
/// reads its `<t>` text (unescaped) back as a string, and any
/// other `<v>` text parses back to `f64`. The header row is
/// skipped.
fn parse_data_rows(sheet: &[u8]) -> Vec<Vec<Option<CellValue>>> {
    let text = std::str::from_utf8(sheet).expect("sheet is utf-8");
    let mut rows = Vec::new();
    let mut first = true;
    for chunk in text.split("<row").skip(1) {
        let body = chunk.split_once('>').expect("row tag").1;
        let inner = body.split_once("</row>").expect("row end").0;
        if first {
            first = false;
            continue;
        }
        let mut cells = Vec::new();
        let mut rest = inner;
        while let Some((_, after)) = rest.split_once("<c ") {
            rest = after;
            let tag_end = rest.find('>').expect("cell tag");
            let tag = &rest[..tag_end];
            if tag.ends_with('/') {
                cells.push(None);
                rest = &rest[tag_end + 1..];
            } else if tag.contains("t=\"inlineStr\"") {
                let after_tag = &rest[tag_end + 1..];
                let t_open = after_tag.split_once("<t").expect("t open").1;
                let t_body = t_open.split_once('>').expect("t tag").1;
                let raw = t_body.split_once("</t>").expect("t close").0;
                cells.push(Some(CellValue::Text(unescape_text(raw))));
                rest = after_tag;
            } else {
                let after_tag = &rest[tag_end + 1..];
                let value = after_tag
                    .split_once("<v>")
                    .expect("value open")
                    .1
                    .split_once("</v>")
                    .expect("value close")
                    .0;
                if tag.contains("t=\"b\"") {
                    let b: bool = match value {
                        "1" => true,
                        "0" => false,
                        other => panic!("bool cell spells {other:?}"),
                    };
                    cells.push(Some(CellValue::Bool(b)));
                } else {
                    cells.push(Some(CellValue::Num(value.parse::<f64>().expect("number"))));
                }
                rest = after_tag;
            }
        }
        rows.push(cells);
    }
    rows
}
