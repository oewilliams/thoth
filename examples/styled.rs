//! Caller exercising the styles: bold, colour, and bold-colour
//! columns across mixed kinds, written through the macro grammar.
//! `cargo run --release --example styled`.

fn main() {
    let t: Vec<i64> = vec![100, thoth::contract::NULL_I64, 300];
    let px: Vec<f64> = vec![10.5, f64::NAN, 12.25];
    let names: Vec<Option<&str>> = vec![Some("alpha"), None, Some("")];
    let flags: Vec<Option<bool>> = vec![Some(true), None, Some(false)];
    let path = std::env::temp_dir().join(format!("thoth-styled-{}-data.xlsx", std::process::id()));
    let _ = std::fs::remove_file(&path);
    thoth::xlsx::write_workbook_file(
        &path,
        &[thoth::dynamic_sheet!("data",
            i64 [bold] "t" => &t,
            f64 [red] "px" => &px,
            str [green, bold] "name" => &names,
            bool [amber] "ok" => &flags,
        )],
    )
    .unwrap();
    let bytes: u64 = std::fs::metadata(&path).unwrap().len();
    println!("{} {} bytes", path.display(), bytes);
}
