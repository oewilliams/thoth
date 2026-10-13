//! Fixed-shape caller for the static template impl: compile-time
//! column set and row count (`StaticSheet<2, 3>`), written through
//! the same contract and emitters as the dynamic impl.
//! `cargo run --release --example static_sheet`.

use thoth::static_sheet;

fn main() {
    let sheet = static_sheet!("fixed",
        i64 "t"  => [100, 200, 300],
        f64 "px" => [10.5, 11.0, 12.25],
    );
    let path = std::env::temp_dir().join(format!("thoth-static-{}-fixed.xlsx", std::process::id()));
    let _ = std::fs::remove_file(&path);
    thoth::xlsx::write_workbook_file(&path, &[sheet]).unwrap();
    let bytes: u64 = std::fs::metadata(&path).unwrap().len();
    println!("{} {} bytes", path.display(), bytes);
}
