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
cell formatting). Criterion 0.5 takes one filter only, and a directly
invoked binary needs the `--bench` flag with it (`./target/release/deps/fill_write-<hash> floor --bench`): without the flag it runs test-only mode, prints bare `Testing/Success`, and writes no estimates. After any bench run, confirm the estimates file mtime moved before quoting a number.

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

Post-004 (zip internals tightened: instruction CRC, ingress-held
validation, pre-sized writer), this machine (Darwin 25.6.0 arm64,
`RELEASE_ARM64_T8132`), bench profile, commit `bb86154` plus the
uncommitted 004 work. Criterion means, pre (same machine, same
profile, two full runs agree within ~2%) then post:

| bench | pre | post | delta |
|---|---|---|---|
| `floor/header_only` | 157.7µs | 142.9µs | −9% |
| `floor/wide_null` | 173.3µs | 144.6µs | −17% |
| `floor/tall_null_1000` | 508.1µs | 211.3µs | −58% |
| `floor/tall_null_20000` | 7.564ms | 1.779ms | −76% |
| `floor/tall_null_200000` | 80.98ms | 18.00ms | −78% |

The tall scales were CRC-bound and the numbers say so: the old
bitwise CRC cost ~5.8ns per output byte (~60ms on the 10.5MB
200k sheet), the new one ~0.085ns (~1ms). Small archives gain
less because CRC was never their cost. Output bytes are
unchanged: the `golden_small_archive_is_byte_identical` test and
the per-example verbatim-embed checks pin the bytes.

Post-005 (row sharing reaches blank cells too), same machine,
bench profile, commit `1a3c0e5`, criterion means with
criterion's own baseline comparison: `floor/header_only`
142.1µs (+1.1%, no change), `floor/wide_null` 146.0µs (−0.4%,
no change), `floor/tall_null_1000` 169.5µs (−20.2%),
`floor/tall_null_20000` 1.151ms (−40.5%),
`floor/tall_null_200000` 11.52ms (−34.6%, wide interval
8.8–15.9ms — this scale stays provisional until re-taken).
Small archives sit still because CRC was never their cost;
tall blanks move because every blank cell used to format the
row digits into its own `String`.

CRC head-to-head (scratch micro-bench, same machine, release
profile with `overflow-checks = true`, `codegen-units = 1`,
best-of-31, sheet-like bytes, 4KB and 4MB agree): bitwise
5.8ns/B, single table 1.7, slicing-by-8 0.37, ARMv8 `crc32x`
0.083. Shipped by those numbers: the instruction spelling under
compile-time `cfg(all(target_arch = "aarch64",
target_feature = "crc"))` (no runtime detection anywhere), the
slicing-by-8 spelling everywhere else. The new permanent `crc`
group records the shipped spelling: `crc32_4k` 342ns
(0.084ns/B), `crc32_1m` 90.7µs (0.087ns/B, ~11.6GB/s).

Assembler notes (`cargo rustc --release -- --emit asm`, same
machine; pre vs post):

- R1: the suspicion (bitwise loop is a vectorizer miss) was half
  wrong. LLVM had already auto-vectorized it (NEON bit-sliced
  dance, ~24 insns per byte, inlined into `add`) — but "already
  vectorized" is not "fast": the instruction spelling is one
  `crc32x` per eight bytes plus an unrolled `crc32b` tail
  (4-insn word loop, no calls, no table traffic), 69x fewer
  ns/B. Two scratch rejections on the way: the `__crc32cb`-family
  names are the Castagnoli polynomial (wrong answers against the
  IEEE vector — the shipped code uses `__crc32b/d`), and a
  single-pass slicing-by-8 table build reads unfilled row-0
  slots (shipped build fills row 0 first).
- R2: `stored_archive_len` is one map/sum expression over a
  shared `entry_sizes` helper (also feeding the writer's running
  total, so the two cannot diverge). The sharper drop is in
  `finish`, which no longer walks the entries to size itself:
  the old sizing loop is gone, replaced by a 3-insn
  running-total load. At dynamic scale the two len spellings are
  at parity (~10 hot insns/entry — checked arithmetic dominates
  both under `overflow-checks = true`); the win is structural.
