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
/// The clock covers exactly the `write_workbook_file` call over a
/// dynamic template (fill plus write); one trailing cleanup after
/// the run clears the last file.
fn write_iter(b: &mut Bencher, example: &common::Example) {
    b.iter_with_setup(
        || {
            let path = write_path(example.name);
            let _ = std::fs::remove_file(&path);
            let cols = example.writer_columns();
            (path, cols)
        },
        |(path, cols)| {
            let sheet = common::Example::dynamic_sheet("data", &cols);
            thoth::xlsx::write_workbook_file(&path, &[sheet]).unwrap();
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
                style: thoth::contract::Style::default(),
                data: vec![thoth::contract::NULL_I64; n],
            },
            common::ExampleColumn::F64 {
                name: "px".to_string(),
                style: thoth::contract::Style::default(),
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
                style: thoth::contract::Style::default(),
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
    let sheets: Vec<thoth::contract::DynamicSheet<'_>> = vec![
        thoth::contract::DynamicSheet::known("First".to_string(), &cols1),
        thoth::contract::DynamicSheet::known("Second".to_string(), &cols2),
        thoth::contract::DynamicSheet::known("Headers".to_string(), &cols3),
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

/// The former `crc` micro-bench group is retired with the public
/// zip surface (cycle 014): the CRC spelling is internal machinery
/// now and is measured in context here, as build minus emission.
///
/// Phase isolation (cycle 012): per recipe, (a) emission alone —
/// `sheet_xml_template` per sheet, summed over the workbook's sheets, result
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
        let ints_sheets: Vec<thoth::contract::DynamicSheet<'_>> =
            vec![thoth::contract::DynamicSheet::known(
                "data".to_string(),
                &ints_cols,
            )];
        group.bench_function(format!("emit_tall_ints_{n}"), |b| {
            b.iter(|| {
                std::hint::black_box(
                    thoth::xlsx::sheet_xml_template(&common::Example::dynamic_sheet(
                        "data", &ints_cols,
                    ))
                    .unwrap(),
                )
            });
        });
        group.bench_function(format!("build_tall_ints_{n}"), |b| {
            b.iter(|| std::hint::black_box(thoth::xlsx::build_workbook(&ints_sheets).unwrap()));
        });
        let floats = common::tall_floats(n);
        let floats_cols = floats.writer_columns();
        let floats_sheets: Vec<thoth::contract::DynamicSheet<'_>> =
            vec![thoth::contract::DynamicSheet::known(
                "data".to_string(),
                &floats_cols,
            )];
        group.bench_function(format!("emit_tall_floats_{n}"), |b| {
            b.iter(|| {
                std::hint::black_box(
                    thoth::xlsx::sheet_xml_template(&common::Example::dynamic_sheet(
                        "data",
                        &floats_cols,
                    ))
                    .unwrap(),
                )
            });
        });
        group.bench_function(format!("build_tall_floats_{n}"), |b| {
            b.iter(|| std::hint::black_box(thoth::xlsx::build_workbook(&floats_sheets).unwrap()));
        });
        let nulls = all_null(n);
        let nulls_cols = nulls.writer_columns();
        let nulls_sheets: Vec<thoth::contract::DynamicSheet<'_>> =
            vec![thoth::contract::DynamicSheet::known(
                "data".to_string(),
                &nulls_cols,
            )];
        group.bench_function(format!("emit_tall_null_{n}"), |b| {
            b.iter(|| {
                std::hint::black_box(
                    thoth::xlsx::sheet_xml_template(&common::Example::dynamic_sheet(
                        "data",
                        &nulls_cols,
                    ))
                    .unwrap(),
                )
            });
        });
        group.bench_function(format!("build_tall_null_{n}"), |b| {
            b.iter(|| std::hint::black_box(thoth::xlsx::build_workbook(&nulls_sheets).unwrap()));
        });
        // Tall strings at a stated text width (12 chars, `value-`
        // plus six digits) and tall bools, phase build only.
        let texts: Vec<String> = (0..n).map(|i| format!("value-{i:06}")).collect();
        let text_views: Vec<Option<&str>> = texts.iter().map(|s| Some(s.as_str())).collect();
        let str_cols: Vec<(thoth::contract::Column, thoth::xlsx::ColumnData<'_>)> = vec![(
            thoth::contract::Column {
                name: "name".to_string(),
                kind: thoth::contract::Kind::Str,
                style: thoth::contract::Style::default(),
            },
            thoth::xlsx::ColumnData::Str(&text_views),
        )];
        let str_sheets: Vec<thoth::contract::DynamicSheet<'_>> =
            vec![thoth::contract::DynamicSheet::known(
                "data".to_string(),
                &str_cols,
            )];
        group.bench_function(format!("emit_tall_strings_{n}"), |b| {
            b.iter(|| {
                std::hint::black_box(
                    thoth::xlsx::sheet_xml_template(&common::Example::dynamic_sheet(
                        "data", &str_cols,
                    ))
                    .unwrap(),
                )
            });
        });
        group.bench_function(format!("build_tall_strings_{n}"), |b| {
            b.iter(|| std::hint::black_box(thoth::xlsx::build_workbook(&str_sheets).unwrap()));
        });
        let flags: Vec<Option<bool>> = (0..n).map(|i| Some(i % 2 == 0)).collect();
        let bool_cols: Vec<(thoth::contract::Column, thoth::xlsx::ColumnData<'_>)> = vec![(
            thoth::contract::Column {
                name: "ok".to_string(),
                kind: thoth::contract::Kind::Bool,
                style: thoth::contract::Style::default(),
            },
            thoth::xlsx::ColumnData::Bool(&flags),
        )];
        let bool_sheets: Vec<thoth::contract::DynamicSheet<'_>> =
            vec![thoth::contract::DynamicSheet::known(
                "data".to_string(),
                &bool_cols,
            )];
        group.bench_function(format!("emit_tall_bools_{n}"), |b| {
            b.iter(|| {
                std::hint::black_box(
                    thoth::xlsx::sheet_xml_template(&common::Example::dynamic_sheet(
                        "data", &bool_cols,
                    ))
                    .unwrap(),
                )
            });
        });
        group.bench_function(format!("build_tall_bools_{n}"), |b| {
            b.iter(|| std::hint::black_box(thoth::xlsx::build_workbook(&bool_sheets).unwrap()));
        });
    }
    // Styled recipe at 200k (mixed kinds, every column styled)
    // beside its unstyled twin: the pair prices the `s` bytes.
    {
        let n: usize = 200_000;
        let ints: Vec<i64> = (0..n as i64).collect();
        let floats: Vec<f64> = (0..n).map(|i| i as f64 + 0.5).collect();
        let texts: Vec<String> = (0..n).map(|i| format!("value-{i:06}")).collect();
        let text_views: Vec<Option<&str>> = texts.iter().map(|s| Some(s.as_str())).collect();
        let flags: Vec<Option<bool>> = (0..n).map(|i| Some(i % 2 == 0)).collect();
        let styled_cols: Vec<(thoth::contract::Column, thoth::xlsx::ColumnData<'_>)> = vec![
            (
                thoth::contract::Column {
                    name: "t".to_string(),
                    kind: thoth::contract::Kind::I64,
                    style: thoth::contract::Style {
                        bold: true,
                        colour: None,
                    },
                },
                thoth::xlsx::ColumnData::I64(&ints),
            ),
            (
                thoth::contract::Column {
                    name: "px".to_string(),
                    kind: thoth::contract::Kind::F64,
                    style: thoth::contract::Style {
                        bold: false,
                        colour: Some(thoth::contract::Colour::Red),
                    },
                },
                thoth::xlsx::ColumnData::F64(&floats),
            ),
            (
                thoth::contract::Column {
                    name: "name".to_string(),
                    kind: thoth::contract::Kind::Str,
                    style: thoth::contract::Style {
                        bold: true,
                        colour: Some(thoth::contract::Colour::Green),
                    },
                },
                thoth::xlsx::ColumnData::Str(&text_views),
            ),
            (
                thoth::contract::Column {
                    name: "ok".to_string(),
                    kind: thoth::contract::Kind::Bool,
                    style: thoth::contract::Style {
                        bold: false,
                        colour: Some(thoth::contract::Colour::Amber),
                    },
                },
                thoth::xlsx::ColumnData::Bool(&flags),
            ),
        ];
        let plain_cols: Vec<(thoth::contract::Column, thoth::xlsx::ColumnData<'_>)> = styled_cols
            .iter()
            .map(|(col, data)| {
                (
                    thoth::contract::Column {
                        name: col.name.clone(),
                        kind: col.kind,
                        style: thoth::contract::Style::default(),
                    },
                    *data,
                )
            })
            .collect();
        let styled_sheets: Vec<thoth::contract::DynamicSheet<'_>> =
            vec![thoth::contract::DynamicSheet::known(
                "data".to_string(),
                &styled_cols,
            )];
        let plain_sheets: Vec<thoth::contract::DynamicSheet<'_>> =
            vec![thoth::contract::DynamicSheet::known(
                "data".to_string(),
                &plain_cols,
            )];
        group.bench_function("build_styled_mixed_200000", |b| {
            b.iter(|| std::hint::black_box(thoth::xlsx::build_workbook(&styled_sheets).unwrap()));
        });
        group.bench_function("build_mixed_200000", |b| {
            b.iter(|| std::hint::black_box(thoth::xlsx::build_workbook(&plain_sheets).unwrap()));
        });
    }
    let ex1 = common::tall_ints(20_000);
    let ex2 = common::tall_floats(20_000);
    let ex3 = common::header_only();
    let cols1 = ex1.writer_columns();
    let cols2 = ex2.writer_columns();
    let cols3 = ex3.writer_columns();
    let sheets: Vec<thoth::contract::DynamicSheet<'_>> = vec![
        thoth::contract::DynamicSheet::known("First".to_string(), &cols1),
        thoth::contract::DynamicSheet::known("Second".to_string(), &cols2),
        thoth::contract::DynamicSheet::known("Headers".to_string(), &cols3),
    ];
    group.bench_function("emit_three_sheet_20000", |b| {
        b.iter(|| {
            for sheet in sheets.iter() {
                std::hint::black_box(thoth::xlsx::sheet_xml_template(sheet).unwrap());
            }
        });
    });
    group.bench_function("build_three_sheet_20000", |b| {
        b.iter(|| std::hint::black_box(thoth::xlsx::build_workbook(&sheets).unwrap()));
    });
    group.finish();
}

