//! Criterion benches: fill plus write per example, and the floor.
//!
//! `fill_write` times template fill plus file write per store example.
//! `floor` times everything except cell formatting (headers alone,
//! blank cells at several shapes and scales) so formatting work later
//! measures against it. Correctness is the round-trip tests' job; these
//! benches never read back.

// The reader helpers are unused here: correctness is the tests' job,
// timing is this bench's. Scope the allow to this include only.
#[allow(dead_code)]
#[path = "../tests/common/mod.rs"]
mod common;

use criterion::{Bencher, Criterion, criterion_group, criterion_main};

fn write_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "thoth-criterion-{}-{name}.xlsx",
        std::process::id()
    ))
}

/// One untimed-setup, timed-write iteration: fill the template and move
/// the bytes to disk, then clear the debris.
fn write_iter(b: &mut Bencher, example: &common::Example) {
    b.iter(|| {
        let path = write_path(example.name);
        let _ = std::fs::remove_file(&path);
        let cols = example.writer_columns();
        thoth::xlsx::write_xlsx_file(&path, &cols).unwrap();
        let _ = std::fs::remove_file(&path);
    });
}

fn fill_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("fill_write");
    for example in common::all(20_000) {
        if example.expect_error.is_some() {
            continue;
        }
        group.bench_function(example.name, |b| write_iter(b, &example));
    }
    group.finish();
}

/// Blank cells skip value formatting, so these measure tag skeleton
/// emission plus CRC plus write. Three tall scales check the cost is
/// linear in rows; anything else is a hidden extra pass (realloc,
/// allocator, second touch) to hunt down.
fn all_null(n: usize) -> common::Example {
    common::Example {
        name: "tall_null",
        columns: vec![
            common::ExampleColumn::I64 {
                name: "t".to_string(),
                data: vec![thoth::contract::NULL_I64; n],
            },
            common::ExampleColumn::F64 {
                name: "px".to_string(),
                data: vec![f64::NAN; n],
            },
        ],
        expected: vec![vec![None, None]; n],
        expect_error: None,
    }
}

/// Blank wide: column-letter and header scaling, no values.
fn wide_null() -> common::Example {
    common::Example {
        name: "wide_null",
        columns: (0..30)
            .map(|i| common::ExampleColumn::I64 {
                name: format!("c{i}"),
                data: vec![thoth::contract::NULL_I64; 2],
            })
            .collect(),
        expected: vec![vec![None; 30]; 2],
        expect_error: None,
    }
}

fn floor(c: &mut Criterion) {
    let mut group = c.benchmark_group("floor");
    let header = common::header_only();
    group.bench_function("header_only", |b| write_iter(b, &header));
    let wide = wide_null();
    group.bench_function("wide_null", |b| write_iter(b, &wide));
    for n in [1_000, 20_000, 200_000] {
        let tall = all_null(n);
        let id = format!("tall_null_{n}");
        group.bench_function(&id, |b| write_iter(b, &tall));
    }
    group.finish();
}

/// Formatted cells at three scales: per-type tall examples through the
/// same fill-plus-write path. The deltas against the `floor`
/// `tall_null` benches at equal scale isolate the value-formatting
/// cost; the three scales check formatted cost is linear in rows.
fn cells(c: &mut Criterion) {
    let mut group = c.benchmark_group("cells");
    for n in [1_000, 20_000, 200_000] {
        let ints = common::tall_ints(n);
        let ints_id = format!("tall_ints_{n}");
        group.bench_function(&ints_id, |b| write_iter(b, &ints));
        let floats = common::tall_floats(n);
        let floats_id = format!("tall_floats_{n}");
        group.bench_function(&floats_id, |b| write_iter(b, &floats));
    }
    group.finish();
}

/// CRC32 micro-bench: the shipped `crc32` over a statics-sized and a
/// sheet-sized buffer, so zip-tightening cycles measure this layer
/// directly instead of inferring it from the floor. One byte is
/// touched per iteration so no two calls share an input (otherwise
/// the compiler hoists the pure call out of the loop and the bench
/// times nothing).
fn crc32_iter(b: &mut Bencher, buf: &mut [u8]) {
    b.iter(|| {
        buf[0] = buf[0].wrapping_add(1);
        thoth::zip::crc32(buf)
    });
}

fn crc(c: &mut Criterion) {
    let mut group = c.benchmark_group("crc");
    let mut small = vec![0xA5u8; 4096];
    group.bench_function("crc32_4k", |b| crc32_iter(b, &mut small));
    let mut large = vec![0xA5u8; 1 << 20];
    group.bench_function("crc32_1m", |b| crc32_iter(b, &mut large));
    group.finish();
}

criterion_group!(benches, fill_write, floor, cells, crc);
criterion_main!(benches);
