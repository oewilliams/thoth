# Thoth

A fast `xlsx` writer from a declared schema, named for Thoth, the Egyptian
scribe of records and measurement.

The schema is declared, never sniffed: one column kind decided up front,
in Excel order. No per-cell type switch, no style object, no
shared-string hash. See [DESIGN.md](file:///Users/oliverwilliams/Documents/thoth/DESIGN.md)
for the locked scope (borrowed ingress, fixed plus dynamic templates,
primitives only, sealed file moves).

## Shape

* Input: borrowed columnar slices (`&[i64]` / `&[f64]`), checked
  rectangular once at the boundary.
* Output: a stored (uncompressed) `.xlsx` — one sheet, frozen static
  parts, two cell styles (default + bold header).
* Nulls: `i64::MIN` and NaN emit blank cells. Infinities stop the run
  with position — they have no Excel spelling. Ragged columns stop the
  run with the column name.
* Strings, bools, and the fixed colour palette wait for a caller that
  needs them.

## Use

As a library:

```rust
let t = vec![100i64, 200];
let px = vec![10.5f64, 11.0];
let cols = vec![
    (
        thoth::contract::Column { name: "t".into(), kind: thoth::contract::Kind::I64 },
        thoth::xlsx::ColumnData::I64(&t),
    ),
    (
        thoth::contract::Column { name: "px".into(), kind: thoth::contract::Kind::F64 },
        thoth::xlsx::ColumnData::F64(&px),
    ),
];
thoth::xlsx::write_xlsx_file(std::path::Path::new("out.xlsx"), &cols).unwrap();
```

A destination that exists is refused; bytes land in `.tmp.thoth.*`
beside it and rename into place.

## Floor first

v0 is the stored-zip floor: `memcpy` plus decimal formatting. Deflate,
widths, freeze panes, and number formats enter later, each measured
against this spelling as oracle.
