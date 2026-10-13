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
- `styled`: bold, colour, and bold-colour columns across mixed kinds,
  styles carried in the column declarations; values round-trip
  unchanged and the `s` indices are asserted on the bytes in
  `tests/styles.rs`.

Reserved, added only when the code supports them: inline strings
(including `&`, `<`, `>` escaping), bools, empty rows and columns,
date-as-string-or-number, and one fixed-size template example.

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

Spell-agreement rule (standing protocol from cycle 010): a number is
recorded only after two agreeing quiet-spell reads — same binary, same
spell, means within 3% of each other, each with a tight interval (CI
half-width under 5% of the mean) and severe-outlier share at or under
10%. Same spell means consecutive runs in one session with box state
re-checked between them; a spell interrupted by a load spike is a new
spell. Noisy passes are discarded and documented to the
Post-006/007/009 standard: discarded means, intervals, and outlier
shares quoted in the entry with a supersede note, never silently
dropped.

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

Post-009 (skeleton factor: the four tag spellings in
`i64_cell` / `f64_cell` into four tiny private `#[inline]`
helpers — `cell_open`, `blank_close`, `value_open`,
`value_close` — three entry groups, no mode flag), this machine
(Darwin 25.6.0 arm64, Apple M4), bench profile, commit
`77d2699` plus the uncommitted 009 work. Criterion means
(change is criterion's own comparison against its stored
baseline; bytes are the unchanged 003 recipes, re-probed at
bench scale this cycle: 48001 / 50002, 990005 / 1030006,
10470009 / 10870010):

| bench | wall (mean) | output bytes | MB/s |
|---|---|---|---|
| `cells/tall_ints_1000` | 0.165ms | 48001 | 291.6 |
| `cells/tall_floats_1000` | 0.185ms | 50002 | 270.6 |
| `cells/tall_ints_20000` | 0.881ms | 990005 | 1123.3 |
| `cells/tall_floats_20000` | 1.245ms | 1030006 | 827.5 |
| `cells/tall_ints_200000` | 8.248ms | 10470009 | 1269.4 |
| `cells/tall_floats_200000` | 12.553ms | 10870010 | 865.9 |

Changes vs the recorded means (ints Post-005: 0.164ms,
0.870ms, 10.18ms; floats Post-007: 0.187ms, 1.257ms,
12.62ms): ints +0.4% / +1.3% / −19.0%, floats −1.2% /
−1.0% / −0.5% at 1k / 20k / 200k — no regression at any
scale. (The 200k-int −19.0% is machine spread, not a code
effect: same-binary reads across the spell ran 8.25–10.64ms,
so no speedup is claimed — within noise, faster than
recorded.) Criterion's own calls on the quoted runs: ints
+0.8% (p = 0.45, no change) / −22.4% (p = 0.07, no change) /
−20.0% (p = 0.11, no change); floats −7.3% / −15.9% /
−36.1% (all p <= 0.05, but their stored baselines were
churned by this cycle's own noisy passes, so the
recorded-mean comparison above is the load-bearing one —
the Post-007 precedent).

Floor re-take, same machine/profile/commit, criterion
means: `floor/header_only` 143.56µs (+1.0% vs recorded
142.1µs; criterion +1.5%, p = 0.24, no change),
`floor/wide_null` 143.00µs (−2.1% vs 146.0µs; criterion
−0.3%, p = 0.74, no change), `floor/tall_null_1000`
175.46µs (+3.5% vs 169.5µs; criterion +1.4%, p = 0.86, no
change), `floor/tall_null_20000` 1.1095ms (−3.6% vs
1.151ms; criterion −8.3%, p = 0.47, no change),
`floor/tall_null_200000` 10.062ms (−12.7% vs 11.52ms;
criterion −6.4%, p = 0.75, no change; agreeing second
read 10.108ms).

Noisy passes superseded the same way as Post-006's and
Post-007's: the first full-group passes read high with
11–21% severe outliers and CIs spanning ±10–50%
(`cells/tall_floats_20000` 1.34–1.91ms,
`cells/tall_floats_200000` 14.37–19.64ms,
`floor/tall_null_200000` 21.39ms with CI [12.8, 32.7]ms).
The environment proof is a control, not a re-take: the
unmodified HEAD tree in a scratch worktree, benched in
the same spell, read `tall_floats_20000` 1.2727ms tight
([1.2548, 1.2942]ms, 13% outliers — the recorded 1.257ms
within 1.2%) while the factored tree was reading
1.34–1.91ms minutes earlier; the factored tree's
quiet-spell re-takes (1.2447ms [1.2289, 1.2641]ms,
12.553ms [12.367, 12.811]ms, both ≤9% outliers) agree
with the control, so the elevation was the spell, not
the change. Two same-spell reads that agree with each
other and with the recorded mean supersede the noisy
ones. Output bytes are unchanged: the `/tmp` byte-size
probe of the bench-scale recipes matches the 003 figures
byte for byte, and round-trip, verbatim-embed, golden,
and pins are green on the shipped tree.

Codegen note (`cargo rustc --release -- --emit asm`, same
machine): the helpers inline at three of the four
`cell_open` sites; the one outlined call is a blank arm
(`bl cell_open` followed immediately by the inlined
`"\"/>"`), and `blank_close` / `value_open` /
`value_close` leave no outline anywhere. No new branch
on any path — every helper body is straight-line pushes.
`#[inline]` hints only, per the carried ruling; no
`inline(always)`.

String/bool body cells have no benches yet: writer, store, and
store reader cover `I64`/`F64` only, so their benches wait on a
feature cycle.

Post-010 (harness harden: setup out of the clock, R1), this machine
(Darwin 25.6.0 arm64, Apple M4), bench profile (inherits `--release`:
`overflow-checks = true`, `codegen-units = 1`), commit `ae8eaae`
plus the uncommitted 010 work. Same binary throughout (direct-binary
runs, single filter, `--bench` flag; estimates mtimes confirmed moved
before quoting). Timed lines, reviewer-confirmed: the criterion
closure holds exactly
`thoth::xlsx::write_xlsx_file(&path, &cols).unwrap();` and nothing
else; the examples clock spans the same call between `Instant::now()`
and `elapsed` (`writer_columns()` and both debris removals sit above
the clock). Box at bench time: 42 days uptime, 1-minute load
1.3–3.3 across the spells, free pages ~410k–429k plus ~6–12k
speculative (16KB pages, `vm_stat`), disk 78% full.

| bench | read A (mean [CI], outliers) | read B (mean [CI], outliers) | output bytes | MB/s A / B |
|---|---|---|---|---|
| `cells/tall_floats_200000` | 11.580ms [11.410, 11.800], 9% (8% severe) | 11.714ms [11.497, 12.004], 13% (10% severe) | 10870010 | 938.7 / 928.0 |
| `cells/tall_ints_200000` | 6.578ms [6.404, 6.798], 8% (7% severe) | 6.646ms [6.464, 6.868], 9% (8% severe) | 10470009 | 1591.7 / 1575.4 |
| `floor/tall_null_200000` | no qualifying read (see gaps) | — | 10470071 | — |

