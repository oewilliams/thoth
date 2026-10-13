//! Timed fill-in plus write per example: `cargo run --release
//! --example bench`.
//!
//! Timed section only: fill the template and write the file. The
//! read-back comparison runs untimed as correctness proof, never
//! score. Always run in `--release`; debug numbers are not results.

#[path = "../tests/common/mod.rs"]
mod common;

use std::time::Instant;

fn bench(example: &common::Example) {
    let path = std::env::temp_dir().join(format!(
        "thoth-bench-{}-{}.xlsx",
        std::process::id(),
        example.name
    ));
    let _ = std::fs::remove_file(&path);
    let cols = example.writer_columns();
    let sheet = common::Example::dynamic_sheet("data", &cols);
    let start = Instant::now();
    thoth::xlsx::write_workbook_file(&path, &[sheet]).unwrap();
    let elapsed = start.elapsed();
    let bytes = std::fs::metadata(&path).unwrap().len();
    let got = common::read_sheet_cells(&path);
    assert_eq!(got, example.expected, "{}", example.name);
    let _ = std::fs::remove_file(&path);
    let ms = elapsed.as_secs_f64() * 1000.0;
    let mb_s = bytes as f64 / elapsed.as_secs_f64() / 1e6;
    println!(
        "{:>16} {:>10.1} ms {:>12} bytes {:>10.1} MB/s",
        example.name, ms, bytes, mb_s
    );
}

fn main() {
    println!(
        "{:>16} {:>13} {:>18} {:>14}",
        "example", "fill+write", "output", "rate"
    );
    for example in common::all(200_000) {
        if example.expect_error.is_some() {
            continue;
        }
        bench(&example);
    }
}
