//! String and bool kinds (cycle 017): boundary refusals, the
//! static impl, and macro expansions for the new kinds.

#[allow(dead_code)]
mod common;

use std::path::PathBuf;

use thoth::contract::{Column, DynamicSheet, Kind, StaticColumn, StaticSheet, Style};
use thoth::xlsx::ColumnData;

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("thoth-prim-{}-{name}.xlsx", std::process::id()))
}

fn str_cols<'a>(data: &'a [Option<&'a str>]) -> Vec<(Column, ColumnData<'a>)> {
    vec![(
        Column {
            name: "name".to_string(),
            kind: Kind::Str,
            style: Style::default(),
        },
        ColumnData::Str(data),
    )]
}

#[test]
fn mixed_kinds_round_trip_every_kind() {
    let example = common::mixed_kinds();
    let cols = example.writer_columns();
    let template = common::Example::dynamic_sheet("data", &cols);
    let path: PathBuf = temp_path("mixed");
    let _ = std::fs::remove_file(&path);
    thoth::xlsx::write_workbook_file(&path, &[template]).unwrap();
    assert_eq!(common::read_sheet_cells(&path), example.expected);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn string_at_cap_is_accepted_and_one_over_refused() {
    let at_cap: String = "x".repeat(32_767);
    let data: Vec<Option<&str>> = vec![Some(at_cap.as_str())];
    let cols = str_cols(&data);
    let sheet: DynamicSheet<'_> = DynamicSheet::known("data".to_string(), &cols);
    assert!(thoth::xlsx::sheet_xml_template(&sheet).is_ok());
    let over: String = "x".repeat(32_768);
    let data: Vec<Option<&str>> = vec![Some("ok"), Some(over.as_str())];
    let cols = str_cols(&data);
    let sheet: DynamicSheet<'_> = DynamicSheet::known("data".to_string(), &cols);
    let err: String = thoth::xlsx::sheet_xml_template(&sheet)
        .unwrap_err()
        .to_string();
    assert!(err.contains("\"name\""), "{err}");
    assert!(err.contains("row 3"), "{err}");
    assert!(err.contains("32768"), "{err}");
    assert!(err.contains("32767"), "{err}");
}

#[test]
fn control_characters_refused_but_tab_lf_cr_kept() {
    for bad in ["a\u{0}b", "a\u{B}b", "a\u{1F}b", "a\u{FFFE}b", "a\u{FFFF}b"] {
        let data: Vec<Option<&str>> = vec![Some("ok"), Some(bad)];
        let cols = str_cols(&data);
        let sheet: DynamicSheet<'_> = DynamicSheet::known("data".to_string(), &cols);
        let err: String = thoth::xlsx::sheet_xml_template(&sheet)
            .unwrap_err()
            .to_string();
        assert!(err.contains("\"name\""), "{err}");
        assert!(err.contains("row 3"), "{err}");
    }
    let kept: Vec<Option<&str>> = vec![Some("a\tb\nc\rd")];
    let cols = str_cols(&kept);
    let sheet: DynamicSheet<'_> = DynamicSheet::known("data".to_string(), &cols);
    assert!(thoth::xlsx::sheet_xml_template(&sheet).is_ok());
}

#[test]
fn new_kind_mismatch_is_refused() {
    let flags: Vec<Option<bool>> = vec![Some(true)];
    let cols: Vec<(Column, ColumnData<'_>)> = vec![(
        Column {
            name: "ok".to_string(),
            kind: Kind::Str,
            style: Style::default(),
        },
        ColumnData::Bool(&flags),
    )];
    let sheet: DynamicSheet<'_> = DynamicSheet::known("data".to_string(), &cols);
    let err: String = thoth::xlsx::sheet_xml_template(&sheet)
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("declared kind str does not match data kind bool"),
        "{err}"
    );
}

#[test]
fn static_str_bool_bytes_match_dynamic() {
    let names: Vec<Option<&str>> = vec![Some("a & b"), Some(""), None];
    let flags: Vec<Option<bool>> = vec![Some(true), None, Some(false)];
    let t: Vec<i64> = vec![1, 2, 3];
    let cols: Vec<(Column, ColumnData<'_>)> = vec![
        (
            Column {
                name: "t".to_string(),
                kind: Kind::I64,
                style: Style::default(),
            },
            ColumnData::I64(&t),
        ),
        (
            Column {
                name: "name".to_string(),
                kind: Kind::Str,
                style: Style::default(),
            },
            ColumnData::Str(&names),
        ),
        (
            Column {
                name: "ok".to_string(),
                kind: Kind::Bool,
                style: Style::default(),
            },
            ColumnData::Bool(&flags),
        ),
    ];
    let dynamic: DynamicSheet<'_> = DynamicSheet::known("fixed".to_string(), &cols);
    let fixed: StaticSheet<3, 3> = StaticSheet::new(
        "fixed",
        [
            StaticColumn::I64 {
                name: "t",
                style: Style::default(),
                data: [1, 2, 3],
            },
            StaticColumn::Str {
                name: "name",
                style: Style::default(),
                data: [Some("a & b"), Some(""), None],
            },
            StaticColumn::Bool {
                name: "ok",
                style: Style::default(),
                data: [Some(true), None, Some(false)],
            },
        ],
    );
    assert_eq!(
        thoth::xlsx::sheet_xml_template(&fixed).unwrap(),
        thoth::xlsx::sheet_xml_template(&dynamic).unwrap()
    );
    assert_eq!(
        thoth::xlsx::build_workbook(&[fixed]).unwrap(),
        thoth::xlsx::build_workbook(&[dynamic]).unwrap()
    );
}

#[test]
fn macro_str_bool_expansions_match_handwritten() {
    let names: Vec<Option<&str>> = vec![Some("x"), None, Some(" y")];
    let flags: Vec<Option<bool>> = vec![Some(false), Some(true), None];
    let macro_bytes = thoth::xlsx::build_workbook(&[thoth::dynamic_sheet!("data",
        str "name" => &names,
        bool "ok" => &flags,
    )])
    .unwrap();
    let cols = thoth::columns!(str "name" => &names, bool "ok" => &flags);
    let hand = DynamicSheet::known("data".to_string(), &cols);
    assert_eq!(macro_bytes, thoth::xlsx::build_workbook(&[hand]).unwrap());

    let fixed_macro = thoth::static_sheet!("fixed",
        str "name" => [Some("x"), None],
        bool "ok" => [Some(true), Some(false)],
    );
    let fixed_hand: StaticSheet<2, 2> = StaticSheet::new(
        "fixed",
        [
            StaticColumn::Str {
                name: "name",
                style: Style::default(),
                data: [Some("x"), None],
            },
            StaticColumn::Bool {
                name: "ok",
                style: Style::default(),
                data: [Some(true), Some(false)],
            },
        ],
    );
    assert_eq!(
        thoth::xlsx::build_workbook(&[fixed_macro]).unwrap(),
        thoth::xlsx::build_workbook(&[fixed_hand]).unwrap()
    );
}
