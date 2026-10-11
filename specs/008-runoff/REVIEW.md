# Cycle 008 — SPEC review (fairness: apples-to-apples?)

Scope: judge `specs/008-runoff/SPEC.md` (producer, prior result 1) against the
question: does every fairness requirement actually force an apples-to-apples
comparison — same workload, same timed section, idiomatic challenger fast
path, no hidden handicap or favor? Bodies inspected: `RUNOFF.md`,
`tests/common/mod.rs` (store + reader), `examples/bench.rs` (timed section),
`benches/fill_write.rs`, `Cargo.toml` (`[profile.release]`), 
`rust-toolchain.toml`, `STYLE.md` (rules 9–10, register 6), `TESTING.md`
(protocol + status). No crates created, no source edited, no benches run.

## Verdict: REJECT — not yet runnable as a fair run-off

Two requirements carry blocking fairness holes (R4, R6), one is blocked
consequentially (R8), and one has a completeness gap against the SPEC's own
acceptance criterion (R3 scale). Details per requirement below. The structure
(R1 order/gate, R2 isolation, R5 fast-path symmetry, R7 pinning, R9
absolutes) is sound; the holes are all fixable by tightening, not redesign.

## Per-requirement verdicts

- **R1 — Challenger order and coverage gate: APPROVE.**
  `rust_xlsxwriter` first, `simple_xlsx_writer` only on full round-trip
  coverage, with ran-vs-skipped plus first-uncovered-example reporting, is
  the fair shape: it bans subset cherry-picking (timing a challenger only
  on the examples it likes). All-or-nothing coverage is a defensible
  design choice, not a handicap. Note: "runs first" interacts with cold
  file-cache effects — the fix belongs to R6 (interleave/alternate order
  within repetitions), not here.

- **R2 — Throwaway-crate isolation: APPROVE.**
  `/tmp` crates depending on `thoth` by path, with the `git status` test
  on `Cargo.toml` / `Cargo.lock` / `src/`, genuinely prevents manifest and
  tree contamination. No fairness bias either direction.

- **R3 — FAIRNESS, same inputs: APPROVE on fairness, REQUIRED
  completeness fix.**
  Forcing identical shapes/scales/values from the store (negatives as
  refusal-only) is the correct same-workload rule, and the cited store
  supports it (`ints_floats`, `nulls_blank`, `header_only`,
  `all_null_column`, `wide`, `tall(n)` family at caller scale). Gap: the
  tall scale `n` is never pinned — `examples/bench.rs:41` uses
  `all(200_000)` but the SPEC says only "caller-chosen scale" / "at
  run-off scale". A later agent must re-ask what `n` to run, which violates
  the SPEC's own acceptance criterion ("execute without re-asking what to
  compare"), and R8's table has no scale column to record it. Fix: pin
  `n` (200_000 to match the bench smoke) and add it to the run record.

- **R4 — FAIRNESS, same machine and profile: REJECT (blocking).**
  The profile/toolchain pin does not survive the throwaway-crate design.
  Cargo profiles and `rust-toolchain.toml` are resolved from the crate
  being built: a fresh crate under `/tmp` gets the default release
  profile (`overflow-checks` off, default codegen units), not thoth's
  `[profile.release]` (`overflow-checks = true`, `codegen-units = 1`,
  each commented per STYLE rule 10), and it escapes the `1.93.1` pin
  entirely. Consequences: (a) `thoth` compiled as a path dependency
  inherits the throwaway's profile, so thoth itself would be timed
  without the overflow checks it ships (and TESTING.md quantifies that
  cost — checked arithmetic dominates several hot paths — so the numbers
  would not even be comparable to the recorded baselines); (b) the two
  competitors could be built under different toolchains. The R4 test
  ("the record names machine, toolchain, profile once") records the
  damage without preventing it. Fix: require each throwaway manifest to
  duplicate the `[profile.release]` table and the toolchain pin (copy of
  `rust-toolchain.toml` or equivalent override), and extend the test to
  reject runs whose throwaway manifests differ from thoth's on either.