Floats: full protocol pass. A and B are consecutive runs in one
session with box state re-checked between them (loads 2.30 / 2.40),
means 1.16% apart, both CIs tight (half-widths 1.7% / 2.2%),
severe-outlier shares 8% / 10%. Span 0.134ms, far under the 1.0ms
bar against the Post-009 same-binary span (2.39ms). Vs the recorded
Post-009 mean (12.553ms, CI [12.367, 12.811]): −7.8% / −6.7%, both
reads below the old interval — a moved clock, not a faster library:
`src/` is untouched and the bytes are identical (see below), so the
delta is harness accounting (setup left the clock) plus machine
state. Same speed, tighter clock; no library speedup claimed.

Ints: protocol gap named — A and B agree (1.02% apart, tight CIs at
3.0% / 3.1% half-width, severe shares 7% / 8%) and a third
qualifying read agrees with them (6.700ms [6.517, 6.926], 10%
total / 8% severe; three-read band 1.8%), all same binary with box
state re-checked — but noisy spells intervened, so A and B are not
consecutive runs and strict same-spell consecutiveness is not met.
Span 0.068ms. Both reads sit below the Post-009 same-binary spell
range (8.25–10.64ms); same attribution as floats, no library speedup
claimed.

Floor: no number claimed — no read in ~30 attempts met both gates
at once (best tight-CI reads 6.96ms [6.74, 7.21] / 7.04ms [6.85,
7.28] / 7.09ms [6.93, 7.28] all carried 12–15% outliers). Evidence
about the box, not a license to loosen the rule: the control is
re-taken next cycle. Lower edge held ~6.9–7.1ms all evening.

Discarded passes (superseded, Post-006/007/009 standard): five
full-group `200000` passes read high with wide CIs and 10–20%
outliers (ints means 7.76–11.88ms, CIs spanning ~5ms; floats
13.36–16.23ms; floor 7.08–10.52ms). Single-bench re-takes added
~25 ints attempts (means 6.45–23.69ms; all discarded on wide CIs
or 11–20% outliers except the three qualifying reads above),
~14 further floats attempts (11.58–18.12ms; the pair above the only
qualifiers) and ~24 further floor attempts (7.09–19.74ms, none
qualifying). Criterion's own change% calls are not load-bearing:
its stored baselines were churned by this cycle's noisy passes
(the Post-007/009 precedent). The unlink probe explains the moved
clocks: `/tmp` scratch, 200 unlinks of 10MB files on the same
volume as the bench temp dir — median 164µs, p90 503µs, mean
3.4ms with a heavy tail. The old harness timed two such unlinks
per iteration; their tail is the old clock's noise and much of its
level. Removing them narrows the span and lowers the mean by
accounting, not by code.

Byte-identity: `sh check.sh` with `CARGO_HOME=/tmp/cargo-home`
green; round-trip, verbatim-embed, golden
(`golden_small_archive_is_byte_identical`), pins
(`static_parts_are_pinned`, `sheet_cap_holds`), emit spelling
tests, and the four `sealed.rs` locks all green unmodified; `git
diff --stat tests/ src/` empty. Byte-size probe matches the 003
figures byte for byte: 48001 / 50002, 990005 / 1030006 (scratch
crate in `/tmp`), 10470009 / 10870010 (hardened `examples/bench.rs`
run, which also timed 7.2ms / 12.0ms single-pass, consistent with
the criterion quiet edges). One changed byte would have failed
the cycle; none changed.

Post-011 (multi-sheet: named sheets, per-sheet columns, fragment-cat
workbook parts), this machine (Darwin 25.6.0 arm64, Apple M4,
`RELEASE_ARM64_T8132`), bench profile (inherits `--release`:
`overflow-checks = true`, `codegen-units = 1`), commit `fe0e896`
plus the uncommitted 011 work. Same binary throughout
(`fill_write-ec72a5f783e16fae`, direct-binary runs, single filter,
`--bench` flag; estimates mtimes confirmed moved before quoting).
Timed lines, protocol-checked: the single-sheet closures hold
exactly the `write_xlsx_file` call and the multi-sheet closure
holds exactly `write_workbook_file` (sheets and columns built once
outside the clock in `benches/fill_write.rs` `multi_sheet`).
Box at bench time: 43 days uptime, 1-minute load 1.3–3.2 across
the spells, free pages ~21k–33k plus ~26k–30k speculative (16KB
pages, `vm_stat`), disk 78% full — materially busier and tighter
than the Post-010 box, which is the context for the gaps below.

| bench | read A (mean [CI], outliers) | read B (mean [CI], outliers) | output bytes | MB/s A / B |
|---|---|---|---|---|
| `cells/tall_floats_200000` | 11.671ms [11.507, 11.871], 9% (9% severe) | 11.765ms [11.570, 12.025], 8% (8% severe) | 10870010 | 931.3 / 923.9 |
| `cells/tall_ints_200000` | no qualifying pair (see gaps) | — | 10470009 | — |
| `floor/tall_null_200000` | no qualifying read (see gaps) | — | 10470071 | — |
| `fill_write/tall` (20k) | no qualifying pair (see gaps) | — | — | — |
| `multi_sheet/three_sheet_20000` | no qualifying pair (see gaps) | — | 2018110 | — |

Floats: full protocol pass. A and B are consecutive runs in one
spell with box state re-checked between them, means 0.81% apart,
both CIs tight (half-widths 1.6% / 1.9%), severe-outlier shares
9% / 8%. Vs the recorded Post-010 means (11.580 / 11.714ms):
+0.8% / +0.4% — within noise, no regression on the single-sheet
path. No library speedup claimed: the workbook-part assembly is
kilobytes against the 10.9MB sheet, and the bytes are identical
(see below).

Ints, floor, fill_write, multi_sheet: protocol gaps named, no
number claimed for them (the Post-010 floor precedent). Isolated
qualifying reads agree with the recorded means and with each
other's lower edges, so no regression is indicted, but strict
same-spell consecutiveness was not met and the means are not
recorded as baselines. Ints: one qualifying read 6.805ms
[6.500, 7.178] (half-width 5.0%, 8% severe) sits +3.5% / +2.4%
above the Post-010 pair (6.578 / 6.646ms); every other tight-CI
read carried 12–18% severe outliers, and the remaining ~45
attempts read 8–31ms with CIs spanning 15–87% of the mean.
Multi_sheet (new bench `multi_sheet/three_sheet_20000` in
`benches/fill_write.rs`: `tall_ints(20000)` as `First`,
`tall_floats(20000)` as `Second`, `header_only` as `Headers`;
2018110 bytes by `/tmp` probe): isolated qualifying reads
1.713ms [1.656, 1.787] (5% severe), 1.670ms [1.609, 1.769]
(4% severe), 1.647ms [1.618, 1.683] (10% severe), and 1.566ms
[1.545, 1.595] (3% severe) — mutually within ~5–9% with lower
bounds 1.545–1.656ms — but no two were consecutive qualifying
runs, so 012 inherits the bench code and must re-take its
baseline in a quiet spell. Floor and fill_write lower edges held
all evening (floor CI lower bounds 7.1–13.7ms on a ~7ms true
scale in Post-010 terms, fill_write/tall lower bounds
1.30–1.38ms); their means were inflated by the same tail.