- R3: with six fixed entries most of `archive_fits` folds away
  at compile time; the sheet-dependent remainder stays on the
  path (TooLarge reachable only past 4GB, as before). The
  cast-to-check mapping is in `xlsx.rs` beside the check.
- R4: `add` lost its duplicate scan (`memcmp` gone), its assert
  formatting (`panic_fmt` gone), and its growth path is cold-only
  (entries `with_capacity` at construction; the `grow_one`
  branch never fires). The push-helper suspicion was refuted:
  `push_u16`/`push_u32` already left no `bl` on the `finish`
  push paths before this cycle, and still leave none — so they
  stay named helpers (width on the line) and the per-push
  reserve branches stay as the safe-`Vec` cost: removing them
  needs `unsafe` for ~nanoseconds per archive, which no
  measurement justifies.

## Post-005 (foundation tighten: seal, words, locks)

Proof-led cycle on the `xlsx.rs` tail: each lock below failed
before its fix (or, where marked, pins current behavior), on
this machine (Darwin 25.6.0 arm64), release profile
(`overflow-checks = true`, `codegen-units = 1`), commit
`dba821b` plus the uncommitted 005 work.

- Seal: `tests/sealed.rs` locks refusal and success — existing
  destination refused with `Exists` and left byte-identical,
  pre-existing temp refused with `TempExists` with no
  destination created, success lands beside the target and
  renames, missing file name refused with the new `NoFileName`.
- Words: `Infinite.row` is the 1-based Excel row (header is row
  1; the `infinite` example refuses at row 3, pinned in
  `infinite_names_the_excel_row`). `TooManyRows` states data
  rows and the resulting sheet total (pinned in
  `too_many_rows_states_data_and_total`). `Rename` names both
  ends of the move (pinned in `rename_names_tmp_and_out`; a
  rename failure is inducible only by the filesystem, so the
  construction site is review evidence, not a test).
- Statics: `static_parts_are_pinned` trips on any edit to the
  five frozen blobs (`[Content_Types].xml` 680/`0xCC79080A`,
  `_rels/.rels` 296/`0x82C75906`, `xl/workbook.xml`
  282/`0xD4738FA2`, `xl/_rels/workbook.xml.rels`
  424/`0x8A9642E0`, `xl/styles.xml` 708/`0x24A3B074`). Pins
  captured from the pre-lock tree; verified genuine by mutation
  (one-byte `STYLES` edit fails the test, revert greens it).
  The checksum is the crate's own `crc32` used as a change
  detector; value correctness stays with the round-trip suite.
- Capacity: `sheet_xml` sizes from the shared `sheet_cap`
  helper (the `entry_sizes` pattern), and `sheet_cap_holds`
  proves `len <= cap` — the no-realloc proof — on header-only,
  wide, and tall shapes at the true extremes. The proof caught
  a real under-sizing on its first run: the old 48-char cell
  budget assumed short values, but `f64` Display expands
  positionally without exponent (probed `1e300` → 301 digits;
  `5e-324` → 326 chars), so extreme floats reallocated
  silently. Per-kind widest (20 `I64`, 327 `F64`, derived in
  the helper doc) holds on all three shapes.
- Static copies (scratch probe, `/tmp` crate against `thoth` by
  path, release profile as above, best-of-31): the five
  `as_bytes().to_vec()` calls cost 117.3ns per build;
  header-only `build_xlsx` costs 796ns in absolute terms. One
  refutation on the way: the first probe run measured 5.1ns
  for the five copies because LLVM elided the dead
  allocations — `black_box` per copy (alloc, copy, and free
  all real) gives the honest figure. The copies stay:
  borrowing them needs owned-entry `ZipWriter` churn, which no
  measurement justifies at 117.3ns per build.
