use super::{ColumnData, WriteError, XlsxError, archive_fits, build_workbook, build_xlsx};
use crate::contract::{Column, DynamicSheet, Kind, sheet_cap, sheet_letters};

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

#[test]
fn archive_fits_many_sheets_overflow_via_entry_count() {
    // 5 static parts plus sheets: at the `u16` count the set
    // fits (small parts, small totals); one more sheet refuses
    // only via the entry count.
    let at_max: Vec<(usize, usize)> = vec![(10, 22); u16::MAX as usize];
    assert!(archive_fits(&at_max));
    let over: Vec<(usize, usize)> = vec![(10, 22); u16::MAX as usize + 1];
    assert!(!archive_fits(&over));
}

/// The second data value refuses at Excel row 3.
#[test]
fn infinite_names_the_excel_row() {
    let px: Vec<f64> = vec![1.0, f64::INFINITY];
    let cols: Vec<(Column, ColumnData<'_>)> = vec![(
        Column {
            name: "px".to_string(),
            kind: Kind::F64,
        },
        ColumnData::F64(&px),
    )];
    let template: DynamicSheet<'_> = DynamicSheet::known("data".to_string(), &cols);
    let err: String = super::sheet_xml_template(&template)
        .unwrap_err()
        .to_string();
    assert!(err.contains("row 3"), "{err}");
}

/// The boundary scan names the first infinite in row-major order:
/// column b at row 2 beats column a at row 3.
#[test]
fn infinite_names_first_in_row_major_order() {
    let a: Vec<f64> = vec![1.0, f64::INFINITY];
    let b: Vec<f64> = vec![f64::NEG_INFINITY, 1.0];
    let cols: Vec<(Column, ColumnData<'_>)> = vec![
        (
            Column {
                name: "a".to_string(),
                kind: Kind::F64,
            },
            ColumnData::F64(&a),
        ),
        (
            Column {
                name: "b".to_string(),
                kind: Kind::F64,
            },
            ColumnData::F64(&b),
        ),
    ];
    let template: DynamicSheet<'_> = DynamicSheet::known("data".to_string(), &cols);
    let err: String = super::sheet_xml_template(&template)
        .unwrap_err()
        .to_string();
    assert!(err.contains("\"b\""), "{err}");
    assert!(err.contains("row 2"), "{err}");
}

/// The message states data rows and the sheet total.
#[test]
fn too_many_rows_states_data_and_total() {
    let err: String = XlsxError::TooManyRows(1_048_576).to_string();
    assert!(err.contains("1048576"), "{err}");
    assert!(err.contains("1048577"), "{err}");
}

/// Both ends of the failed move (a rename failure is inducible
/// only by the filesystem).
#[test]
fn rename_names_tmp_and_out() {
    let err: WriteError = WriteError::Rename {
        tmp: std::path::PathBuf::from("/tmp/.tmp.thoth.sealed.xlsx"),
        out: std::path::PathBuf::from("/tmp/sealed.xlsx"),
        source: std::io::Error::other("boom"),
    };
    let words: String = err.to_string();
    assert!(words.contains("/tmp/.tmp.thoth.sealed.xlsx"), "{words}");
    assert!(words.contains("/tmp/sealed.xlsx"), "{words}");
    assert!(words.contains("boom"), "{words}");
}

#[test]
fn wrapper_matches_direct_workbook() {
    let t: Vec<i64> = vec![100, 200];
    let cols: Vec<(Column, ColumnData<'_>)> = vec![(
        Column {
            name: "t".to_string(),
            kind: Kind::I64,
        },
        ColumnData::I64(&t),
    )];
    let direct: Vec<u8> =
        build_workbook(&[DynamicSheet::known("data".to_string(), &cols)]).unwrap();
    let wrapped: Vec<u8> = build_xlsx(&cols).unwrap();
    assert_eq!(wrapped, direct);
}

#[test]
fn ragged_in_second_sheet_names_sheet_and_column() {
    let a: Vec<i64> = vec![1, 2];
    let b: Vec<f64> = vec![1.0];
    let good_t: Vec<i64> = vec![1];
    let good: Vec<(Column, ColumnData<'_>)> = vec![(
        Column {
            name: "t".to_string(),
            kind: Kind::I64,
        },
        ColumnData::I64(&good_t),
    )];
    let ragged: Vec<(Column, ColumnData<'_>)> = vec![
        (
            Column {
                name: "a".to_string(),
                kind: Kind::I64,
            },
            ColumnData::I64(&a),
        ),
        (
            Column {
                name: "b".to_string(),
                kind: Kind::F64,
            },
            ColumnData::F64(&b),
        ),
    ];
    let sheets: Vec<DynamicSheet<'_>> = vec![
        DynamicSheet::known("First".to_string(), &good),
        DynamicSheet::known("Second".to_string(), &ragged),
    ];
    let err: String = build_workbook(&sheets).unwrap_err().to_string();
    assert!(err.contains("Second"), "{err}");
    assert!(err.contains("b"), "{err}");
}

/// A missing file name is its own refusal, never `Exists`.
#[test]
fn no_file_name_states_its_condition() {
    let err: WriteError = WriteError::NoFileName(std::path::PathBuf::from(""));
    assert!(err.to_string().contains("no file name"), "{err:?}");
    assert!(std::error::Error::source(&err).is_none());
}

/// `len <= cap` is the no-realloc proof; shapes are
/// structural representatives, extremes included.
#[test]
fn sheet_cap_holds() {
    fn check(cols: &[(Column, ColumnData<'_>)]) {
        let nrows: usize = cols[0].1.len();
        let template: DynamicSheet<'_> = DynamicSheet::known("data".to_string(), cols);
        let letters: Vec<String> = sheet_letters(&template);
        let cap: usize = sheet_cap(&template, nrows, &letters);
        let sheet: Vec<u8> = super::sheet_xml_template(&template).unwrap();
        assert!(
            sheet.len() <= cap,
            "sheet {} bytes over cap {cap}",
            sheet.len()
        );
    }
    let empty_i: Vec<i64> = vec![];
    let empty_f: Vec<f64> = vec![];
    let header: Vec<(Column, ColumnData<'_>)> = vec![
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
    ];
    check(&header);
    let wide_data: Vec<Vec<i64>> = (0..30).map(|i| vec![i, -i]).collect();
    let wide: Vec<(Column, ColumnData<'_>)> = wide_data
        .iter()
        .enumerate()
        .map(|(i, d)| {
            (
                Column {
                    name: format!("c{i}"),
                    kind: Kind::I64,
                },
                ColumnData::I64(d),
            )
        })
        .collect();
    check(&wide);
    // Non-null integer ends, longest positional floats (all finite).
    let extremes_i: [i64; 2] = [i64::MAX, i64::MIN + 1];
    let tall_i: Vec<i64> = (0..500).map(|r| extremes_i[r % 2]).collect();
    let extremes_f: [f64; 4] = [1e300, 5e-324, -f64::MAX, 0.5];
    let tall_f: Vec<f64> = (0..500).map(|r| extremes_f[r % 4]).collect();
    let tall: Vec<(Column, ColumnData<'_>)> = vec![
        (
            Column {
                name: "t".to_string(),
                kind: Kind::I64,
            },
            ColumnData::I64(&tall_i),
        ),
        (
            Column {
                name: "px".to_string(),
                kind: Kind::F64,
            },
            ColumnData::F64(&tall_f),
        ),
    ];
    check(&tall);
}