Discarded passes (superseded, Post-006/007/009 standard, never
silently dropped): floats ~8 discarded reads (means
11.56–16.39ms; the tight ones failed only on 13–17% severe
outliers, the rest on CIs spanning 17–27%); ints ~60 attempts
(means 6.81–31.24ms, all discarded on wide CIs or 11–20%
severe outliers except the isolated qualifiers above); floor
~15 attempts (9.84–26.89ms, CIs spanning 27–86%); fill_write/tall
8 attempts (1.414–3.811ms, first read tight at [1.380, 1.454]ms
but 12% severe, the rest wide); multi_sheet ~25 attempts
(1.566–5.971ms, most wide-CI with lower bounds pinned at
1.54–1.65ms — the tail, not the build, moved). Criterion's own
change% calls are not load-bearing: its stored baselines were
churned by these noisy passes (the Post-007/009 precedent).

Byte-identity: `sh check.sh` green; round-trip and the new
`tests/multi_sheet.rs` three-sheet round-trip, per-sheet
verbatim-embed, golden (`golden_small_archive_is_byte_identical`),
pins (`static_parts_are_pinned` now fragment pins plus
cat-identity at 680/`0xCC79080A`, 282/`0xD4738FA2`,
424/`0x8A9642E0`; `sheet_cap_holds`, `workbook_parts_cap_holds`),
emit spelling tests, and the four `sealed.rs` locks all green.
`/tmp` probe at HEAD and post-011 agree byte for byte on the
single-sheet example (3501 bytes, crc `C47A1A82`) and at bench
scale (10470009 / 10870010). `src/zip.rs` is untouched. One
changed single-sheet byte would have failed the cycle; none
changed.

Post-012 (phase bench: emission / CRC + assembly / file write,
separately; harness only, `src/` untouched), this machine
(Darwin 25.6.0 arm64, Apple M4, `RELEASE_ARM64_T8132`), bench
profile (inherits `--release`: `overflow-checks = true`,
`codegen-units = 1`), commit `fe0e896` plus the uncommitted 011
and 012 work. Same binary throughout
(`fill_write-ec72a5f783e16fae`, direct-binary runs, single
filter, `--bench` flag; estimates mtimes confirmed moved before
quoting every number below). New group `phase` in
`benches/fill_write.rs` times, per recipe: (a) emission —
`sheet_xml` per sheet summed over the workbook's sheets, result
`black_box`ed, no file touched; (b) build — `build_workbook`,
no file touched. End-to-end (c) is re-used from the existing
`cells` / `floor` / `multi_sheet` groups, never re-implemented.
Recipes: `tall_ints`, `tall_floats`, `tall_null` (the `floor`
`all_null` recipe) at 20_000 and 200_000, plus the 011
three-sheet example at its bench scale (20k ints + 20k floats
+ header-only). Timed closures, in full: emission is exactly
`black_box(sheet_xml(cols).unwrap())` (three-sheet: one such
statement per sheet in a loop over the prebuilt sheet set);
build is exactly `black_box(build_workbook(sheets).unwrap())`.
Columns and sheet sets are built once outside the clock
(cycle 010); the emission allocation stays in the clock,
exactly as the library performs it. Byte counts from a `/tmp`
probe of the same recipes (sheet / archive): ints 986921 /
990005 at 20k and 10466925 / 10470009 at 200k; floats 1026922
/ 1030006 and 10866926 / 10870010; null 986981 / 990065 and
10466987 / 10470071; three-sheet sheets-sum 2014130 / archive
2018110 — the archive figures match the recorded 003 and
Post-011 bytes exactly. Box at bench time: 43 days uptime,
1-minute load 1.8–3.8 across the spells (file-write benches
drew the spikes), free pages ~51k falling to ~7k plus
~11k–16k speculative late in the session (16KB pages,
`vm_stat`), disk 78% full.

Claimed pairs (two agreeing quiet-spell reads each: means
within 3%, CI half-width under 5%, severe outliers ≤ 10%):

| bench | read A (mean [CI], severe) | read B (mean [CI], severe) |
|---|---|---|
| `phase/emit_tall_ints_200000` | 3.6492ms [3.6361, 3.6634], 9% | 3.6486ms [3.6379, 3.6594], 0% |
| `phase/build_tall_ints_200000` | 4.8045ms [4.7922, 4.8174], 0% | 4.8168ms [4.8047, 4.8293], 0% |
| `phase/emit_tall_floats_200000` | 8.6158ms [8.6003, 8.6358], 3% | 8.6057ms [8.6014, 8.6099], 0% |
| `phase/build_tall_floats_200000` | 9.8697ms [9.8637, 9.8755], 0% | 9.8778ms [9.8725, 9.8832], 0% |
| `cells/tall_floats_200000` (e2e) | 11.7170ms [11.5850, 11.8780], 6% | 11.7590ms [11.5680, 12.0230], 6% |
| `phase/emit_tall_null_200000` | 4.0559ms [4.0381, 4.0741], 0% | 4.0605ms [4.0365, 4.0860], 0% |
| `phase/build_tall_null_200000` | 5.2289ms [5.2116, 5.2472], 1% | 5.2040ms [5.1856, 5.2237], 0% |
| `phase/emit_three_sheet_20000` | 1.1524ms [1.1461, 1.1588], 0% | 1.1504ms [1.1450, 1.1559], 0% |
| `phase/build_three_sheet_20000` | 1.3838ms [1.3782, 1.3892], 0% | 1.3833ms [1.3781, 1.3885], 0% |
| `phase/emit_tall_ints_20000` | 0.3203ms [0.3194, 0.3214], 3% | 0.3194ms [0.3187, 0.3201], 2% |
| `phase/build_tall_ints_20000` | 0.4356ms [0.4339, 0.4375], 0% | 0.4291ms [0.4277, 0.4307], 0% |
| `phase/emit_tall_floats_20000` | 0.7894ms [0.7890, 0.7899], 1% | 0.7892ms [0.7890, 0.7895], 4% |
| `phase/build_tall_floats_20000` | 0.9069ms [0.9066, 0.9071], 0% | 0.9057ms [0.9054, 0.9060], 1% |
| `cells/tall_floats_20000` (e2e) | 1.1168ms [1.0998, 1.1384], 7% | 1.1216ms [1.1004, 1.1493], 10% |
| `phase/emit_tall_null_20000` | 0.3618ms [0.3607, 0.3631], 2% | 0.3614ms [0.3604, 0.3625], 1% |
| `phase/build_tall_null_20000` | 0.4683ms [0.4669, 0.4698], 1% | 0.4706ms [0.4692, 0.4721], 0% |
| `crc/crc32_1m` (anchor) | 0.0965ms [0.0962, 0.0967], 0% | 0.0959ms [0.0955, 0.0963], 0% |

Phase table (shares are ranges over the agreeing reads;
subtractions across benches are never single-point, per the
012 spec; rates are output MB/s — sheet bytes for emission,
archive bytes for build and end-to-end):