- **R5 — FAIRNESS, idiomatic fast path: APPROVE.**
  Each side runs its own fastest legitimate spelling, with every
  non-default option listed and justified and unjustified options failing
  review, is symmetric: it neither handcuffs the challenger to thoth's
  API shape nor lets it buy speed with options thoth doesn't get.
  Recommendation (non-blocking): explicitly count compression/entry mode
  as a declarable "option" — thoth writes stored entries (the store
  reader accepts stored only, `tests/common/mod.rs:291`) while a
  challenger's idiomatic path will likely deflate, so time and byte
  counts embed that design difference. Recording the mode keeps R9
  absolutes interpretable.

- **R6 — FAIRNESS, same timed section and untimed verification: REJECT
  (blocking). Two holes.**
  1. *The specified verification cannot verify challenger output.*
     The only in-tree reader, `read_sheet_values`, walks stored zip
     entries and returns `None` for anything else
     (`tests/common/mod.rs:274-297`), so a challenger that writes
     deflated entries — the normal case — panics at "sheet part missing
     from zip" instead of verifying. TESTING.md confirms the third-party
     reader oracle (`calamine`) is specified but not installed, with the
     stored-zip path as the interim verification. As written, R6 orders
     an untimed read-back that is unusable for exactly the files being
     compared. Fix: name the challenger verification path (install the
     specified reader, or decompress-then-parse with the same
     `expected` comparison) before any run-off counts.
  2. *No repetition or ordering methodology, so file-write timing is
     systematically biasable.* `examples/bench.rs` measures each example
     once with a single `Instant` span, and R6/R8 enshrine that
     (single ms cells, "traces to an observed run log"). Single-shot
     file writes are page-cache- and noise-sensitive — TESTING.md's own
     history shows wide intervals and superseded noisy passes — and with
     R1's fixed order the first runner always pays the cold cache.
     STYLE rule 9 already demands best-of-N within a run and spread (or
     no cross-run ratio at all). Fix: require warmup, best-of-N with
     stated N, interleaved/alternated competitor order, per-iteration
     file cleanup, and reporting of N, aggregation, and spread —
     identically for both sides. (The clock-boundary rule itself —
     fill plus file write inside, metadata/verification/printing
     outside, checked line-by-line against `examples/bench.rs` — is
     correct and stays.)
  - Citation nit (non-blocking): the timed span is strictly
    `examples/bench.rs:20-23`; the cited `:13-34` is the whole `bench()`
    function including the untimed read-back. The prose describes the
    boundary correctly; tighten the citation on the fix pass.

- **R7 — Version pinning recorded: APPROVE.**
  Exact challenger version plus thoth commit with dirty/clean state, and
  a re-resolvability test, is the right reproducibility rule.
  Recommendation (non-blocking): archive the throwaway harness sources
  and full run logs with the record — `/tmp` crates are ephemeral, and
  R5 review evidence (construction calls) otherwise evaporates.

- **R8 — Results table from the actual run: REJECT (consequential on
  R3/R6).**
  The no-carried-numbers rule and the `header_only` order-of-magnitude
  re-check are good, but the table schema bakes in the holes above:
  single ms cells with no N/aggregation/spread (R6 hole 2), no tall-scale
  column (R3 gap), and no compression-mode note (R5 recommendation).
  Fix together with R3/R6; no independent redesign needed.

- **R9 — Absolute baselines, no relative-size justification: APPROVE.**
  Correctly restates STYLE register 6 (absolute figures with
  machine/profile/commit; zero relative-size claims). Orthogonal to
  fairness, correctly scoped, testable by inspection.

## Required fixes before `go` (all tightening, no redesign)

1. R4: pin throwaway `[profile.release]` (duplicate `overflow-checks =
   true`, `codegen-units = 1`) and toolchain (`1.93.1`) inside each
   `/tmp` crate; test rejects divergence. (Blocking, fairness.)
2. R6: name a challenger-capable verification reader (the specified
   third-party reader or decompress-then-parse against the same
   `expected`). (Blocking, fairness/correctness.)
3. R6/R8: warmup + best-of-N with stated N, interleaved order,
   per-iteration cleanup; table carries N, aggregation, spread.
   (Blocking, fairness.)
4. R3/R8: pin tall `n` (recommend 200_000 per `bench.rs:41`) and record
   it per run. (Required, SPEC's own acceptance criterion.)
