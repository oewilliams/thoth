# DECISION — Cycle 005 applicator (2026-10-11)

## Reasoning (recorded before acting)

Read `SPEC.md` in full against the tree at `dba821b` plus the
uncommitted `STYLE.md` register line. The spec's proof-first order
is the point of the cycle (register row 6): every fix below names
the failing proof that demanded it, and R7's measurement decides
the borrow question either way. The three prescribed contract
shapes (Excel-row `Infinite`, `Rename` paths, additive
`NoFileName`) were approved as written before any code changed.
No emitter, deflate, template, or row-major changes: the cell
cycle owns the hot loop.

## What changed (proof → fix)

- R1: new `tests/sealed.rs` (4 tests). All green on the
  untouched tree — locks on current behavior, pinning refusal
  words, untouched bytes, and the rename success path. The
  `#[allow(dead_code)]` on the shared-store include mirrors the
  bench precedent; without it the new binary emits 14
  dead-code warnings and the clippy gate fails.
- R2: `infinite_names_the_excel_row` and
  `too_many_rows_states_data_and_total` failed before the fix
  (`row 1` for the second data value; no total in the message).
  Fix: `Infinite.row` is now the Excel row (`i + 2`, base
  documented on the variant), and `TooManyRows` displays data
  rows plus the sheet total. Both green after.
- R3/R4: `Rename { tmp, out, source }` with both ends in the
  message (pinned in `rename_names_tmp_and_out`) and additive
  `NoFileName(PathBuf)` with `source()` spelled explicitly
  (pinned in unit and `sealed.rs` integration tests). These two
  could not fail before the fix without breaking compilation —
  the shape change IS the fix — so their tests were written
  against the new shape and green after; the pre-change
  behavior (wrong-words `Exists` for missing file names,
  pathless rename) is recorded here, not re-proven.
- R5: `static_parts_are_pinned` freezes the five statics'
  `(len, crc32)` from a measured pre-lock run. Proven genuine
  by mutation: a one-byte `STYLES` edit fails it, revert greens
  it. The checksum is a change detector, not an oracle; the
  round-trip suite stays the oracle.
- R6: capacity math extracted into the shared `sheet_cap`
  helper, and `sheet_cap_holds` failed on its first run (sheet
  192469 bytes over cap 67901 on 500 extreme rows) — the proof
  catching a real under-sizing: positional `f64` Display makes
  `1e300` 301 digits, against the old 48-char cell budget.
  Fix: per-kind widest (20 `I64`, 327 `F64`, derived in the
  helper doc) with the test's tall shape at the true extremes
  (`i64::MAX`, `i64::MIN + 1`, `1e300`, `5e-324`, `-f64::MAX`).
  Green after. No emitter touched; the bound is structural.
- R7: scratch probe (`/tmp` crate, `thoth` by path, release
  profile, best-of-31) measured the five copies at 117.3ns per
  build against header-only `build_xlsx` at 796ns absolute.
  First probe run read 5.1ns — LLVM had elided the dead
  allocations; `black_box` per copy makes alloc, copy, and free
  real, and the refutation is recorded in `TESTING.md`. Copies
  stay with the measured figure cited at the site.
- R8: one-line caller-owned durability on `write_xlsx_file`,
  try-from-or-saturate comment on the `TooLarge` payload,
  `TESTING.md` Post-005 section, `STYLE.md` register row 6.

## What was rejected and why

- Borrowing/`Cow` for the five statics: rejected per R7. The
  honest figure is 117.3ns per build absolute; removal needs
  owned-entry `ZipWriter` churn, which no measurement
  justifies. Relative size was never stated, per the corrected
  spec.
- Infinity-scan fusion and the per-cell `ColumnData` match:
  rejected as cell-cycle work per the cycle boundary. R2 fixes
  the row base at the validation site; the loop fusion belongs
  with the emitter rewrite that re-takes the `cells` numbers.
- Re-running the criterion suites: rejected. No bench-scale
  input changed (capacity math is allocation sizing, byte
  output unchanged — round-trip plus verbatim checks green);
  the new numbers are probe-measured and recorded, not
  inherited as baselines.

## Verification