| recipe | emission | build − emission (CRC + assembly) | write − build (file + seal) | end-to-end |
|---|---|---|---|---|
| ints 200k | 3.6486–3.6492ms, 2868 MB/s, 54.2–54.8% † | 1.1553–1.1682ms, 17.2–17.5% † | 1.8406–1.9277ms, 27.3–29.0% † | gap † (1555–1573 MB/s †) |
| floats 200k | 8.6057–8.6158ms, 1261–1263 MB/s, 73.2–73.5% | 1.2539–1.2721ms, 10.7–10.9% | 1.8392–1.8893ms, 15.6–16.1% | 11.7170–11.7590ms, 924–928 MB/s |
| null 200k | 4.0559–4.0605ms, 2578–2581 MB/s, 57.0% ‡ | 1.1435–1.1730ms, 16.1–16.5% ‡ | 1.8898–1.9147ms, 26.5–26.9% ‡ | gap ‡ (1471 MB/s ‡) |
| ints 20k | 0.3194–0.3203ms, 3081–3090 MB/s | 0.1088–0.1162ms | — (gap) | gap |
| floats 20k | 0.7892–0.7894ms, 1301 MB/s, 70.4–70.7% | 0.1163–0.1177ms, 10.4–10.5% | 0.2099–0.2159ms, 18.7–19.3% | 1.1168–1.1216ms, 918–922 MB/s |
| null 20k | 0.3614–0.3618ms, 2728–2731 MB/s | 0.1065–0.1092ms | — (gap) | gap |
| 3-sheet 20k | 1.1504–1.1524ms, 1748–1751 MB/s, 68.2–68.3% ‡ | 0.2309–0.2334ms, 13.7–13.8% ‡ | 0.3033–0.3038ms, 18.0% ‡ | gap ‡ (1196 MB/s ‡) |

End-to-end gaps (†, ‡ — no number claimed, the Post-010/011
precedent): the file-write benches never produced two
consecutive qualifying reads this session — tight reads
carried 11–16% severe outliers or wide CIs, the same
filesystem tail the Post-010 unlink probe named. Isolated
qualifying reads, quoted as evidence and never as baselines:
† ints 200k 6.6574ms [6.4982, 6.8483] (8% severe) and 6.7322ms
[6.5544, 6.9545] (8% severe) — two reads agreeing within
1.1% of each other and within 1.3% of the recorded Post-010
pair (6.578 / 6.646ms), but not consecutive qualifying runs;
‡ null 200k best read 7.1187ms [6.9862, 7.2699] with a tight
CI failing only on 12% severe outliers; ‡ three-sheet
1.6871ms [1.6566, 1.7213] (8% severe), with a second tight
read 1.6510ms [1.6255, 1.6812] failing only on 11% severe.
Shares marked †/‡ divide by those isolated reads and inherit
their status: corroboration, not claimed baselines. The gate
below stands on the claimed floats pairs, corroborated by
the †/‡ ranges, which nowhere cross a verdict.

Anchors, quoted beside the subtractions (spec R2). CRC: the
claimed `crc32_1m` pair reads 0.0959–0.0965ms per MiB =
0.0915–0.0920 ns/B (~10.9 GB/s, against the Post-004 recorded
0.087 ns/B). Applied to each sheet byte count it predicts
the CRC inside (build − emission): ints 200k 0.960ms of the
measured 1.1553–1.1682ms (82–83%), floats 200k 0.997ms of
1.2539–1.2721ms (78–80%), null 200k 0.960ms of
1.1435–1.1730ms (82–84%), three-sheet 0.185ms of
0.2309–0.2334ms (79–80%), and 78–85% at every 20k scale.
The remainder — the sheet→archive copy in `ZipWriter::finish`
plus part staging — is 0.1950–0.2079ms (ints 200k),
0.2569–0.2751ms (floats), 0.1832–0.2127ms (null),
0.0461–0.0486ms (three-sheet). No subtraction disagrees with
its anchor beyond the CIs: the anchor sits inside every
(build − emission) range with a small positive copy
remainder, so no anchor-override is invoked. Floor: the
`tall_null` emission pair (4.0559–4.0605ms at 200k, two
columns of blanks) bounds skeleton emission — blank-cell
emission alone costs more than the whole integer emission
(3.6486–3.6492ms, one column with values), so at 200k the
float emission premium over skeleton-and-tags is at least
~4.5ms of the 8.6057–8.6158ms float emission (shape caveat:
null carries two columns per row against the floats' one,
so the bound is generous and the premium is a floor, not a
split).

R4 peak-memory probe (`/tmp` scratch crate against `thoth`
by path, release profile as above; method: whole-process
peak RSS from `/usr/bin/time -l` "maximum resident set size",
three runs each, bytes held alive to exit): on the 200k
3-sheet recipe (ints 200k + floats 200k + header-only;
sheets-sum 21334138 bytes, archive 21338118 bytes),
`build_workbook` peaks at 47611904–47628288 bytes and
end-to-end `write_workbook_file` at 47628288 bytes — the
file write adds no measurable residency; the peak is set
inside the build. The double residency 013 would remove is
the sheets and the archive co-resident at the `finish` copy:
21334138 + 21338118 = 42672256 live bytes at that moment,
of which the sheet copies are the removable half. (RSS
counts touched pages: the sheet buffers are
capacity-allocated above their lengths by `sheet_cap`, so
allocator high-water would read higher; the figure above
is the RSS method, named as such.)

The 013 decision (spec R3 gate: a fusion is `go` only at
≥ 15% of end-to-end on at least two recipes at 200k on
agreeing reads). Phase ranking at 200k: emission first
(54.2–54.8% † ints, 73.2–73.5% claimed floats, 57.0% ‡
null), file write second (15.6–16.1% claimed floats,
27.3–29.0% †, 26.5–26.9% ‡), CRC + assembly third
(10.7–10.9% claimed floats, 17.2–17.5% †, 16.1–16.5% ‡),
split by the anchor into CRC (8.5% claimed floats,
14.3–14.4% † ints, 13.5% ‡ null, 11.0% ‡ three-sheet at
20k scale) and copy + staging (2.2–2.3% claimed floats,
2.9–3.1% †, 2.6–3.0% ‡). Verdicts: (i) emit-into-archive —
**no-go**: its removable share is the copy + staging range,
2.0–3.1% of end-to-end wherever an end-to-end denominator
exists (floats claimed at both scales; ints, null, and the
three-sheet on their †/‡ denominators), an order under
the gate. (ii) incremental CRC during emission
— **no-go**: the CRC share reaches at most 14.3–14.4%
(ints 200k, on the † denominator; 14.5% even at the most
favorable edge of the anchor and read ranges), 13.5% ‡ on
null, 11.0% ‡ on the three-sheet, and 8.5% on the claimed
floats pair — no recipe reaches 15%, let alone two.
(iii) **neither, then**: emission itself owns the time on
every recipe, so 013's target is the emitter (float/digit
spelling, per-push cost), and both fusions are recorded
no-go so 013's scope shrinks accordingly (its applicator
records the shrink in `DECISION.md`). Completeness note:
the two fusions combined would remove all of
(build − emission), 17.2–17.5% † / 16.1–16.5% ‡ at 200k —
over the gate only as a bundle and only off the
unclaimed †/‡ denominators; the spec gates the candidates
individually, and individually neither clears it. The file
write is the second-largest phase (up to ~29% †) but is
not a 013 candidate under R3; it is one sealed `fs::write`
plus rename already, and its share is recorded here for
the next spec author, not acted on.

