# Cycle 008 — first proper run-off

## Background

`RUNOFF.md` defines the protocol (same examples, same machine, same
`--release` profile and toolchain; same timed section as
`examples/bench.rs`; untimed read-back verification; record date,
machine, toolchain, thoth commit, challenger name plus version,
per-example wall time and byte counts) and names the challengers:
`rust_xlsxwriter` first, `simple_xlsx_writer` only if its subset
covers the examples. Each runs from its own throwaway crate
depending on `thoth` by path, never touching the library manifest.
The results table is empty: "Not run yet."

The example store (`tests/common/mod.rs`) is the shared input:
`ints_floats`, `nulls_blank`, `header_only`, `all_null_column`,
`wide`, `tall(n)` / `tall_ints(n)` / `tall_floats(n)` at caller-chosen
scale, plus the negatives `ragged` and `infinite` (never round-trip).
The timed section (`examples/bench.rs:13-34`) is fill
(`writer_columns()`, or the challenger's equivalent construction)
plus `write_xlsx_file` to a temp-dir file, timed with `Instant`;
read-back comparison (`read_sheet_values` vs `expected`) runs
untimed as correctness proof, never score. `TESTING.md` adds the
measurement rules that bind here: same-machine, same-profile runs,
absolute figures with machine/profile/commit (STYLE rule 9), and
the register ruling that relative size never justifies a cost
(STYLE register 6 — record absolutes, no "kilobytes against
megabytes").

This cycle runs the first proper run-off per that protocol. The
SPEC only: no harness code, no results are written by this cycle's
producer.

Binding references: `RUNOFF.md`, `STYLE.md`, `TESTING.md`,
`tests/common/mod.rs` (the store), `examples/bench.rs` (the timed
section).

## Requirements (each item testable)

- **R1 — Challenger order and coverage gate.** `rust_xlsxwriter`
  runs first. `simple_xlsx_writer` runs only if its supported subset
  still covers every round-trip example in the store at run-off
  scale (headers, i64/f64 values, nulls-blank, header-only,
  all-null column, wide past `Z`, tall at scale). Test: the run-off
  record states per challenger ran-vs-skipped; a skipped
  `simple_xlsx_writer` names the first uncovered example and the
  missing capability; an included one demonstrates all round-trip
  examples written and read back.
- **R2 — Throwaway-crate isolation.** Each challenger runs from its
  own throwaway crate under `/tmp` that depends on `thoth` by path.
  Challenger dependencies never touch the library's own manifest.
  Test: `git status` / `git diff` shows `Cargo.toml`, `Cargo.lock`,
  and `src/` untouched by challenger wiring; the record names each
  throwaway crate path.
- **R3 — FAIRNESS, same inputs.** Every competitor writes the same
  examples at the same shapes and scales from the store (same
  column names, kinds, values, row counts; negatives excluded from
  timing, reported as refusal behavior only). Test: reviewer maps
  each timed example to its store constructor and scale, and
  mismatched-shape runs are rejected.
- **R4 — FAIRNESS, same machine and profile.** Same machine for all
  competitors in a run; same `--release` profile (`overflow-checks
  = true`, `codegen-units = 1`) and the pinned toolchain
  (`rust-toolchain.toml`, today `1.93.1`). Test: the record names
  machine (OS, CPU, RAM, disk), toolchain (`rustc --version`), and
  profile settings once per run; runs split across machines or
  profiles are rejected as one run.
- **R5 — FAIRNESS, challenger's idiomatic fast path.** Each
  challenger uses its idiomatic fast path: no extra formatting, no
  debug scaffolding, no options thoth does not also pay for.
  Anything beyond defaults is documented per challenger with why it
  is the fair equivalent. Test: reviewer inspects each
  competitor's construction call and lists every non-default option
  with its justification; unjustified options fail review.
- **R6 — FAIRNESS, same timed section and untimed verification.**
  The timed section is identical to `examples/bench.rs`: fill /
  construct plus file write to the same temp-dir filesystem —
  nothing else inside the clock. Read-back value comparison
  (values present, nulls blank, headers in order) runs untimed as
  correctness proof, never score. Test: reviewer checks the clock
  boundaries line-by-line against `examples/bench.rs:13-34`; any
  timed verification or untimed construction fails review.
- **R7 — Version pinning recorded.** The record states each
  challenger's exact version (plus thoth commit, dirty or clean)
  the numbers were taken against. Test: versions present and
  re-resolvable (name + pinned version + thoth commit hash);
  a floating version or missing commit fails review.
- **R8 — RUNOFF.md results table filled from the actual run.**
  The empty table is replaced with measured rows: date, example,
  thoth ms, challenger name, challenger ms, thoth bytes,
  challenger bytes — plus the surrounding record (machine,
  CPU/RAM/disk, toolchain, thoth commit). Every figure comes from
  the actual run on the stated machine; no carried, estimated, or
  placeholder numbers. Test: table has no `–` cells; each number
  traces to an observed run log on the stated machine/commit;
  reviewer re-runs at least the `header_only` row and reproduces
  the same order of magnitude.
- **R9 — Absolute baselines, no relative-size justification.**
  Post-run-off benchmarks are comparison baselines: record absolute
  figures (wall time, bytes, and machine/profile/commit per STYLE
  rule 9); never justify a cost by its ratio against an unrelated
  total (STYLE register 6). Test: inspection — the record contains
  zero relative-size claims; every cost carries its absolute
  figure with machine, profile, and commit.

## Scoped files

`specs/008-runoff/SPEC.md` (this file) by the producer now;
by the run-off agent on `go`: `RUNOFF.md` (results table plus
record only), throwaway crates under `/tmp` (harness code lives
there, never in-tree). Untouched: `Cargo.toml`, `Cargo.lock`,
`src/`, `tests/`, `benches/`, `examples/`, `STYLE.md`,
`DESIGN.md`, `TESTING.md` (append a run-off entry only if the
record needs benchmark context — otherwise untouched).

## Acceptance

- SPEC lists numbered, testable requirements covering order (R1),
  isolation (R2), fairness (R3–R6), pinning (R7), results (R8),
  and absolute baselines (R9).
- No harness code and no results in this cycle's deliverable —
  the SPEC only.
- A later run-off agent can execute the SPEC without re-asking
  what to time, what to compare, or where to record it.

## Boundaries (not this cycle)

- No harness code, no challenger crates, no measured numbers here.
- Never part of `check.sh`; never repeated per change (RUNOFF.md:
  run once per serious challenger).
- No new in-tree tests, benches, or dependencies.
- No `STYLE.md` register change unless the run-off reviewer rules one.