- `cargo fmt`, then `sh check.sh` with `CARGO_HOME=/tmp/cargo-home`:
  all gates green (fmt, tests, clippy `-D warnings`, doc).
- Focused runs during the work: `sealed` 4/4, `lib` 17/17
  (pre-existing 11 plus 6 new), `roundtrip` 1/1.
- `git status --short`: `M STYLE.md`, `M TESTING.md`,
  `M src/lib.rs`, `M src/xlsx.rs`, plus untracked
  `specs/005-foundation-tighten/`, `src/static_parts.rs`, and
  `tests/sealed.rs`. No commit; changes left uncommitted
  for review.

## Re-review application (user review, same day)

The reviewer's five findings filed verbatim in `REVIEW.md`.
Applied as applicator, with one recorded deviation:

- (1) Consts: partial compliance. The five static blobs moved
  to `src/static_parts.rs` (one concern: frozen skeleton
  bytes); the pin test passing against the moved bytes proves
  the move byte-identical. `MAX_ROWS`/`MAX_COLS` and the
  `U32_MAX`/`U16_MAX` ceilings stay with their checks:
  each is used once, beside its use, and moving them a module
  away would cost every reader a dereference for no caller.
  Overrule by saying so; the move is one import either way.
- (2) Comments: every comment added this cycle cut to the
  forced minimum (variant bases, the 327 derivation, the
  measured 117.3ns cite, the len-is-the-proof note). The fixed
  per-cell `17` is now a compiler-checked length expression
  instead of a comment, per rule 3. Pre-existing reviewed
  comments (004's cast mapping, seal debug-asserts) untouched.
- (3) `archive_fits` is a closed form: the loop gathers
  `datas`/`names` only, and each section total is one
  multiplication plus add. The removed per-entry running
  check is subsumed by the final total (offsets grow
  monotonically), so the spelling is behavior-identical —
  proven by the five boundary tests green unchanged, not by
  argument. Cast mapping in the doc updated to match.
- (4) `sheet_cap` and the emit loop march `letters` by zip;
  no indexing, no enumerate where the index was only a
  subscript. Header loop keeps its enumerate: the index feeds
  `header_cell`.
- (5) `sheet_xml` cleanup in one pass: zip march, trims
  above, byte-identical output (round-trip plus verbatim
  green). What stays is cell-cycle scope, not residue: the
  per-cell `ColumnData` match (monomorphization), per-cell
  row/value formatting inside the emitters (row-string
  hoisting needs emitter signature changes), and per-cell
  bounds checks (removal needs `unsafe`, no measurement
  justifies it — the push-branch precedent). Speed measured,
  not labeled: the best-of-51 A/B (TESTING.md) reads equal
  speed within spread, and the superseded 11-round deficit is
  recorded there as noise.

## Speed-up application (follow-on directive: measurably faster
at identical bytes)

Diagnosis before code (counting-allocator probe, tall ints):
600,006 allocs per sheet build with formatting at 88% of build
time — two temp `String`s per cell. The fix removes them:

- Row digits formatted once per row into a reused buffer and
  shared across the row's cells: `i64_cell`/`f64_cell` now
  take `row: &str` instead of `row1: usize`. Mechanical
  interface change, same tags, proven by every existing lock
  green unchanged.
- Integer values written straight into the buffer by
  `push_i64`/`push_usize` (safe code throughout: the rejected
  `as_mut_vec` spelling needed `unsafe`, so digits push as
  chars). Spelling locked by `int_spelling_matches` (±100k
  sweep plus decimal-width extremes against `to_string`).
- Boundary note for ruling: this touches `emit.rs` cell
  signatures, adjacent to cell-cycle territory. What crossed
  is mechanical (shared row spelling, direct integer digits);
  the `f64` formatting backend is untouched and stays
  deferred — floats keep one `String` per cell, which the
  numbers show (1.5x vs 2.5x).
- Measured (best-of-51 now/base/now, byte-identity asserted
  every run, TESTING.md; re-validated on the final tree):
  ints 12.68ms → 4.59ms (2.8x), floats 16.89ms → 10.92ms
  (1.5x), identical bytes throughout; sheet allocs 600,006
  → 7.

## Review II application

