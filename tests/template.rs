//! Template contract locks (cycle 015): both impls through the
//! identical emitters, the named-unknown growth path, and the
//! static caller's round-trip.

#[allow(dead_code)]
mod common;

use std::path::PathBuf;

use thoth::contract::{
    ARENA_BLOCK, Column, DynamicSheet, Kind, ShapeKnowledge, SheetTemplate, StaticColumn,
    StaticSheet, Style, UnknownDimension, sheet_hint, sheet_letters,
};
use thoth::xlsx::ColumnData;

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("thoth-template-{}-{name}.xlsx", std::process::id()))
}

fn sample_cols<'a>(t: &'a [i64], px: &'a [f64]) -> Vec<(Column, ColumnData<'a>)> {
    vec![
        (
            Column {
                name: "t".to_string(),
                kind: Kind::I64,
                style: Style::default(),
            },
            ColumnData::I64(t),
        ),
        (
            Column {
                name: "px".to_string(),
                kind: Kind::F64,
                style: Style::default(),
            },
            ColumnData::F64(px),
        ),
    ]
}

#[test]
fn unknown_rows_bytes_match_known_shape() {
    let t: Vec<i64> = vec![100, 200, 300];
    let px: Vec<f64> = vec![10.5, 11.0, 12.25];
    let cols = sample_cols(&t, &px);
    let known: DynamicSheet<'_> = DynamicSheet::known("data".to_string(), &cols);
    let unknown: DynamicSheet<'_> = DynamicSheet::unknown_rows("data".to_string(), &cols);
    assert_eq!(
        unknown.shape().unknown_dimension(),
        Some(UnknownDimension::Rows)
    );
    assert_eq!(known.shape(), ShapeKnowledge::known(3, 2));
    // Sheet bytes and whole-archive bytes are identical either way.
    assert_eq!(
        thoth::xlsx::sheet_xml_template(&unknown).unwrap(),
        thoth::xlsx::sheet_xml_template(&known).unwrap()
    );
    assert_eq!(
        thoth::xlsx::build_workbook(&[unknown]).unwrap(),
        thoth::xlsx::build_workbook(&[known]).unwrap()
    );
}

