//! Caller exercising both new kinds: strings and bools through
//! the macro grammar, beside the existing numeric kinds.
//! `cargo run --release --example strings_bools`.

fn main() {
    let names: Vec<Option<&str>> = vec![Some("alpha"), Some(""), None, Some(" pad ")];
    let flags: Vec<Option<bool>> = vec![Some(true), Some(false), None, Some(true)];
    let path = std::env::temp_dir().join(format!(
        "thoth-strings-bools-{}-data.xlsx",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    thoth::xlsx::write_workbook_file(
        &path,
        &[thoth::dynamic_sheet!("data",
            str "name" => &names,
            bool "ok" => &flags,
        )],
    )
    .unwrap();
    let bytes: u64 = std::fs::metadata(&path).unwrap().len();
    println!("{} {} bytes", path.display(), bytes);
}
