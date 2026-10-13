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

/// One untimed-setup, timed-write iteration: per-iteration debris
/// removal and column construction run in setup, outside the clock.
/// The clock covers exactly the `write_xlsx_file` call (fill plus
/// write); one trailing cleanup after the run clears the last file.
fn write_iter(b: &mut Bencher, example: &common::Example) {
    b.iter_with_setup(
        || {
            let path = write_path(example.name);
            let _ = std::fs::remove_file(&path);
            let cols = example.writer_columns();
            (path, cols)
        },
        |(path, cols)| {
            thoth::xlsx::write_xlsx_file(&path, &cols).unwrap();
        },
    );
    let path = write_path(example.name);
    let _ = std::fs::remove_file(&path);
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

/// Three-sheet workbook at bench scale (20k rows on each of two
/// sheets, one header-only sheet): the multi-sheet baseline 012
/// inherits. Sheets and columns are built once outside the clock;
/// the timed section is exactly the `write_workbook_file` call.
fn multi_sheet(c: &mut Criterion) {
    let ex1 = common::tall_ints(20_000);
    let ex2 = common::tall_floats(20_000);
    let ex3 = common::header_only();
    let cols1 = ex1.writer_columns();
    let cols2 = ex2.writer_columns();
    let cols3 = ex3.writer_columns();
    let sheets: Vec<thoth::xlsx::Sheet<'_>> = vec![
        thoth::xlsx::Sheet {
            name: "First".to_string(),
            columns: &cols1,
        },
        thoth::xlsx::Sheet {
            name: "Second".to_string(),
            columns: &cols2,
        },
        thoth::xlsx::Sheet {
            name: "Headers".to_string(),
            columns: &cols3,
        },
    ];
    let mut group = c.benchmark_group("multi_sheet");
    group.bench_function("three_sheet_20000", |b| {
        b.iter_with_setup(
            || {
                let path = write_path("three_sheet_20000");
                let _ = std::fs::remove_file(&path);
                path
            },
            |path| {
                thoth::xlsx::write_workbook_file(&path, &sheets).unwrap();
            },
        );
        let path = write_path("three_sheet_20000");
        let _ = std::fs::remove_file(&path);
    });
    group.finish();
}

/// Phase isolation (cycle 012): per recipe, (a) emission alone —
/// `sheet_xml` per sheet, summed over the workbook's sheets, result
/// `black_box`ed, no file touched; (b) build alone — `build_workbook`,
/// no file touched. End-to-end (c) is never re-implemented here: the
/// `fill_write` / `cells` / `floor` / `multi_sheet` groups already time
/// it, and the Post-012 table re-uses those numbers. Columns and sheet
/// sets are built once outside the clock (cycle 010); the emission
/// allocation is part of the phase cost and stays in the clock, exactly
/// as the library performs it.
fn phase(c: &mut Criterion) {
    let mut group = c.benchmark_group("phase");
    for n in [20_000, 200_000] {
        let ints = common::tall_ints(n);
        let ints_cols = ints.writer_columns();
        let ints_sheets: Vec<thoth::xlsx::Sheet<'_>> = vec![thoth::xlsx::Sheet {
            name: "data".to_string(),
            columns: &ints_cols,
        }];
        group.bench_function(format!("emit_tall_ints_{n}"), |b| {
            b.iter(|| std::hint::black_box(thoth::xlsx::sheet_xml(&ints_cols).unwrap()));
        });
        group.bench_function(format!("build_tall_ints_{n}"), |b| {
            b.iter(|| std::hint::black_box(thoth::xlsx::build_workbook(&ints_sheets).unwrap()));
        });
        let floats = common::tall_floats(n);
        let floats_cols = floats.writer_columns();
        let floats_sheets: Vec<thoth::xlsx::Sheet<'_>> = vec![thoth::xlsx::Sheet {
            name: "data".to_string(),
            columns: &floats_cols,
        }];
        group.bench_function(format!("emit_tall_floats_{n}"), |b| {
            b.iter(|| std::hint::black_box(thoth::xlsx::sheet_xml(&floats_cols).unwrap()));
        });
        group.bench_function(format!("build_tall_floats_{n}"), |b| {
            b.iter(|| std::hint::black_box(thoth::xlsx::build_workbook(&floats_sheets).unwrap()));
        });
        let nulls = all_null(n);
        let nulls_cols = nulls.writer_columns();
        let nulls_sheets: Vec<thoth::xlsx::Sheet<'_>> = vec![thoth::xlsx::Sheet {
            name: "data".to_string(),
            columns: &nulls_cols,
        }];
        group.bench_function(format!("emit_tall_null_{n}"), |b| {
            b.iter(|| std::hint::black_box(thoth::xlsx::sheet_xml(&nulls_cols).unwrap()));
        });
        group.bench_function(format!("build_tall_null_{n}"), |b| {
            b.iter(|| std::hint::black_box(thoth::xlsx::build_workbook(&nulls_sheets).unwrap()));
        });
    }
    let ex1 = common::tall_ints(20_000);
    let ex2 = common::tall_floats(20_000);
    let ex3 = common::header_only();
    let cols1 = ex1.writer_columns();
    let cols2 = ex2.writer_columns();
    let cols3 = ex3.writer_columns();
    let sheets: Vec<thoth::xlsx::Sheet<'_>> = vec![
        thoth::xlsx::Sheet {
            name: "First".to_string(),
            columns: &cols1,
        },
        thoth::xlsx::Sheet {
            name: "Second".to_string(),
            columns: &cols2,
        },
        thoth::xlsx::Sheet {
            name: "Headers".to_string(),
            columns: &cols3,
        },
    ];
    group.bench_function("emit_three_sheet_20000", |b| {
        b.iter(|| {
            for sheet in sheets.iter() {
                std::hint::black_box(thoth::xlsx::sheet_xml(sheet.columns).unwrap());
            }
        });
    });
    group.bench_function("build_three_sheet_20000", |b| {
        b.iter(|| std::hint::black_box(thoth::xlsx::build_workbook(&sheets).unwrap()));
    });
    group.finish();
}

criterion_group!(benches, fill_write, floor, cells, crc, multi_sheet, phase);
criterion_main!(benches);
