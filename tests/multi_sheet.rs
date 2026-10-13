//! Multi-sheet round-trip: three sheets with differing shapes
//! through disk, read back per sheet via the independent reader.

// Only the reader helpers are exercised here: the rest of the shared
// store is unused in this binary. Scope the allow to this include only.
#[allow(dead_code)]
mod common;

use std::path::PathBuf;

use thoth::contract::{Column, Kind};
use thoth::xlsx::{ColumnData, Sheet};

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("thoth-multi-{}-{name}.xlsx", std::process::id()))
}

#[test]
fn three_sheets_round_trip_per_sheet() {
    let t1: Vec<i64> = vec![100, 200];
    let px1: Vec<f64> = vec![10.5, 11.0];
    let cols1: Vec<(Column, ColumnData<'_>)> = vec![
        (
            Column {
                name: "t".to_string(),
                kind: Kind::I64,
            },
            ColumnData::I64(&t1),
        ),
        (
            Column {
                name: "px".to_string(),
                kind: Kind::F64,
            },
            ColumnData::F64(&px1),
        ),
    ];
    let x2: Vec<i64> = vec![7, 8, 9];
    let cols2: Vec<(Column, ColumnData<'_>)> = vec![(
        Column {
            name: "x".to_string(),
            kind: Kind::I64,
        },
        ColumnData::I64(&x2),
    )];
    let empty_i: Vec<i64> = vec![];
    let empty_f: Vec<f64> = vec![];
    let cols3: Vec<(Column, ColumnData<'_>)> = vec![
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
        (
            Column {
                name: "y".to_string(),
                kind: Kind::I64,
            },
            ColumnData::I64(&empty_i),
        ),
    ];
    let sheets: Vec<Sheet<'_>> = vec![
        Sheet {
            name: "First".to_string(),
            columns: &cols1,
        },
        Sheet {
            name: "A&B".to_string(),
            columns: &cols2,
        },
        Sheet {
            name: "Headers".to_string(),
            columns: &cols3,
        },
    ];
    let path: PathBuf = temp_path("three");
    let _ = std::fs::remove_file(&path);
    thoth::xlsx::write_workbook_file(&path, &sheets).unwrap();
    let expected1: Vec<Vec<Option<f64>>> =
        vec![vec![Some(100.0), Some(10.5)], vec![Some(200.0), Some(11.0)]];
    let expected2: Vec<Vec<Option<f64>>> = vec![vec![Some(7.0)], vec![Some(8.0)], vec![Some(9.0)]];
    let expected3: Vec<Vec<Option<f64>>> = vec![];
    assert_eq!(common::read_sheet_values_by_index(&path, 1), expected1);
    assert_eq!(common::read_sheet_values_by_index(&path, 2), expected2);
    assert_eq!(common::read_sheet_values_by_index(&path, 3), expected3);
    assert_eq!(common::read_sheet_values(&path), expected1);
    // Verbatim embed per sheet: each sheet's bytes appear whole.
    let bytes: Vec<u8> = std::fs::read(&path).unwrap();
    for cols in [&cols1, &cols2, &cols3] {
        let sheet: Vec<u8> = thoth::xlsx::sheet_xml(cols).unwrap();
        assert!(
            bytes.windows(sheet.len()).any(|w| w == sheet.as_slice()),
            "zip does not embed sheet verbatim"
        );
    }
    // Workbook names in declaration order, escaped where required.
    let workbook: Vec<u8> = common::read_part(&path, "xl/workbook.xml");
    let text: String = String::from_utf8(workbook).unwrap();
    let first: usize = text.find("name=\"First\"").expect("First missing");
    let escaped: usize = text.find("name=\"A&amp;B\"").expect("A&B missing");
    let headers: usize = text.find("name=\"Headers\"").expect("Headers missing");
    assert!(first < escaped && escaped < headers, "{text}");
    let _ = std::fs::remove_file(&path);
}