- Re-review pass (same machine/profile/commit): `archive_fits`
  is a closed form — the loop gathers two sums and the section
  totals are one multiplication plus add each; the five
  boundary tests green unchanged, so the spelling is
  behavior-identical. `sheet_cap` and the emit loop march
  `letters` by zip, no indexing.
- Re-review equal-speed checkpoint (superseded below, kept for
  the method lesson): a best-of-51 A/B read HEAD inside the
  worktree's own range on both shapes, while an earlier
  best-of-11 pass had read a ~2% deficit and reversed under
  more rounds. Thin margins need rounds; eleven was not
  enough.
- Re-review speed-up (same machine/profile/commit): diagnosis
  first — one 200k-integer build cost 600,006 allocations
  (~20MB) with formatting at 11.15ms of 12.70ms total, so
  every cell bought two temp `String`s (row digits, value).
  Fix: row digits formatted once per row into a reused buffer
  and shared across the row's cells; integer values written
  straight into the buffer by a direct writer (spelling locked
  against `to_string` over ±100k plus extremes; two-digit
  table halves the divisions). `#[inline]` hints stay, but
  forcing `inline(always)` on the unrolled digit cascade
  regressed 28% (4.67ms → 6.00ms) and was reverted — the
  calls cost less than the bloat. The infinity pre-scan fused
  into the emit loop: one pass over float bytes, wall-neutral
  within noise, kept as structure (single-infinite reports
  match the old column-major order exactly; multi-infinite
  order is now row-major (reviewer ruled: stays). Float values
  still format through their `String` backend — that swap
  belongs to the cell cycle, and it is the largest named
  remainder (float builds still cost ~2.4x int builds).
  Scratch A/B (`/tmp` probe, release profile as above,
  best-of-51, runs ordered now/base/now, byte-identity
  asserted across runs): 200k-integer builds 4.67/4.59ms now
  vs 12.68ms HEAD (2.8x); 200k-float builds 11.06/10.92ms now
  vs 16.89ms HEAD (1.5x). Output bytes identical on every run
  (10470009 / 10870010, matching the recorded 200k figures).
  Sheet allocs fall from 600,006 to 7.
- Priced and kept (same probe, temporary mutations, every one
  reverted, zero `unsafe` in the tree outside the reviewed
  CRC intrinsics): per-cell bounds checks cost ~2–4% on tall
  ints (4.59ms → 4.41/4.49ms unchecked). Kept: the price is
  `unsafe` on the hot path, and no measured percentage buys
  that without an explicit call — reviewer ruled: declined,
  checks stay. The `(name, payload)` tuple stays
  for the same reason in reverse: heterogeneous slices force
  per-column dispatch somewhere, so parallel arrays move the
  pairing at full API churn for zero measured gain — ingress
  shape belongs to the template cycle, which is the named
  future owner, not a vague later.
- Review II sizing (same machine/profile/commit): the `sheet_cap`
  width selection compiles to `cmp` plus `csel` (release aarch64
  assembler: ~13 scalar instructions per column, cold-only
  overflow exits) — predicated, no branch, so no mispredict and
  no vectorizer grievance. The max-column header-only build
  (16384 columns, the fold's longest run) costs 1484.7µs for
  941702 bytes. The match stays: this target has no AVX, and
  the checked arithmetic rule 8 requires refuses the portable
  vector transform without a measurement overruling it — none
  does.

## Cell-format benches (pre-cell-optimization, provisional)

One `cargo bench --bench fill_write` run on this machine (Darwin
25.6.0 arm64, Apple M4), bench profile (inherits `--release`:
`overflow-checks = true`, `codegen-units = 1`), commit `279adae`
plus the uncommitted 003 work. Wall times are criterion means;
output bytes come from a byte-size probe of the same recipes.
Cell writes are unoptimized at this point: these numbers verify
this layer (linearity across scales, per-type isolation against
the floor) and are provisional — the cell-optimization cycle
re-takes them, it does not inherit them as a baseline.

| bench | wall (mean) | output bytes | MB/s |
|---|---|---|---|
| `cells/tall_ints_1000` | 0.495ms | 48001 | 97.0 |
| `cells/tall_floats_1000` | 0.535ms | 50002 | 93.5 |
| `cells/tall_ints_20000` | 7.807ms | 990005 | 126.8 |
| `cells/tall_floats_20000` | 9.139ms | 1030006 | 112.7 |
| `cells/tall_ints_200000` | 82.43ms | 10470009 | 127.0 |
| `cells/tall_floats_200000` | 91.33ms | 10870010 | 119.0 |

No per-cell column on absolute totals: unitizing a total folds
fixed framework costs (headers, zip skeleton, central directory,
file open) into a fake unit cost — the 1k-to-200k drift in the
same code path proves it. The load-bearing figures are paired
total-time diffs at equal scale from the same run.

Same-run floor for the isolation delta: `tall_null_1000`
0.505ms / 48059 bytes / 95.2 MB/s, `tall_null_20000` 7.941ms /
990065 bytes / 124.7 MB/s, `tall_null_200000` 83.23ms /
10470071 bytes / 125.8 MB/s.

Format-cost isolation (paired total diffs, `cells` total minus
`floor` `tall_null` total, same run, equal scale): ints −0.010ms
/ −58 bytes at 1k, −0.134ms / −60 bytes at 20k, −0.806ms / −62
bytes at 200k; floats +0.030ms / +1943 bytes at 1k, +1.198ms /
+39941 bytes at 20k, +8.103ms / +399939 bytes at 200k. The
negative int diff is not free formatting: the pairs are
shape-mismatched (two blank columns per row vs one formatted),
so the framework does not fully cancel. Secondary normalized
view (diff-derived, formatted cell vs blank cell): ints +243ns /
+192ns / +204ns, floats +282ns / +258ns / +249ns at 1k / 20k /
200k. Exact divorce needs shape-matched pairs — a
single-blank-column floor bench — queued for the next bench
touch. Totals are linear in rows within each type; floats cost
more than ints at every scale.

Post-005 re-take, same machine, bench profile, commit
`1a3c0e5`, criterion means (change is criterion's own
comparison against its stored 003 baseline, all p < 0.05;
bytes are the unchanged 003 recipes):

| bench | wall (mean) | output bytes | MB/s |
|---|---|---|---|
| `cells/tall_ints_1000` | 0.164ms | 48001 | 292.6 |
| `cells/tall_floats_1000` | 0.200ms | 50002 | 249.9 |
| `cells/tall_ints_20000` | 0.870ms | 990005 | 1138.3 |
| `cells/tall_floats_20000` | 1.886ms | 1030006 | 546.1 |
| `cells/tall_ints_200000` | 10.18ms | 10470009 | 1029.0 |
| `cells/tall_floats_200000` | 17.07ms | 10870010 | 636.6 |

Changes: ints −66.7% / −88.5% / −87.7%, floats −63.0% /
−77.7% / −81.3% at 1k / 20k / 200k. These deltas bundle two
cycles — 004's hardware CRC (never re-taken for `cells`
until now) with 005's row sharing, integer writer, and scan
fusion. The scratch A/B isolates the 005 share on the
formatting path (2.8x ints, 1.5x floats in-memory,
byte-identical). Still pre-`f64`-backend: floats keep one
`String` per cell, and the ~1.7x float-vs-int gap at 200k
(17.07 vs 10.18ms) is the cell cycle's named target.

Post-006 (`f64` write-through: values format via `Display`
straight into the sheet, no temp `String` per cell), this
machine (Darwin 25.6.0 arm64, `RELEASE_ARM64_T8132`), bench
profile, commit `1a3c0e5` plus the uncommitted 006 work.
Criterion means (change is criterion's own comparison
against its stored baseline; bytes are the unchanged 003
recipes):

| bench | wall (mean) | output bytes | MB/s |
|---|---|---|---|
| `cells/tall_floats_1000` | 0.187ms | 50002 | 266.8 |
| `cells/tall_floats_20000` | 1.324ms | 1030006 | 777.9 |
| `cells/tall_floats_200000` | 12.57ms | 10870010 | 864.5 |

Changes vs the recorded Post-005 means (0.200ms, 1.886ms,
17.07ms): −6.3% / −29.8% / −26.3% at 1k / 20k / 200k — no
regression at any scale; removing the last per-cell
allocation on the float path moves every scale, and the
200k float-vs-int gap narrows to ~1.2x (12.57 vs 10.18ms).
Criterion's own calls: −5.56% at 1k and −35.0% at 20k
(both p < 0.05; the 20k baseline is its stored ~2.04ms).
The 200k re-take reads 12.57ms with a tight interval
([12.44, 12.74]ms). One earlier 200k pass read 19.95ms
with a [15.5, 25.6]ms interval (p = 0.34, no change
detected) under evident machine noise; the immediate
tight re-take above supersedes it. Output bytes are
unchanged: round-trip, verbatim-embed, golden, and pins
green unmodified, so float bytes are identical.

Post-007 (f64 tune-up: R1 comment trim and R2 import regroup on
the 006 surface; R3 keeps `write!(out, "{v}")`), this machine
(Darwin 25.6.0 arm64, `RELEASE_ARM64_T8132`), bench profile,
commit `1cbe856` plus the uncommitted 007 work. Criterion
means (change is criterion's own comparison against its stored
baseline; bytes are the unchanged 003 recipes):

| bench | wall (mean) | output bytes | MB/s |
|---|---|---|---|
| `cells/tall_floats_1000` | 0.187ms | 50002 | 266.9 |
| `cells/tall_floats_20000` | 1.257ms | 1030006 | 819.2 |
| `cells/tall_floats_200000` | 12.62ms | 10870010 | 861.0 |

Changes vs the recorded Post-006 means (0.187ms, 1.324ms,
12.57ms): +0.2% / −5.0% / +0.4% at 1k / 20k / 200k — no
regression at any scale. The 20k −5.0% is inter-session
variance, not a code effect: the shipped tree differs from
Post-006 only by deleted comment lines and reordered imports,
neither of which reaches codegen, and the two tight 20k
re-takes agree with each other (1.2556ms then 1.2573ms, CIs
[1.2408, 1.2739] and [1.2399, 1.2783]ms). Criterion's own
calls on the quoted runs: −0.62% at 1k (p = 0.45, no change;
the stored baseline there is the positional-spelling A/B run
below, so this is the within-noise tie reading), −24.3% at
20k and −20.7% at 200k (both p < 0.05, but their stored
baselines were churned by this cycle's own noisy passes, so
the recorded-mean comparison above is the load-bearing one).
R3 identity first: `/tmp` scratch probe over 4024 values (22
specials incl. both zeros, min-normal, max-subnormal
boundary, plus 4002 ±1000 quarter-steps) across `"{v}"`,
`"{}", v`, and `to_string()` bare plus both `write!`
spellings inside the `<v>…</v>` wrapper — zero mismatches;
signed-zero bytes `"-0"` on both `write!` spellings. Scratch
best-of-61 timing at parity (~200µs vs ~198µs, ~1.3%), and
the criterion 1k A/B agrees (inline 187.33µs vs positional
188.82µs, overlapping CIs) — tie, so the current spelling
stays per the tie rule; R3 ships no code change. Noisy
passes superseded the same way as Post-006's: one 20k pass
(1.71ms, CI [1.38, 2.15]ms, 22% outliers) and four 200k
passes (means 14.9–17.3ms, CIs spanning ~6ms, 11–21%
outliers) under evident machine noise; the tight re-takes
above (1k [185.47, 190.11]µs, 200k [12.48, 12.79]ms)
supersede them. Output bytes are unchanged: round-trip,
verbatim-embed, golden, and pins green on the shipped tree,
so float bytes are identical.

String/bool body cells have no benches yet: writer, store, and
store reader cover `I64`/`F64` only, so their benches wait on a
feature cycle.

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
