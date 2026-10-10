//! Shared example store for tests and benches.
//!
//! Each example owns its data and exposes a borrowed column view for
//! the writer plus the values a reader must observe. Round-trip
//! examples carry row-major expectations (`None` is blank); negative
//! examples carry the error substring they must fail with and never
//! reach a file.

use thoth::contract::{Column, Kind};
use thoth::xlsx::ColumnData;

/// One column of example data, in Excel order.
pub enum ExampleColumn {
    I64 { name: String, data: Vec<i64> },
    F64 { name: String, data: Vec<f64> },
}

/// One example of a unit of functionality.
pub struct Example {
    pub name: &'static str,
    pub columns: Vec<ExampleColumn>,
    /// Expected cells row-major, header row excluded. Empty when
    /// [`Example::expect_error`] is set.
    pub expected: Vec<Vec<Option<f64>>>,
    /// Substring the writer error must contain. `None` round-trips.
    pub expect_error: Option<&'static str>,
}

impl Example {
    /// Borrowed column view for the writer.
    pub fn writer_columns(&self) -> Vec<(Column, ColumnData<'_>)> {
        self.columns
            .iter()
            .map(|c| match c {
                ExampleColumn::I64 { name, data } => (
                    Column {
                        name: name.clone(),
                        kind: Kind::I64,
                    },
                    ColumnData::I64(data),
                ),
                ExampleColumn::F64 { name, data } => (
                    Column {
                        name: name.clone(),
                        kind: Kind::F64,
                    },
                    ColumnData::F64(data),
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
                data: vec![100, 200],
            },
            ExampleColumn::F64 {
                name: "px".to_string(),
                data: vec![10.5, 11.0],
            },
        ],
        expected: vec![vec![Some(100.0), Some(10.5)], vec![Some(200.0), Some(11.0)]],
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
                data: vec![100, thoth::contract::NULL_I64],
            },
            ExampleColumn::F64 {
                name: "px".to_string(),
                data: vec![f64::NAN, 11.0],
            },
        ],
        expected: vec![vec![Some(100.0), None], vec![None, Some(11.0)]],
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
                data: vec![],
            },
            ExampleColumn::F64 {
                name: "px".to_string(),
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
                data: vec![thoth::contract::NULL_I64, thoth::contract::NULL_I64],
            },
            ExampleColumn::F64 {
                name: "px".to_string(),
                data: vec![1.0, 2.0],
            },
        ],
        expected: vec![vec![None, Some(1.0)], vec![None, Some(2.0)]],
        expect_error: None,
    }
}

/// Many columns, few rows: exercises column letters past `Z`.
pub fn wide() -> Example {
    let columns: Vec<ExampleColumn> = (0..30)
        .map(|i| ExampleColumn::I64 {
            name: format!("c{i}"),
            data: vec![i as i64, -(i as i64)],
        })
        .collect();
    let expected: Vec<Vec<Option<f64>>> = [0, 1]
        .iter()
        .map(|&r| {
            (0..30)
                .map(|i| Some(if r == 0 { i as f64 } else { -(i as f64) }))
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
    let expected: Vec<Vec<Option<f64>>> = (0..n)
        .map(|i| vec![Some(i as f64), Some(i as f64 + 0.5)])
        .collect();
    Example {
        name: "tall",
        columns: vec![
            ExampleColumn::I64 {
                name: "t".to_string(),
                data: t,
            },
            ExampleColumn::F64 {
                name: "px".to_string(),
                data: px,
            },
        ],
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
                data: vec![1, 2],
            },
            ExampleColumn::F64 {
                name: "b".to_string(),
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
            data: vec![1.0, f64::INFINITY],
        }],
        expected: vec![],
        expect_error: Some("px"),
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
pub fn read_sheet_values(path: &std::path::Path) -> Vec<Vec<Option<f64>>> {
    let bytes = std::fs::read(path).unwrap();
    let sheet =
        find_stored_entry(&bytes, "xl/worksheets/sheet1.xml").expect("sheet part missing from zip");
    parse_data_rows(sheet)
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

/// Data rows as cells: self-closing `<c/>` is blank, otherwise the
/// `<v>` text parses back to `f64`. The header row is skipped.
fn parse_data_rows(sheet: &[u8]) -> Vec<Vec<Option<f64>>> {
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
            } else {
                let after_tag = &rest[tag_end + 1..];
                let value = after_tag
                    .split_once("<v>")
                    .expect("value open")
                    .1
                    .split_once("</v>")
                    .expect("value close")
                    .0;
                cells.push(Some(value.parse::<f64>().expect("number")));
                rest = after_tag;
            }
        }
        rows.push(cells);
    }
    rows
}
