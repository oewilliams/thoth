# Cycle 008 — applicator decision record

Review verdicts from `specs/008-runoff/REVIEW.md` (overall: REJECT, with
required tightenings) and what this run did with each. The run-off agent
built one throwaway harness (`/tmp/thoth-runoff-008`, thoth by path dep,
challenger crates stay in `/tmp`, nothing committed) and ran the run-off
reported in `RUNOFF.md`.

- **R1 (order/coverage gate) — APPROVE → applied.** `rust_xlsxwriter`
  ran first and alone. `simple_xlsx_writer =0.1.2` was fetched and its
  sources inspected (`src/row.rs`, `src/sheet.rs`): SKIPPED. First
  uncovered example is `nulls_blank` — `CellValue` offers only
  bool/number/string with no blank variant, and every emitted `<c>`
  carries a `<v>`, so nulls cannot read back blank. Recorded in
  `RUNOFF.md` as required.
- **R2 (throwaway-crate isolation) — APPROVE → applied.** Only crate is
  `/tmp/thoth-runoff-008` (plus deleted probes); `git status` shows the
  library tree untouched (`Cargo.toml`, `Cargo.lock`, `src/` clean —
  the sole worktree change is `specs/008-runoff/` untracked plus this
  `RUNOFF.md` record).
- **R3 (same inputs) — APPROVE on fairness, REQUIRED fix → fix applied.**
  Tall scale pinned at `n = 200_000` (matches `examples/bench.rs`) and
  recorded per run in `RUNOFF.md`; every timed example maps to its store
  constructor; negatives excluded from timing, reported as refusal
  behavior only.
- **R4 (same machine/profile) — REJECT (blocking) → fix applied as
  prescribed.** The throwaway manifest duplicates thoth's
  `[profile.release]` (`overflow-checks = true`, `codegen-units = 1`)
  and ships a `rust-toolchain.toml` pinning `1.93.1`; machine,
  toolchain, and profile are recorded once in `RUNOFF.md`. Both sides
  built in the same crate invocation, so no profile/toolchain split is
  possible within the run.
- **R5 (idiomatic fast path) — APPROVE → applied, recommendation taken.**
  Challenger runs all-default: `add_worksheet` (not constant/low-memory
  modes), no formatting beyond the empty `Format::new()` required by the
  `write_blank` signature, headers via `write_string`, values via
  `write_number`, nulls via `write_blank`. Compression mode declared per
  side in the record (thoth stored, rust_xlsxwriter deflated default).
- **R6 (timed section + untimed verification) — REJECT (blocking, two
  holes) → both fixes applied as prescribed.** (1) Verification path
  named and built: challenger output verified with the specified
  third-party reader (`calamine =0.36.1`, headers in order plus values
  against the same `expected`); thoth output with the store reader.
  Every timed file on both sides passed read-back. (2) Methodology:
  1 warmup + best-of-5, interleaved with alternating starting side,
  per-iteration temp files removed before/after; `RUNOFF.md` carries N,
  aggregation (best/min), full samples, and spread. Clock boundaries
  kept as reviewed (fill/construct + file write inside; metadata and
  read-back outside). Citation corrected in-harness: the timed span is
  `examples/bench.rs:20-23`.
- **R7 (version pinning) — APPROVE → applied, recommendation taken as
  far as durable.** Exact pins (`rust_xlsxwriter =0.99.1`,
  `calamine =0.36.1`, thoth `82b9fb3` + worktree state) are in the
  record; the full per-round run log (`run.log`) and harness sources
  live beside the crate in `/tmp` (ephemeral by design — the table,
  samples, and this file are the durable record).
- **R8 (results table from the actual run) — REJECT (consequential)
  → resolved via the R3/R6 fixes.** Table filled with observed bests,
  no `–` cells, no carried/estimated numbers; schema extended exactly
  as the review required (tall scale per row, N/aggregation/spread,
  compression-mode note). Re-checks hold: thoth tall bytes match the
  recorded TESTING.md recipes byte-for-byte; `header_only` reproduces
  the same sub-millisecond order on both sides.
- **R9 (absolute baselines) — APPROVE → applied.** Record carries
  absolute wall time, bytes, machine, profile, and commit throughout;
  zero relative-size claims.

Disposition: every verdict honored — approvals applied as specified,
the blocking R4/R6 holes and the required R3/R8 completions fixed per
the reviewer's own prescriptions, recommendations adopted. No
unresolved gaps. `RUNOFF.md` now holds the first real run-off numbers;
challenger crates remain in `/tmp`; everything is uncommitted.
