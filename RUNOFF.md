# Thoth run-offs (run once, never a gate)

One-shot comparison against the best-of-breed Rust `xlsx` writers for
this job, reusing the example store in `tests/common/`. Run once per
serious challenger, record below, and do not repeat per change. Never
part of `check.sh`.

## Protocol

- Same examples, same machine, same `--release` profile and toolchain.
- Same timed section as `examples/bench.rs`: fill the template (or the
  challenger's equivalent construction) and write the file to disk.
- Same verification: read back and compare values, untimed.
- Record: date, machine, CPU/RAM/disk, Rust toolchain, thoth commit,
  challenger name plus version, per-example wall time and byte counts.

## Challengers

1. `rust_xlsxwriter` (first; the most complete maintained writer).
2. `simple_xlsx_writer` (only if its subset still covers the examples).

Each runs from its own throwaway crate depending on `thoth` by path,
so challenger dependencies never touch the library's own manifest.

## Results — first run, 2026-10-11 (Cycle 008)

Run record (every figure below was observed on this machine/commit; no
carried, estimated, or placeholder numbers):

- Date: 2026-10-11. Machine: Darwin 25.6.0 arm64, Apple M4 (10 cores),
  16 GB RAM, disk 228 GB total / 46 GB available.
- Toolchain: `rustc 1.93.1`; profile `--release` with
  `overflow-checks = true`, `codegen-units = 1` (duplicated into the
  throwaway crate per the Cycle 008 review, so thoth-as-path-dep and the
  challenger built under identical settings).
- Thoth: commit `82b9fb3` with untracked `specs/008-runoff/` present
  (library tree itself unmodified).
- Challenger: `rust_xlsxwriter =0.99.1` (pinned). Verification reader:
  `calamine =0.36.1` (pinned, untimed use only).
- `simple_xlsx_writer =0.1.2`: SKIPPED at the R1 coverage gate. First
  uncovered example `nulls_blank`: its `CellValue` has no blank/empty
  variant (only bool/number/string) and every emitted `<c>` carries a
  `<v>`, so nulls cannot read back blank.
- Tall scale: `n = 200_000` for `tall`/`tall_ints`/`tall_floats` (matches
  `examples/bench.rs`).
- Method: 1 warmup + best-of-5 timed iterations per side per example,
  interleaved with alternating starting side, per-iteration temp files
  removed before and after. Timed section is fill/construct plus file
  write only (clock boundaries as `examples/bench.rs`); byte count and
  read-back run untimed. Thoth read-back via the store reader (stored
  entries); challenger read-back via calamine (deflated entries),
  both against the same `expected` (headers in order, values present,
  nulls blank). Every timed file verified; bytes identical across all
  5 timed rounds on both sides (asserted in-harness).
- Compression mode (declared per R5): thoth stored (method 0);
  rust_xlsxwriter deflated (method 8, default level). Byte counts embed
  that design difference; both are recorded absolute, never ratioed.
- Challenger fast path (R5, all defaults): default in-memory
  worksheet (`add_worksheet`, not constant/low-memory modes), no
  `Format` beyond `Format::new()` (empty; required only because
  `write_blank` takes a format), headers via `write_string`, values via
  `write_number`, nulls via `write_blank`.
- Negatives (refusal behavior only, untimed): `ragged`/`infinite` never
  round-trip. Thoth refuses `infinite` with a positioned error;
  rust_xlsxwriter does not refuse — it saved `f64::INFINITY` as the
  shared string `INF` with no error.
- Absolute figures only (STYLE register 6): no relative-size claims.

| date | example | thoth ms | challenger | challenger ms | thoth bytes | challenger bytes |
| ---- | ------- | -------- | ---------- | ------------- | ----------- | ---------------- |
| 2026-10-11 | ints_floats | 0.1 | rust_xlsxwriter 0.99.1 | 0.3 | 3501 | 5477 |
| 2026-10-11 | nulls_blank | 0.1 | rust_xlsxwriter 0.99.1 | 0.3 | 3474 | 5468 |
| 2026-10-11 | header_only | 0.1 | rust_xlsxwriter 0.99.1 | 0.3 | 3371 | 5442 |
| 2026-10-11 | all_null_column | 0.1 | rust_xlsxwriter 0.99.1 | 0.3 | 3471 | 5461 |
| 2026-10-11 | wide | 0.1 | rust_xlsxwriter 0.99.1 | 0.3 | 6283 | 5873 |
| 2026-10-11 | tall (n=200000) | 16.1 | rust_xlsxwriter 0.99.1 | 184.6 | 17047851 | 1975639 |
| 2026-10-11 | tall_ints (n=200000) | 6.4 | rust_xlsxwriter 0.99.1 | 152.2 | 10470009 | 1256908 |
| 2026-10-11 | tall_floats (n=200000) | 11.9 | rust_xlsxwriter 0.99.1 | 152.7 | 10870010 | 1292906 |

"ms" is best-of-5. Full timed samples (ms) and spread (max−min):

- ints_floats: thoth [0.1, 0.1, 0.1, 0.1, 0.1] spread 0.0;
  rxw [0.7, 0.4, 0.3, 0.3, 0.3] spread 0.4.
- nulls_blank: thoth [0.1 ×5] spread 0.0;
  rxw [0.3, 0.4, 0.5, 0.3, 0.3] spread 0.2.
- header_only: thoth [0.1 ×5] spread 0.0;
  rxw [0.3, 0.3, 0.3, 0.4, 0.3] spread 0.1.
- all_null_column: thoth [0.1 ×5] spread 0.0;
  rxw [0.3, 0.3, 0.3, 0.3, 0.3] spread 0.0.
- wide: thoth [0.1 ×5] spread 0.0; rxw [0.4, 0.3, 0.4, 0.3, 0.3]
  spread 0.1.
- tall: thoth [16.5, 16.4, 16.1, 16.2, 16.4] spread 0.4;
  rxw [192.4, 189.7, 184.6, 187.3, 186.5] spread 7.8.
- tall_ints: thoth [7.1, 6.5, 7.2, 6.4, 7.7] spread 1.3;
  rxw [152.2, 153.8, 154.4, 152.2, 154.7] spread 2.5.
- tall_floats: thoth [12.0, 11.9, 12.1, 11.9, 12.0] spread 0.2;
  rxw [154.6, 154.0, 152.7, 154.9, 153.5] spread 2.2.

Small-example times sit at the ~0.1 ms display floor; the load-bearing
rows are the three talls. Cross-checks: thoth `tall_ints`/`tall_floats`
bytes (10470009/10870010) match the recorded TESTING.md recipes exactly;
`header_only` reproduces the same sub-millisecond order on both sides.
Harness: `/tmp/thoth-runoff-008` (throwaway crate, thoth by path dep);
full run log with per-round lines is `run.log` beside it. The harness
is ephemeral; the table and samples above are the durable record.

## Results — second run, 2026-10-13 (post cycles 018–019)

- Machine: Darwin 27.0.0 arm64, Apple M4 (10 cores), 16 GB RAM.
  Toolchain: `rustc 1.93.1`; `--release` with `overflow-checks = true`,
  `codegen-units = 1` in the throwaway crate, so both sides built
  under identical settings.
- Thoth: commit `a0fddc9` (cycle 019). Challenger:
  `rust_xlsxwriter =0.99.1` (pinned), default in-memory worksheet,
  deflated output; per-column formats through the `*_with_format`
  spellings (bold/background for the `styled` example, plain format
  elsewhere). Verification reader: `calamine =0.36.1` (untimed).
- `simple_xlsx_writer`: not re-run; the Cycle 008 coverage gate
  stands (its `CellValue` has no blank variant).
- Method: 1 warmup + best-of-5 timed iterations per side per
  example, sides run sequentially (not interleaved, unlike 008).
  Timed section is fill/construct plus file write only. Every timed
  file verified untimed: thoth through the store reader (stored
  entries), challenger through calamine (deflated entries), both
  against the same `expected`. One reader caveat: calamine reads
  the challenger's empty-string cells as blank, so `Text("")` and
  blank are normalised for the challenger comparison only; thoth's
  empty string is verified distinct from blank by its own reader.
- Extra recipe `spec_buckets1` (not a store example): one sheet at
  the `external/workbook_spec_sheet.md` §1 shape — 8,880 rows ×
  1,298 columns (98 mixed trunk columns plus 1,200 mostly-`0.0`
  float buckets), spot-verified by row count and named cells after
  a full parse on each side.
- Absolute figures only (STYLE register 6).

| date | example | thoth ms | challenger | challenger ms | thoth bytes | challenger bytes |
| ---- | ------- | -------- | ---------- | ------------- | ----------- | ---------------- |
| 2026-10-13 | ints_floats | 0.3 | rust_xlsxwriter 0.99.1 | 0.7 | 5028 | 5502 |
| 2026-10-13 | nulls_blank | 0.2 | rust_xlsxwriter 0.99.1 | 0.6 | 5001 | 5494 |
| 2026-10-13 | header_only | 0.2 | rust_xlsxwriter 0.99.1 | 0.5 | 4898 | 5467 |
| 2026-10-13 | all_null_column | 0.2 | rust_xlsxwriter 0.99.1 | 0.5 | 4998 | 5487 |
| 2026-10-13 | wide | 0.2 | rust_xlsxwriter 0.99.1 | 0.5 | 7810 | 5907 |
| 2026-10-13 | tall (n=200000) | 15.3 | rust_xlsxwriter 0.99.1 | 267.3 | 17049378 | 1975665 |
| 2026-10-13 | tall_ints (n=200000) | 5.7 | rust_xlsxwriter 0.99.1 | 194.7 | 10471536 | 1256933 |
| 2026-10-13 | tall_floats (n=200000) | 11.7 | rust_xlsxwriter 0.99.1 | 197.7 | 10871537 | 1292930 |
| 2026-10-13 | mixed_kinds | 0.1 | rust_xlsxwriter 0.99.1 | 0.4 | 5536 | 5611 |
| 2026-10-13 | styled | 0.1 | rust_xlsxwriter 0.99.1 | 0.4 | 5423 | 5675 |
| 2026-10-13 | spec_buckets1 | 254.1 | rust_xlsxwriter 0.99.1 | 4044.1 | 310909948 | 30820751 |

"ms" is best-of-5. Small-example times sit at the display floor; the
load-bearing rows are the talls and `spec_buckets1`. Compression
mode embeds a design difference as in 008: thoth stored (method 0),
challenger deflated (method 8) — on `spec_buckets1` that is
310,909,948 stored bytes against 30,820,751 deflated for the same
cells, written in 254.1 ms against 4,044.1 ms. Harness:
`/tmp/thoth-runoff-020` (throwaway crate, thoth by path dep); the
harness is ephemeral and the table above is the durable record.
