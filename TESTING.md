# Thoth testing, benchmarks, and run-offs

Tests and benchmarks share one example store. Run-offs reuse it once
against best-of-breed writers and are not part of any gate.

## Example store

Lives in `tests/common/`, imported by integration tests and benches
alike. Each example is owned data plus its column declarations plus the
expected values a reader must observe. Idiom: the example owns its
`Vec`s and exposes a borrowed column view for the writer, so ingress
stays borrowed and no example pays for a layout the writer does not
take.

When the template type lands, examples migrate to filling it; the
protocol below does not change.

Current examples (match what the code supports today):

- `ints_floats`: mixed `i64` / `f64` values, headers present.
- `nulls_blank`: `i64::MIN` and NaN cells, which must read back blank,
  never as sentinels.
- `header_only`: zero rows; the sheet is headers alone.
- `all_null_column`: one column entirely blank.
- `wide`: many columns, few rows (exercises column letters past `Z`).
- `tall`: many rows for the timed path (small in tests, scaled up in
  benches).
- `ragged` (negative): unequal column lengths stop the run naming the
  column. Never round-trips.
- `infinite` (negative): infinities stop the run with position. Never
  round-trips.

Reserved, added only when the code supports them: inline strings
(including `&`, `<`, `>` escaping), bools, bold/colour palette combos,
empty rows and columns, date-as-string-or-number, and one fixed-size
template example.

## Tests (`cargo test`)

Per round-trip example, same code path as production:

1. Fill the template from the example and `write_xlsx_file` to a temp
   dir. Untimed.
2. Load the file with an existing reader (`calamine`). Untimed; the
   reader is verification, never score.
3. Assert the read-back cells equal the example's expected values:
   values present, nulls blank, headers in order.

Negative examples assert the error string (column name / position),
not a file. The stored-zip-verbatim check (archive embeds exactly the
sheet the emitter built) stays as the floor oracle.

## Benchmarks (timed)

Timed section only: fill the template and write the file to a temp
file on the same filesystem, per example, in `--release`. Everything
else (reader load, comparison) runs untimed as correctness proof, not
score. Report wall time, output bytes, and MB/s per example. Bench
code shares the store; only the row scale differs from tests. Two
harnesses: `examples/bench.rs` for a quick smoke
(`cargo run --release --example bench`) and `benches/fill_write.rs`
under criterion (`cargo bench`) for scored runs, including the `floor`
group (headers alone, blank cells at several scales: everything except
cell formatting).

Floor baseline, this machine, `--release`: `header_only` ~156µs,
`tall_null_1000` ~0.5ms, `tall_null_20000` ~7.6ms,
`tall_null_200000` ~80.5ms. Cost per row creeps ~346 to ~401ns across
that range, but sheet bytes per row grow 45.0 to 52.3 over the same
scales (row digits widening 1 to 6, three occurrences per row), so
cost per byte is flat at ~130MB/s all-in. Pre-sizing both buffers was
tested and measured zero gain: realloc was not the cost. The
per-row creep is content width, not overhead.

Post-001 (with the `TooLarge` guard), same machine, `--release`:
`header_only` ~156µs, `tall_null_1000` ~0.50ms,
`tall_null_20000` ~7.6ms, `tall_null_200000` ~78.7ms — all within
noise of the pre-guard baseline, so the guard costs nothing
measurable. `wide_null` ~176µs (first recording).

## Run-offs (once, not a gate)

Once per serious challenger, same examples, same machine, same
profile, same timed section (fill plus write to disk): the best-of-breed
Rust `xlsx` writers for this job (`rust_xlsxwriter`, then
`simple_xlsx_writer` if its subset still qualifies). Protocol and
results live in `RUNOFF.md`. Never in `check.sh`, never repeated per
change.

## Status

- Example store, disk round-trip tests, and the timed bench example
  are wired and gated.
- The third-party reader oracle (`calamine`) is specified above but
  not yet installed: this sandbox cannot write cargo's registry cache,
  so new dependencies cannot be fetched here. Until then the store's
  independent stored-zip read path is the verification step. Wire the
  real reader on a machine with registry access; the protocol does not
  change.
- The run-off has not been run; `RUNOFF.md` holds the empty table.