Discarded passes (superseded, Post-006/007/009 standard,
never silently dropped): every mis-qualified read was an
end-to-end file-write bench; all 15 `phase`/`crc` benches
(14 phase + the `crc32_1m` anchor) qualified on their
first pair, as did both floats end-to-end benches (one extra
`emit_tall_ints_20000` smoke read, 0.32224ms [0.32140,
0.32312], agrees with its pair). ints 200k e2e: 10 runs,
8 discarded (means 8.0255–17.6630ms, CIs spanning
15–41% of the mean, severe 8–16%) around the two † reads.
null 200k e2e: 8 runs, all discarded (7.1187–25.6170ms;
the best failing only on 12% severe, the rest on CIs
spanning 18–66%). three-sheet e2e: 4 runs, 3 discarded
(1.6510–1.9254ms, severe 11–15%). ints 20k e2e: 4 runs,
all discarded (0.7114–0.8236ms, severe 11–15%). null 20k
e2e: 4 runs, all discarded (0.6980–1.2711ms, severe
8–13%). Criterion's own change% calls are not
load-bearing: its stored baselines were churned by these
noisy passes (the Post-007/009 precedent).

Byte-identity: `sh check.sh` green (see the producer's
final run); round-trip, per-sheet verbatim-embed, golden
(`golden_small_archive_is_byte_identical`), pins
(`static_parts_are_pinned`, `sheet_cap_holds`,
`workbook_parts_cap_holds`), emit spelling tests, the
four `sealed.rs` locks, and `tests/multi_sheet.rs` all
green unmodified; the `/tmp` byte probe matches the 003
figures byte for byte (990005 / 1030006, 10470009 /
10870010) and the Post-011 three-sheet figure (2018110).
`git diff` on `src/` is byte-identical to its pre-012
state (the uncommitted 011 work only); 012 touches only
`benches/fill_write.rs` (one new group, `phase`) and this
entry. One changed library byte would have failed the
cycle; none changed.

Post-014 (arena: one buffer, emitted once, checksummed as it
goes), this machine (Darwin 27.0.0 arm64, Apple M4,
`RELEASE_ARM64_T8132`), bench profile (inherits `--release`:
`overflow-checks = true`, `codegen-units = 1`), rustc 1.93.1,
commit `2a51dc9` plus the uncommitted 014 work. Box during the
spells: 1-minute load 1.7–2.7. Criterion reads are direct-binary
runs (`fill_write-ec72a5f783e16fae`, single filter, `--bench`
flag; estimates mtimes confirmed moved before quoting). The
scratch probes lived in `examples/` only for the measurement
and were removed before delivery; the HEAD tree for the A/B
reads is a `git archive` extraction in `/tmp/thoth-head`.

(a) Byte-identity held. `sh check.sh` green (33 unit tests,
round-trip, multi-sheet, and the four `sealed.rs` locks);
verbatim-embed, cat-identity, `static_parts_are_pinned`,
`sheet_cap_holds`, `workbook_parts_cap_holds`, `archive_fits`
ceiling tests, and the `golden_small_archive_is_byte_identical`
lock all pass — the golden lock's expected 207 bytes unchanged,
its construction re-spelled through the arena primitives. A
`/tmp` probe compared `build_workbook` output from the HEAD and
post-014 binaries byte for byte (`cmp`) on ints, floats, null,
and mixed single sheets and the 200k three-sheet recipe at
200_000 rows: identical on every recipe (archive lengths
10470009 / 10870010 / 10470071 / 16914521 / 21338118).

(b) Peak memory (Post-012 R4 method: whole-process peak RSS
from `/usr/bin/time -l`, three runs each, bytes held alive to
exit) on the 200k three-sheet recipe (sheets-sum 21334138
bytes, archive 21338118): HEAD peaks at 47923200 bytes on all
three runs (the Post-012 recorded 47611904–47628288), post-014
at 26558464 on all three. The sheets-plus-archive double
residency (42672256 live bytes at the old `finish` copy) is
gone; the new peak is the one arena (24805050 capacity) plus
the borrowed ingress. Retained capacity versus length on the
returned buffer, same probe: ints 12203418 / 10470009 (1.17x),
floats 12603419 / 10870010 (1.16x), mixed 20203485 / 16914521
(1.19x), three-sheet 24805050 / 21338118 (1.16x) — and null
20203485 / 10470071 (1.93x): the hint spells typical values
while blank rows emit about half of it, so a null-heavy caller
retains about twice the archive length. That figure is
recorded here for the successor's decision (spec R2); no
shrink step is taken this cycle.

(c) Build time. Criterion phase re-take, agreeing pairs (means
within 3%, CI half-width under 5%, severe outliers ≤ 10%):

| bench | read A (mean [CI], severe) | read B (mean [CI], severe) | Post-012 pair | delta |
|---|---|---|---|---|
| `phase/build_tall_ints_200000` | 3.6122ms [3.6037, 3.6214], 1% | 3.6419ms [3.6270, 3.6578], 3% | 4.8045 / 4.8168ms | −24% |
| `phase/build_tall_floats_200000` | 8.9042ms [8.8989, 8.9095], 0% | 8.8957ms [8.8886, 8.9047], 1% | 9.8697 / 9.8778ms | −10% |
| `phase/build_tall_null_200000` | 4.1103ms [4.0903, 4.1331], 9% | 4.0304ms [4.0256, 4.0360], 3% | 5.2289 / 5.2040ms | −21 to −23% |
| `phase/build_three_sheet_20000` | 1.1689ms [1.1675, 1.1703], 0% | 1.1663ms [1.1650, 1.1677], 1% | 1.3838 / 1.3833ms | −16% |

Emission (the unchanged `sheet_xml` oracle) beside them: ints
3.6359 / 3.6152ms (Post-012 3.6486 / 3.6492, within 1%),
floats 9.4328 / 9.3920ms (Post-012 8.6057 / 8.6158, +9%),
null 4.2102 / 4.2338ms (Post-012 4.0559 / 4.0605, +4%). The
float and null emission deltas are cross-session drift on
untouched code — ints emission, measured the same way in the
same spells, agrees with Post-012 — so the load-bearing build
comparison is the interleaved A/B below, and the deltas above
are reported, not gated (spec R6). Interleaved cross-binary
A/B (`/tmp` probe, HEAD vs post-014 binaries alternated, five
rounds of 15 builds each, medians of per-round medians):
ints 4.725ms -> 3.748ms (−21%), floats 10.324ms -> 8.704ms
(−16%), null 5.199ms -> 4.231ms (−19%). All three sit above
the prototype's recorded range (ints ~8.5–11%, floats
~7.7–9%, nulls ~2–3%); no recipe regresses on any read.

Emission parity (R4), same interleaved probe method (9 rounds
of 15 emissions, rotating order, byte-identity asserted first):
the byte emitter reads within noise of the `sheet_xml` oracle
on ints across ten spells (−5% to +5%, centred on parity),
about 2–5% faster on floats, and a consistent ~0.6–3.8%
slower on nulls — the prototype's own tuned figure (~2%),
recorded as the remaining emitter deficit rather than hidden.
One tuning attempt is rejected on its numbers: a precomputed
per-column cell prefix read 6–17% slower on every recipe
(runtime-length appends lose to constant-length ones) and was
reverted before delivery.

