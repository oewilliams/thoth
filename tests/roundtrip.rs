//! Top-level round-trips: every example through disk.
//!
//! Per example: fill the template, write the file (this is the path the
//! benches time), load it back through the store's independent read
//! path, and compare against expected values. Negative examples assert
//! the error string and never reach a file.

mod common;

use std::path::PathBuf;

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("thoth-test-{}-{name}.xlsx", std::process::id()))
}

#[test]
fn examples_round_trip_through_disk() {
    for example in common::all(64) {
        let cols = example.writer_columns();
        let template = common::Example::dynamic_sheet("data", &cols);
        if let Some(err_sub) = example.expect_error {
            let err = thoth::xlsx::sheet_xml_template(&template)
                .unwrap_err()
                .to_string();
            assert!(err.contains(err_sub), "{}: {err}", example.name);
            continue;
        }
        let path = temp_path(example.name);
        let _ = std::fs::remove_file(&path);
        thoth::xlsx::write_workbook_file(&path, &[template]).unwrap();
        let got = common::read_sheet_values(&path);
        assert_eq!(got, example.expected, "{}", example.name);
        // The stored archive embeds exactly the sheet built: the floor
        // oracle, checked per example, not just once.
        let sheet = thoth::xlsx::sheet_xml_template(&common::Example::dynamic_sheet("data", &cols))
            .unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(
            bytes.windows(sheet.len()).any(|w| w == sheet.as_slice()),
            "{}: zip does not embed sheet verbatim",
            example.name
        );
        let _ = std::fs::remove_file(&path);
    }
}