/// Static impl at its example's fixed shape (2 columns x 3 rows):
/// build and end-to-end write, beside the dynamic spelling of the
/// same sheet, so Post-015 records what compile-time shape buys.
fn static_template(c: &mut Criterion) {
    let fixed: thoth::contract::StaticSheet<2, 3> = thoth::contract::StaticSheet::new(
        "fixed",
        [
            thoth::contract::StaticColumn::I64 {
                name: "t",
                style: thoth::contract::Style::default(),
                data: [100, 200, 300],
            },
            thoth::contract::StaticColumn::F64 {
                name: "px",
                style: thoth::contract::Style::default(),
                data: [10.5, 11.0, 12.25],
            },
        ],
    );
    let t: Vec<i64> = vec![100, 200, 300];
    let px: Vec<f64> = vec![10.5, 11.0, 12.25];
    let cols: Vec<(thoth::contract::Column, thoth::xlsx::ColumnData<'_>)> = vec![
        (
            thoth::contract::Column {
                name: "t".to_string(),
                kind: thoth::contract::Kind::I64,
                style: thoth::contract::Style::default(),
            },
            thoth::xlsx::ColumnData::I64(&t),
        ),
        (
            thoth::contract::Column {
                name: "px".to_string(),
                kind: thoth::contract::Kind::F64,
                style: thoth::contract::Style::default(),
            },
            thoth::xlsx::ColumnData::F64(&px),
        ),
    ];
    let dynamic: thoth::contract::DynamicSheet<'_> =
        thoth::contract::DynamicSheet::known("fixed".to_string(), &cols);
    let mut group = c.benchmark_group("static_template");
    group.bench_function("build_static_fixed", |b| {
        b.iter(|| std::hint::black_box(thoth::xlsx::build_workbook(&[fixed]).unwrap()));
    });
    group.bench_function("build_dynamic_fixed", |b| {
        b.iter(|| {
            std::hint::black_box(
                thoth::xlsx::build_workbook(std::slice::from_ref(&dynamic)).unwrap(),
            )
        });
    });
    group.bench_function("write_static_fixed", |b| {
        b.iter_with_setup(
            || {
                let path = write_path("static_fixed");
                let _ = std::fs::remove_file(&path);
                path
            },
            |path| {
                thoth::xlsx::write_workbook_file(&path, &[fixed]).unwrap();
            },
        );
        let path = write_path("static_fixed");
        let _ = std::fs::remove_file(&path);
    });
    group.finish();
}

criterion_group!(
    benches,
    fill_write,
    floor,
    cells,
    multi_sheet,
    phase,
    static_template
);
criterion_main!(benches);