Reviewer findings filed verbatim in `REVIEW.md` (Verdict II).
Implemented (1) and (2); investigated (3); (4)–(6) deferred
with reasoning. Every overrule below is one sentence from the
reviewer.

- (1) Ceilings hoisted as compile-time `usize` consts
  (`DATA_MAX`/`NAME_MAX`/`COUNT_MAX`); the loop compares
  without casting and iterates by reference, no `.copied()`.
  The sums stay `u64` against `TOTAL_MAX`: a `usize` sum could
  wrap on narrower targets under `overflow-checks`, where the
  build must still refuse rather than panic — the two
  widening `as u64` are sound on all widths. Proven by the
  five boundary tests green unchanged.
- (2) `sheet_cap` is one fold expression, no `mut`; both loops
  fused into a single pass. Identical values proven by
  `sheet_cap_holds` green.
- (3) Investigated, match kept. Release aarch64 assembler: the
  width selection is `cmp` plus `csel` — predicated, no
  branch, ~13 scalar instructions per column with cold-only
  overflow exits. Max-column header-only build (16384
  columns, the fold's longest run) costs 1484.7µs for 941702
  bytes. This target has no AVX, and the checked arithmetic
  rule 8 requires refuses the portable vector transform
  without a measurement overruling it — none does. Overrule
  by producing one.
- (4) Deferred: rectangularity, row limits, and the infinity
  scan consume the borrowed data delivered per call — no
  validated container exists yet to guarantee them up front,
  and `DESIGN.md` leaves the template shape unlocked.
  Splitting validation into a separate pre-pass API has no
  caller (rule 1) and removes no work. The template cycle
  owns ingress shape; checks stay until it lands.
- (5) Deferred: infinite input is a data condition, so rule 8
  keeps it `Err` with words — flipping to panic breaks the
  `infinite` example, the row-3 test, and the round-trip
  negative path. Single-pass fusion belongs to the emit loop,
  i.e. the cell cycle. A taxonomy flip needs a SPEC-level
  decision, not a quiet applicator call.
- (6) Deferred: `ColumnData` is heterogeneous slices, so
  per-column dispatch lives somewhere regardless — splitting
  `(name, payload)` pairs into parallel arrays moves the
  pairing without removing it, with no identified consumer
  (rule 1). Ingress shape belongs to the template cycle; the
  tuple stays until it needs otherwise.

## Maximum-speed round (reviewer: nothing is fast enough)

Operating rule for this round: every candidate measured, every
deferral tied to a named future pass, nothing dismissed for
being small. Temporary mutations below were all reverted the
same session; the tree holds zero `unsafe` outside the reviewed
CRC intrinsics (verified by grep after each revert).

- Assembler first: `push_magnitude` unrolls into a per-width
  cascade (no `udiv` anywhere — LLVM already converts
  div/mod-10 to multiply-shift), which is why the `#[inline]`
  hints never took and two `bl`s stood on the per-cell path.
- Two-digit table (`DEC_PAIRS`, compile-time rodata like the
  CRC tables): halves the divisions. ~7% on tall ints
  (4.92ms → 4.55ms), locks green.
- `inline(always)` on the digit core and both cell emitters:
  calls gone, but 28% SLOWER (4.67ms → 6.00ms) — the inlined
  cascade bloats the hot loop past what the calls cost.
  Reverted the same session; recovery confirmed at 4.55ms.
  Hints stay.
- Infinity pre-scan fused into the emit loop: one pass over
  float bytes, wall-neutral within noise, kept as structure
  with net code deleted. Single-infinite reports match the old
  order exactly (locks prove it); multi-infinite order is now
  row-major instead of column-major — reviewer ruled: stays.
- Bounds checks priced by temporary `get_unchecked`
  (reverted): ~2–4% on tall ints (4.59ms → 4.41/4.49ms),
  ~1–2% on floats. Reviewer ruled: not worth `unsafe` —
  checks stay; the cell-cycle restructure may still remove
  them structurally.
- Largest named remainder: the `f64` value `String` backend
  (float builds still ~2.4x int builds). Owned by the cell
  cycle, which TESTING.md already promises re-takes these
  numbers — a named future pass, not a vague later.