Discarded reads (never silently dropped): one
`build_tall_ints_200000` read at 3.6099ms carried 17% outliers
and was discarded for the qualifying pair above; criterion's
own change% calls are not load-bearing, its stored baselines
having been churned by these runs (the Post-007/009
precedent). `ZipWriter` is retired as a public type and the
`zip` module is crate-internal; the `crc` bench group is
dropped with it, CRC now measured in context as build minus
emission in the `phase` group. No whole-sheet `crc32` call
survives on the build path: sheets fold per row over the
arena range, small parts checksum their just-emitted range,
and `crc_update_any_split_matches_one_shot` pins every split
(including empty chunks) against the one-shot spelling.

Applicator re-check (cycle 014, same machine and bench profile,
commit `2a51dc9` plus the uncommitted 014 work and the applicator's
`build_workbook` split; details in `specs/014-arena/DECISION.md`):
`cargo fmt` applied and clean, `sh check.sh` green (format, 33 unit
tests, round-trip, multi-sheet, four sealed tests, clippy
`-D warnings`, docs). Byte-identity re-probed in the final tree:
`build_workbook` dumps at 200k rows on ints, floats, null, and mixed
single sheets and the three-sheet recipe are byte-identical to the
HEAD dumps (archive lengths 10470009 / 10870010 / 10470071 /
16914521 / 21338118), retained capacities unchanged (null
20203485 / 10470071 = 1.93x). Build-time sanity in an elevated
session (1-minute load 1.7–1.9, rising across the spells; medians
of 15 builds): ints 4.19–4.38ms, floats 9.74–10.20ms, null
4.29–4.46ms — uniformly above the quiet-spell reads above and
slowest on the untouched float path, with null (the recipe most
exposed to the split) retaining the largest gain against its
Post-012 build pair (5.20–5.23ms); no split-attributable regression
is read. The R4 emitter-parity re-probe lives in DECISION.md.

Post-015 (template contract: one contract, dynamic and static
impls, hint or named unknown through the declaration), this
machine (Darwin 27.0.0 arm64, Apple M4, `RELEASE_ARM64_T8132`),
bench profile (inherits `--release`: `overflow-checks = true`,
`codegen-units = 1`), rustc 1.93.1, commit `c03f374` plus the
uncommitted 015 work. Box during the spells: 1-minute load
1.25–2.11, free pages ~75k (16KB pages, `vm_stat`, one check).
Probes are `/tmp` scratch crates against `thoth` by path (HEAD
from a `git archive` extraction in `/tmp/thoth-head`); the
timed probe alternates HEAD and post-015 binaries in one shell
(interleaved A/B, five rounds of 15 builds per recipe per run,
medians of per-round medians quoted as ranges over three
interleaved runs, byte-identity asserted before timing).

(a) Byte-identity held. `cargo fmt` applied, `sh check.sh`
green (format, 33 unit tests plus round-trip, multi-sheet,
sealed, and the 6 new template tests, clippy `-D warnings`,
docs); `git diff -- src/emit.rs` is empty and `sheet_xml`
remains the oracle (`sheet_xml_template` over a known dynamic
template). The `/tmp` probe compared `build_workbook` /
`build_xlsx` output byte for byte (`cmp`) against HEAD at
200_000 rows on ints, floats, null, and mixed single sheets
and the three-sheet recipe (ints 200k + floats 200k +
header-only): identical on every recipe (archive lengths
10470009 / 10870010 / 10470071 / 17047851 / 21338118; the
mixed length differs from Post-014's 16914521 only because
this probe's mixed recipe pairs `0..n` ints with `i + 0.5`
floats — HEAD and post-015 agree on the recipe as built).

(b) Retained capacity, same probe (returned `Vec` capacity /
length, known shape): ints 12203418 / 10470009 (1.17x),
floats 12603419 / 10870010 (1.16x), mixed 20203485 /
17047851 (1.19x), three-sheet 24805050 / 21338118 (1.16x) —
all unchanged from HEAD — and null 20203485 / 10470071
(1.93x), the Post-014 null-heavy retention reported again,
not silently inherited: the hint still spells typical values
while blank rows emit about half of it, and whether the hint
learns blank-heavy columns stays the successor's decision
(spec R3), not taken here. Unknown-rows spelling of the same
sheets (builder starts from `ARENA_BLOCK`, 8192 bytes, and
grows): ints 11546624 / 10470009 (1.10x), null 11546624 /
10470071 (1.10x), mixed 23093248 / 17047851 (1.35x), with
bytes asserted identical to the known spelling in the probe
and in `tests/template.rs`.

(c) Build time, interleaved A/B (HEAD first, then reversed;
the ranges are the three interleaved medians):

| bench (200k) | HEAD | post-015 | read |
|---|---|---|---|
| `build_ints` | 4.14–4.23ms | 3.83–3.92ms | below HEAD, not claimed* |
| `build_floats` | 9.12–9.14ms | 8.90–8.93ms | below HEAD, not claimed* |
| `build_null` | 4.42–4.60ms | 4.45–4.52ms | parity |
| `build_three_sheet` | 13.29–13.34ms | 12.89–13.00ms | below HEAD, not claimed* |
| `emit_ints` (oracle) | 3.56–3.78ms | 3.59–3.75ms | parity |
| `emit_floats` (oracle) | 9.25–9.26ms | 9.19–9.36ms | parity |

\*No speedup is claimed for the contract: `src/emit.rs` is
untouched and the emission drivers re-express the same loop,
so sub-HEAD build reads are code-layout drift between the two
binaries, recorded because no recipe regresses on any
interleaved read. One discarded spelling is named, not
dropped: the first trait draft indexed the template per cell
and read null builds ~10–15% and float builds ~5% over HEAD
across five interleavings; the shipped drivers gather each
sheet's `ColumnView`s once before the row loop (the pre-015
slice march) and the table above supersedes those reads.
An isolated null probe (25 builds per round, emit beside
build) confirmed the fix: emit at parity (4.00–4.09ms both
sides) and build within the interleaved ranges above.

(d) Growth-path pricing (same probe binaries, known beside
unknown in one binary): ints 4.29–4.34ms unknown vs
3.83–3.92ms known (~+12%), null 4.93–5.00ms vs 4.45–4.52ms
(~+10%), mixed in a paired 15-build probe 11.47 / 11.95ms
unknown vs 10.53 / 11.29ms known (~+6–9% over two runs).
Block growth is priced, not free: it costs high single digits
to ~12% at 200k and buys the named-unknown declaration, with
byte-identity either way.

(e) Static impl, measured plainly. The example
(`examples/static_sheet.rs`, `StaticSheet<2, 3>`, 3569-byte
file, `cargo run --release --example static_sheet`) and the
probe's batched fixed-shape builds (2000 builds per round,
five rounds): `build_static_fixed` medians 1.030–1.033µs vs
`build_dynamic_fixed` 1.043–1.062µs for the dynamic spelling
of the same sheet — within noise at this shape. End-to-end
`write_static_fixed` (batch of 200 sealed writes per round):
0.136–0.141ms per 3569-byte file (~26 MB/s, filesystem
dominated). Compile-time shape buys nothing measurable at
the example's shape; the static impl stands on ergonomics
and the DESIGN promise, not on an invented speed claim.
The `static_template` criterion group in
`benches/fill_write.rs` carries these three benches in-tree.

