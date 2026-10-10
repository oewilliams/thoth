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

## Results

Not run yet. No numbers below are real until this table is filled from
an actual run.

| date | example | thoth ms | challenger | challenger ms | thoth bytes | challenger bytes |
| ---- | ------- | -------- | ---------- | ------------- | ----------- | ---------------- |
| –    | –       | –        | –          | –             | –           | –                |
