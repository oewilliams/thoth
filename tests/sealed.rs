//! Sealed-write locks: refusal leaves bytes alone, success renames
//! into place. Each test owns a fresh temp directory.

// Only the header example is exercised here: the rest of the shared
// store is unused in this binary. Scope the allow to this include only.
#[allow(dead_code)]
mod common;

fn fresh_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("thoth-seal-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn header_cols(
    example: &common::Example,
) -> Vec<(thoth::contract::Column, thoth::xlsx::ColumnData<'_>)> {
    example.writer_columns()
}

#[test]
fn existing_destination_is_refused_and_untouched() {
    let dir = fresh_dir("exists");
    let out = dir.join("sealed.xlsx");
    let example = common::header_only();
    thoth::xlsx::write_xlsx_file(&out, &header_cols(&example)).unwrap();
    let before = std::fs::read(&out).unwrap();
    let err = thoth::xlsx::write_xlsx_file(&out, &header_cols(&example)).unwrap_err();
    match err {
        thoth::xlsx::WriteError::Exists(p) => assert_eq!(p, out),
        other => panic!("want Exists, got {other:?}"),
    }
    assert_eq!(std::fs::read(&out).unwrap(), before);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn preexisting_tmp_is_refused_without_dest() {
    let dir = fresh_dir("tmpexists");
    let out = dir.join("sealed.xlsx");
    let tmp = dir.join(".tmp.thoth.sealed.xlsx");
    std::fs::write(&tmp, b"crash debris").unwrap();
    let example = common::header_only();
    let err = thoth::xlsx::write_xlsx_file(&out, &header_cols(&example)).unwrap_err();
    match err {
        thoth::xlsx::WriteError::TempExists(p) => assert_eq!(p, tmp),
        other => panic!("want TempExists, got {other:?}"),
    }
    assert!(!out.exists());
    assert_eq!(std::fs::read(&tmp).unwrap(), b"crash debris");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn missing_file_name_states_its_condition() {
    let example = common::header_only();
    let err =
        thoth::xlsx::write_xlsx_file(std::path::Path::new(""), &header_cols(&example)).unwrap_err();
    match err {
        thoth::xlsx::WriteError::NoFileName(p) => assert_eq!(p, std::path::PathBuf::from("")),
        other => panic!("want NoFileName, got {other:?}"),
    }
}

#[test]
fn success_lands_beside_target_and_renames() {
    let dir = fresh_dir("success");
    let out = dir.join("sealed.xlsx");
    let tmp = dir.join(".tmp.thoth.sealed.xlsx");
    let example = common::header_only();
    thoth::xlsx::write_xlsx_file(&out, &header_cols(&example)).unwrap();
    assert!(out.exists());
    assert!(!tmp.exists());
    let bytes = std::fs::read(&out).unwrap();
    let sheet = thoth::xlsx::sheet_xml(&header_cols(&example)).unwrap();
    assert!(bytes.windows(sheet.len()).any(|w| w == sheet.as_slice()));
    let _ = std::fs::remove_dir_all(&dir);
}