Post-016 (emitter: the spellings, measured one at a time), this
machine (Darwin 27.0.0 arm64, Apple M4, `RELEASE_ARM64_T8132`),
bench profile (inherits `--release`: `overflow-checks = true`,
`codegen-units = 1`), rustc 1.93.1, commit `3c98e3b` plus the
uncommitted 016 work. Box during the spells: 1-minute load
1.5–3.1. Method for every candidate, in order (SPEC R2):
`/tmp` probe crates against `thoth` by path — HEAD from a
`git archive` extraction in `/tmp/thoth-head`, candidate from
the worktree — interleaved A/B with rotating order, seven
rounds of 15 builds (emissions for C3) per side, the quoted
figure the median of the round medians; archive byte-identity
(`cmp` against the HEAD dumps at 200k on ints, floats, null,
mixed, and the three-sheet recipe) asserted before any timing.
No sequential Criterion read is used as close-call evidence.

- **C1 — float append shape: reverted.** Format into a fixed
  512-byte stack scratch (`fmt::Write`, widest `Display`
  spelling is 327 bytes), then one arena append, write-through
  fallback on scratch overflow. Byte-identical on every recipe
  and over the extended probe set (below). Interleaved floats
  build at 200k: HEAD 9.195ms (rounds 9.159–9.236) vs candidate
  10.185ms (10.151–10.238), **+10.8%** — the scratch copy and
  its per-cell zeroing cost more than the per-piece capacity
  checks it removes. Non-target reads in the same spells were
  within noise (ints −2.5%, null −4.8% on three rounds, layout
  drift on an untouched path). Loser reverted; the extended
  `byte_sink_matches_string_oracle` pin is kept (specials, both
  zeros, subnormal boundary, quarter-steps, plus the new
  301-digit positional extremes `1e308`, `1.5e300`, the
  `2.225073858507201e-308` min-normal and the `1e-310`
  subnormal) and passes on the incumbent spelling.
- **C2 — blank path: reverted.** Candidate named, not a
  rejected shape: the common `"<c r=\""` plus letters prefix
  stays single and shared, and only the blank arm fuses the
  shared row digits and `"\"/>"` into one append from a
  16-byte stack tail (longer rows fall back to two appends) —
  blank appends 4 → 3, no whole-cell stack assembly, no
  duplicated cell open. Byte-identical on every recipe.
  Interleaved null build at 200k: HEAD 4.518ms (4.480–4.672)
  vs candidate 4.814ms (4.675–4.949), **+6.6%** — well outside
  the recorded ~1–3% emitter deficit's noise band, in the
  wrong direction. Reverted; the null deficit stands as
  recorded in Post-014.
- **C3 — oracle digit respelling: reverted.** Candidate:
  `push_magnitude`'s finished digits append as one slice
  (`from_utf8` on the ASCII-by-construction buffer, per-char
  fallback unreachable), `int_spelling_matches` sweep
  unchanged, build-path bytes untouched and confirmed
  identical. Interleaved oracle emission (`sheet_xml`) on
  ints at 200k: HEAD 3.902ms (3.737–4.042) vs candidate
  4.777ms (4.548–4.917), **+22.4%** — the validation and the
  wider, uninlined body cost more than the per-char pushes.
  Reverted; the oracle keeps its digit-by-digit spelling and
  stays the parity benchmark as it stands.
- **C4 — `unsafe` cursor: refused, no candidate entered.**
  After C1–C3 no remaining candidate carries a measurement
  naming the nanoseconds it buys over the best safe spelling
  (each safe spelling measured slower than its incumbent), so
  no `unsafe` enters this cycle, per the cycle-004 precedent.
  The refusal is recorded here for the applicator's
  `DECISION.md`.
- **Drivers and boundaries held.** `emit_sheet` /
  `emit_sheet_arena` in `src/contract.rs` are untouched (no
  shipped signature, so no forced call-site change);
  `src/xlsx.rs`, `src/workbook_parts.rs`, `src/zip.rs`,
  `src/macros.rs` untouched; no new dependencies. The only
  shipped tree change is the C1 test-pin extension above.
- **Final build reads** (final tree, same interleaved probe,
  columns built once outside the clock): floats HEAD 9.087ms
  vs final 9.075ms (−0.1%), ints 3.939 vs 3.814ms (−3.2%,
  overlapping rounds, parity), null 4.429 vs 4.421ms (−0.2%);
  oracle emission ints 3.823 vs 3.856ms (+0.9%), floats 9.289
  vs 9.242ms (−0.5%). Against the Post-015 baselines (ints
  3.83–3.92ms, floats 8.90–8.93ms, null 4.45–4.52ms) the
  final tree reads at parity on every recipe — expected, as
  no emitter code shipped, and no speed is claimed for this
  cycle. `cargo fmt` applied, `sh check.sh` green (format,
  tests, clippy `-D warnings`, docs).

Post-017 (primitive kinds: strings and bools through the
contract), this machine (Darwin 27.0.0 arm64, Apple M4,
`RELEASE_ARM64_T8132`), bench profile (inherits `--release`:
`overflow-checks = true`, `codegen-units = 1`), rustc 1.93.1,
commit `1482e88` plus the uncommitted 017 work. Criterion reads
are direct-binary runs (`fill_write-ec72a5f783e16fae`, single
filter, `--bench` flag; estimates mtimes confirmed moved).
Box during the spells: 1-minute load 2.2–3.4. New recipes are
in the `phase` group beside the existing ones: tall strings
(one `str` column, 200_000 rows, 12-char texts `value-` plus
six digits) and tall bools (one `bool` column, alternating
`Some(true)`/`Some(false)`), built once outside the clock.

| bench (200k) | read A (mean [CI]) | read B (mean [CI]) | archive bytes |
|---|---|---|---|
| `phase/build_tall_strings_200000` | 11.678ms [11.665, 11.693] | 11.628ms [11.615, 11.642] | 16381122 |
| `phase/build_tall_bools_200000` | 3.2514ms [3.2399, 3.2631] | 3.2001ms [3.1929, 3.2074] | 10781120 |
| `phase/emit_tall_strings_200000` | 9.1769ms [9.1642, 9.1897] | 9.1264ms [9.1182, 9.1347] | — |
| `phase/emit_tall_bools_200000` | 3.0499ms [3.0448, 3.0553] | 3.0364ms [3.0327, 3.0402] | — |

Both build pairs agree (0.4% strings, 1.6% bools) with tight
intervals on the final binary; that is ~1406 MB/s (strings)
and ~3340 MB/s (bools) at archive size. No speed claim beyond
these reads: the new kinds have no prior baseline. Superseded
bool reads are named, not dropped: an earlier pair on the
pre-outline binary read 3.3637 / 3.2724ms and a later pair
3.2971 / 3.4259ms (3.9% apart) under a busier box; the final
binary's agreeing pair above supersedes them.