#[test]
fn unknown_rows_end_to_end_through_disk() {
    let t: Vec<i64> = vec![100, 200];
    let px: Vec<f64> = vec![10.5, 11.0];
    let cols = sample_cols(&t, &px);
    let unknown: DynamicSheet<'_> = DynamicSheet::unknown_rows("data".to_string(), &cols);
    let path: PathBuf = temp_path("unknown");
    let _ = std::fs::remove_file(&path);
    thoth::xlsx::write_workbook_file(&path, &[unknown]).unwrap();
    let expected: Vec<Vec<Option<f64>>> =
        vec![vec![Some(100.0), Some(10.5)], vec![Some(200.0), Some(11.0)]];
    assert_eq!(common::read_sheet_values(&path), expected);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn hint_is_expected_width_never_worst_case() {
    let t: Vec<i64> = (0..1000).collect();
    let px: Vec<f64> = (0..1000).map(|i| i as f64 + 0.5).collect();
    let cols = sample_cols(&t, &px);
    let known: DynamicSheet<'_> = DynamicSheet::known("data".to_string(), &cols);
    let unknown: DynamicSheet<'_> = DynamicSheet::unknown_rows("data".to_string(), &cols);
    let letters: Vec<String> = sheet_letters(&known);
    let hint: usize = sheet_hint(&known, &letters).expect("known shape hints");
    let cap: usize = thoth::contract::sheet_cap(&known, 1000, &letters);
    assert!(hint < cap, "hint {hint} must stay under cap {cap}");
    assert_eq!(sheet_hint(&unknown, &letters), None);
}

#[test]
fn static_bytes_match_dynamic_spelling() {
    let t: Vec<i64> = vec![100, 200, 300];
    let px: Vec<f64> = vec![10.5, 11.0, 12.25];
    let cols = sample_cols(&t, &px);
    let dynamic: DynamicSheet<'_> = DynamicSheet::known("fixed".to_string(), &cols);
    let fixed: StaticSheet<2, 3> = StaticSheet::new(
        "fixed",
        [
            StaticColumn::I64 {
                name: "t",
                style: Style::default(),
                data: [100, 200, 300],
            },
            StaticColumn::F64 {
                name: "px",
                style: Style::default(),
                data: [10.5, 11.0, 12.25],
            },
        ],
    );
    assert_eq!(fixed.shape(), ShapeKnowledge::known(3, 2));
    assert_eq!(fixed.sheet_name(), "fixed");
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
fn static_example_round_trips_through_disk() {
    let fixed: StaticSheet<2, 3> = StaticSheet::new(
        "fixed",
        [
            StaticColumn::I64 {
                name: "t",
                style: Style::default(),
                data: [100, 200, 300],
            },
            StaticColumn::F64 {
                name: "px",
                style: Style::default(),
                data: [10.5, 11.0, 12.25],
            },
        ],
    );
    let path: PathBuf = temp_path("static");
    let _ = std::fs::remove_file(&path);
    thoth::xlsx::write_workbook_file(&path, &[fixed]).unwrap();
    let expected: Vec<Vec<Option<f64>>> = vec![
        vec![Some(100.0), Some(10.5)],
        vec![Some(200.0), Some(11.0)],
        vec![Some(300.0), Some(12.25)],
    ];
    assert_eq!(common::read_sheet_values(&path), expected);
    // Verbatim embed: the archive embeds exactly the oracle sheet.
    let sheet: Vec<u8> = thoth::xlsx::sheet_xml_template(&fixed).unwrap();
    let bytes: Vec<u8> = std::fs::read(&path).unwrap();
    assert!(bytes.windows(sheet.len()).any(|w| w == sheet.as_slice()));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn kind_mismatch_is_refused_at_the_boundary() {
    // RC1: declared F64 over I64 data must refuse, never emit ints.
    let t: Vec<i64> = vec![100, 200];
    let cols: Vec<(Column, ColumnData<'_>)> = vec![(
        Column {
            name: "t".to_string(),
            kind: Kind::F64,
            style: Style::default(),
        },
        ColumnData::I64(&t),
    )];
    let dynamic: DynamicSheet<'_> = DynamicSheet::known("data".to_string(), &cols);
    let err: String = thoth::xlsx::sheet_xml_template(&dynamic)
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("declared kind f64 does not match data kind i64"),
        "{err}"
    );
    let err: String = thoth::xlsx::build_workbook(&[dynamic])
        .unwrap_err()
        .to_string();
    assert!(err.contains("declared kind f64"), "{err}");
    // The compat spelling routes through the same refusal.
    assert!(thoth::xlsx::sheet_xml(&cols).is_err());
}

#[test]
fn unknown_columns_bytes_match_known_and_hint_is_none() {
    // RC2: unknown columns names the dimension, hints None (growth),
    // and emits bytes identical to the known spelling.
    let t: Vec<i64> = vec![100, 200, 300];
    let px: Vec<f64> = vec![10.5, 11.0, 12.25];
    let cols = sample_cols(&t, &px);
    let known: DynamicSheet<'_> = DynamicSheet::known("data".to_string(), &cols);
    let unknown: DynamicSheet<'_> = DynamicSheet::unknown_columns("data".to_string(), &cols);
    assert_eq!(
        unknown.shape().unknown_dimension(),
        Some(UnknownDimension::Columns)
    );
    let letters: Vec<String> = sheet_letters(&known);
    assert_eq!(sheet_hint(&unknown, &letters), None);
    assert_eq!(
        thoth::xlsx::sheet_xml_template(&unknown).unwrap(),
        thoth::xlsx::sheet_xml_template(&known).unwrap()
    );
    assert_eq!(
        thoth::xlsx::build_workbook(&[unknown]).unwrap(),
        thoth::xlsx::build_workbook(&[known]).unwrap()
    );
}

#[test]
fn unknown_rows_growth_bytes_match_known_shape() {
    // RC3: 5000 rows emit well past the ARENA_BLOCK initial
    // reservation, so the builder must grow on the unknown path.
    let t: Vec<i64> = (0..5000).collect();
    let px: Vec<f64> = (0..5000).map(|i| i as f64 + 0.5).collect();
    let cols = sample_cols(&t, &px);
    let known: DynamicSheet<'_> = DynamicSheet::known("data".to_string(), &cols);
    let unknown: DynamicSheet<'_> = DynamicSheet::unknown_rows("data".to_string(), &cols);
    let known_bytes: Vec<u8> = thoth::xlsx::build_workbook(&[known]).unwrap();
    assert!(known_bytes.len() > ARENA_BLOCK);
    assert_eq!(
        thoth::xlsx::build_workbook(&[unknown]).unwrap(),
        known_bytes
    );
}

#[test]
fn compat_spellings_route_through_contract() {
    // Pre-015 public signatures keep working as contract spellings.
    let t: Vec<i64> = vec![100, 200];
    let px: Vec<f64> = vec![10.5, 11.0];
    let cols = sample_cols(&t, &px);
    let dynamic: DynamicSheet<'_> = DynamicSheet::known("data".to_string(), &cols);
    assert_eq!(
        thoth::xlsx::sheet_xml(&cols).unwrap(),
        thoth::xlsx::sheet_xml_template(&dynamic).unwrap()
    );
    assert_eq!(
        thoth::xlsx::build_xlsx(&cols).unwrap(),
        thoth::xlsx::build_workbook(&[dynamic]).unwrap()
    );
}

#[test]
fn macro_expansions_match_handwritten_bytes() {
    // Sugar expands to the plain values: same archive bytes as the
    // hand-written declarations of the same sheets.
    let fixed_macro = thoth::static_sheet!("fixed",
        i64 "t"  => [100, 200, 300],
        f64 "px" => [10.5, 11.0, 12.25],
    );
    let fixed_hand: StaticSheet<2, 3> = StaticSheet::new(
        "fixed",
        [
            StaticColumn::I64 {
                name: "t",
                style: Style::default(),
                data: [100, 200, 300],
            },
            StaticColumn::F64 {
                name: "px",
                style: Style::default(),
                data: [10.5, 11.0, 12.25],
            },
        ],
    );
    assert_eq!(
        thoth::xlsx::build_workbook(&[fixed_macro]).unwrap(),
        thoth::xlsx::build_workbook(&[fixed_hand]).unwrap()
    );

    let t: Vec<i64> = vec![1, 2, 3];
    let px: Vec<f64> = vec![1.5, 2.5, 3.5];
    let macro_bytes = thoth::xlsx::build_workbook(&[thoth::dynamic_sheet!("data",
        i64 "t" => &t,
        f64 "px" => &px,
    )])
    .unwrap();
    let cols = thoth::columns!(i64 "t" => &t, f64 "px" => &px);
    let hand = DynamicSheet::known("data".to_string(), &cols);
    assert_eq!(macro_bytes, thoth::xlsx::build_workbook(&[hand]).unwrap());

    let unknown_bytes = thoth::xlsx::build_workbook(&[thoth::dynamic_sheet!(unknown_rows "data",
        i64 "t" => &t,
        f64 "px" => &px,
    )])
    .unwrap();
    assert_eq!(unknown_bytes, macro_bytes);
}