Existing recipes unregressed, two ways. (a) Byte-identity:
a `/tmp` probe compared `build_workbook` output from a
`git archive` HEAD extraction and the post-017 tree byte for
byte (`cmp`) at 200_000 rows on ints, floats, null, mixed,
and the three-sheet recipe — identical on every recipe
(10470009 / 10870010 / 10470071 / 17047851 / 21338118).
(b) Time: an interleaved HEAD/post-017 probe (three rounds
of 15 builds per recipe, alternating order) reads ints,
floats, null, and three-sheet at parity (round medians
overlap on every recipe; null HEAD 4.35–4.72ms vs post-017
4.24–4.45ms). One layout note from that probe: with
`#[inline]` on the four new cell spellings, null builds read
a consistent ~8% over HEAD interleaved; outlining the new
spellings (same-crate callers still inline where LLVM
chooses) restored parity, so the new cell functions ship
without the hint. Final-binary criterion phase builds
corroborate, cross-session drift included: ints
3.8936 / 3.8403ms, floats 8.9009 / 8.9206ms, null
4.5128 / 4.5201ms, three-sheet (20k) 1.2036 / 1.1926ms —
against Post-014's pairs (3.6122 / 3.6419, 8.9042 / 8.8957,
4.1103 / 4.0304, 1.1689 / 1.1663) floats sit on the recorded
means and the rest within the cross-session spread those
entries already document; the interleaved reads are the
load-bearing comparison and no recipe regresses on them.

Retained capacity (returned `Vec` capacity / archive length,
same `/tmp` probe, known shape): strings 17003421 /
16381122 (1.04x), bools 11403419 / 10781120 (1.06x),
ints-plus-strings-plus-bools 31403554 / 29047915 (1.08x) —
the witnessed-length hint reserves string sheets near their
actual size. Existing recipes unchanged: ints 12203418 /
10470009 (1.17x), floats 12603419 / 10870010 (1.16x), mixed
20203485 / 17047851 (1.19x), three-sheet 24805050 /
21338118 (1.16x), and null 20203485 / 10470071 (1.93x),
the known blank-heavy retention, reported again not fixed
here. `sheet_cap_holds` now covers bool and string columns
(escapable, padded, empty, blank, and long repeated texts)
and holds.

Post-018 (styles: bold and fill colour through the contract), this
machine (Darwin 27.0.0 arm64, Apple M4, `RELEASE_ARM64_T8132`),
bench profile (inherits `--release`: `overflow-checks = true`,
`codegen-units = 1`), rustc 1.93.1, commit `3c1c266` plus the
uncommitted 018 work. Criterion reads are direct-binary runs
(`fill_write-ec72a5f783e16fae`, single filter, `--bench` flag;
estimates mtimes confirmed moved before quoting). The interleaved
probe is a `/tmp` scratch crate against `thoth` by path (HEAD from
a `git archive` extraction in `/tmp/thoth-head-018`), alternating
HEAD and post-018 binaries, three rounds of 15 builds per recipe,
medians of each round quoted as ranges, byte-identity asserted
before any timing.

(a) Byte-identity for unstyled sheets held. A `/tmp` probe compared
every archive part from HEAD and post-018 binaries at 200_000 rows
on ints, floats, null, mixed, strings/bools, and the three-sheet
recipe: every part is byte-identical except `xl/styles.xml`, which
grows 708 -> 2235 bytes (new pin 2235/`0x76B2525A`, recaptured in
`static_parts_are_pinned`), so every archive grows by exactly the
1527 styles bytes (ints 10470009 -> 10471536, floats 10870010 ->
10871537, null 10470071 -> 10471598, mixed 17047851 -> 17049378,
strings/bools 22870074 -> 22871601, three-sheet 21338118 ->
21339645). Sheet bytes, headers included, do not change. Styled
spellings are pinned the other way: the emitter sweep in
`src/emit.rs` covers every kind valued and blank at bold-only and
every colour plain and bold (indices 1..=13) byte-for-byte between
the oracle and arena spellings, `tests/styles.rs` asserts the `s`
indices per cell (valued and blank), the palette fills in order,
and the 14-entry `cellXfs` table on the stored parts, and the
styled example's values round-trip unchanged.

(b) Retained capacity (returned `Vec` capacity / archive length,
same probe, known shape), styled recipes included per R4:
ints 12204945 / 10471536 (1.17x), floats 12604946 / 10871537
(1.16x), mixed 20205012 / 17049378 (1.19x), three-sheet 24806577 /
21339645 (1.16x), strings/bools 23805015 / 22871601 (1.04x) — all
unchanged ratios from HEAD, each capacity exactly 1527 over its
HEAD figure for the larger styles part — and null 20205012 /
10471598 (1.93x), the known blank-heavy retention, again not
fixed here. The styled bench recipe below: styled 44205148 /
40427284 (1.09x) against its unstyled twin 39405148 / 35627284
(1.11x); the capacity difference is exactly the 4,800,000 `s`
bytes, so the hint prices the exact index spelling per cell.
`sheet_cap_holds` now covers styled columns of every kind at the
widest index and holds.

(c) Existing recipes on the final binary, criterion `phase`
build pairs (two agreeing reads each): ints 3.9613 / 3.9117ms,
floats 9.0332 / 9.0930ms, null 4.5808 / 4.5911ms, against the
Post-017 final-binary pairs (ints 3.8936 / 3.8403ms, floats
8.9009 / 8.9206ms) — ints and floats within 2% of the recorded
means, no regression claimed or read beyond that. The 017
null-timing discrepancy stays unquoted: no cross-binary null
delta is recorded here, and the null pair above is a
final-binary read only. The interleaved probe corroborates on
the recipes it can price without that question: ints post-018
3.686–3.900ms against HEAD 3.682–3.922ms (parity), floats
8.938–8.992ms against 9.069–9.133ms (below HEAD, not claimed),
mixed 10.924–11.092ms against 11.502–11.747ms (below, not
claimed), strings/bools 13.312–13.414ms against 12.954–13.233ms,
three-sheet 12.741–13.085ms against 12.876–13.204ms (parity).
No speed claim beyond these reads. One rejected spelling is
named, not dropped: a first styled spelling that appended the
`"` close, the fragment, and the suffix on every cell read
floats ~5% and null ~20% over HEAD interleaved; the shipped
spelling keeps each plain body verbatim beside an outlined
styled body, and the reads above are that spelling's.

(d) Styled recipe, priced against its twin. New `phase` benches
`build_styled_mixed_200000` and `build_mixed_200000` in
`benches/fill_write.rs` (mixed kinds at 200_000 rows, every
column styled against the same data plain). Criterion on the
final binary: styled 25.576ms and 25.826ms, twin 23.017ms and
23.186ms — the styled build costs ~2.6ms (~11%) over its twin.
The byte difference is exactly the `s` spellings: styled
40427284 archive bytes against twin 35627284, a difference of
4,800,000 = 800,000 cells x 6 bytes (` s="1"`/` s="2"`/` s="5"`/
` s="6"`, all single-digit indices in this recipe), so the cost
stated is the cost of the `s` bytes themselves at ~1540–1570
MB/s on both sides. `sh check.sh` green (format, 35 unit tests
plus round-trip, multi-sheet, sealed, template, primitives, and
the 4 new styles tests, clippy `-D warnings`, docs, and the
macro compile-fail doctests for an unknown colour, two colours,
and a repeated `bold`).

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
