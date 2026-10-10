# Cycle 003 review — Cell-format benches

Reviewer: child agent-2. Read-only review; no source file was edited.
The only file created by this review is this `REVIEW.md`.

## Scope record (against HEAD `279adae`)

`git status --short` at review time:

```
M TESTING.md
 M benches/fill_write.rs
 M tests/common/mod.rs
?? specs/003-cell-format-benches/
```

`git diff --stat HEAD` at review time:

```
TESTING.md            | 45 +++++++++++++++++++++++++++++++++++++++++++++
benches/fill_write.rs | 19 ++++++++++++++++++-
tests/common/mod.rs   | 32 ++++++++++++++++++++++++++++++++
3 files changed, 95 insertions(+), 1 deletion(-)
```

Untracked `specs/003-cell-format-benches/` contained only `SPEC.md`
(no `REVIEW.md`/`DECISION.md` before this review). `git diff --stat
HEAD -- src/` is empty: zero `src/` changes. Scoped files are exactly
the SPEC list (`tests/common/mod.rs`, `benches/fill_write.rs`,
`TESTING.md` plus this review file). No `STYLE.md`, `DESIGN.md`,
`check.sh`, or `tests/roundtrip.rs` modifications.

## Verdicts

### R1 — Per-type tall examples in the shared store: PASS

- `tall_ints(n)` at [tests/common/mod.rs](/Users/oliverwilliams/Documents/thoth/tests/common/mod.rs:179): single `I64` column named `"t"`, data `0..n`, expected `Option<f64>` row-major singletons — matches the SPEC recipe.
- `tall_floats(n)` at [tests/common/mod.rs](/Users/oliverwilliams/Documents/thoth/tests/common/mod.rs:194): single `F64` column named `"px"`, data `i as f64 + 0.5`, expected row-major singletons — matches the SPEC recipe.
- Wired into `all()` in stable order after `tall` at [tests/common/mod.rs](/Users/oliverwilliams/Documents/thoth/tests/common/mod.rs:250): `tall`, then `tall_ints`, then `tall_floats`. Values deterministic, exactly round-trippable (same recipe as `tall`). `tests/roundtrip.rs` unchanged in shape (still iterates `all(64)`).

### R2 — `cells` criterion group: PASS

- `cells()` at [benches/fill_write.rs](/Users/oliverwilliams/Documents/thoth/benches/fill_write.rs:102) benches `tall_ints` and `tall_floats` at `[1_000, 20_000, 200_000]` through the existing `write_iter` (fill plus write to temp, no read-back) at [benches/fill_write.rs](/Users/oliverwilliams/Documents/thoth/benches/fill_write.rs:104).
- Registered in `criterion_group!` at [benches/fill_write.rs](/Users/oliverwilliams/Documents/thoth/benches/fill_write.rs:115): `(benches, fill_write, floor, cells)`.
- No other bench changes: the diff touches only the new `cells` function plus the one registration line; `fill_write` and `floor` bodies are untouched.

### R3 — Recorded numbers: PASS (with noted limits)

- `TESTING.md` section `## Cell-format benches
  (pre-cell-optimization, provisional)` at [TESTING.md](/Users/oliverwilliams/Documents/thoth/TESTING.md:81) records per-bench wall (criterion mean), output bytes, MB/s, ns/cell for all six `cells` benches at [TESTING.md](/Users/oliverwilliams/Documents/thoth/TESTING.md:93), the same-run floor figures at [TESTING.md](/Users/oliverwilliams/Documents/thoth/TESTING.md:102), and the format-cost isolation (literal equal-n deltas plus normalized formatted-vs-blank per-cell cost) at [TESTING.md](/Users/oliverwilliams/Documents/thoth/TESTING.md:107).
- Pre-cell-optimization/provisional label present twice: heading (line 81) and body ("Cell writes are unoptimized at this point … the cell-optimization cycle re-takes them, it does not inherit them as a baseline", lines 88–91). Machine/profile/commit stated (lines 83–86: Darwin 25.6.0 arm64 Apple M4, bench profile inheriting `--release` with `overflow-checks = true`, `codegen-units = 1`, commit `279adae` plus uncommitted 003 work).
- Internal arithmetic verified by the reviewer: every MB/s (= bytes/wall), ns/cell (= wall/n), literal delta, and normalized per-cell delta recomputes from the printed figures (e.g. 200k ints 10470009 B / 82.43 ms = 127.0 MB/s, 412.1 ns/cell; normalized ints +204 ns = 412.1 − 83.23 ms/400k). Normalized math correctly accounts for the shape difference (floor writes two blank columns per row, each `cells` bench one formatted column), and the table calls out that the near-zero literal int delta is a shape artifact, not free formatting.
- Independent corroboration: reviewer ran `cargo run --locked --release --example bench` (which now iterates `all(200_000)`, so it exercises the new examples through the real writer). Observed output bytes `tall_ints 10470009`, `tall_floats 10870010` — exact match to the recorded 200k bytes column. Wall times from that smoke run are single-shot timings, not criterion means, so they are not expected to match; per STYLE rule 9 only within-run ratios are load-bearing, and the review does not treat absolute wall times as re-verified. The 1k/20k bytes figures were not independently re-probed (same writer path; 200k exact match plus round-trip green covers the recipe).

### R4 — Null behavior diff: PASS

- Zero `src/` changes (empty `git diff HEAD -- src/`).
- `sh check.sh` with `CARGO_HOME=/tmp/cargo-home` run by the reviewer: green (`cargo fmt --check`, `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo doc --locked --no-deps`, ending `all gates green`). Round-trip suite passes (10 unit + 1 round-trip integration test ok), and the round-trip test iterates `all(64)`, so the new examples round-trip at test scale. Existing expectations unmodified (diff shows pure additions in `tests/common/mod.rs`).

### R5 — String/bool gap on record: PASS

- Status line at [TESTING.md](/Users/oliverwilliams/Documents/thoth/TESTING.md:122): "String/bool body cells have no benches yet: writer, store, and store reader cover `I64`/`F64` only, so their benches wait on a feature cycle." No types added (no `src/` or store-type changes).

### Acceptance roll-up

- Gate green (`cargo fmt` clean, `sh check.sh` with `CARGO_HOME=/tmp/cargo-home`): PASS — both observed this session (`fmt clean`, `all gates green`).
- New examples round-trip; `git status` scoped files only: PASS.
- `TESTING.md` holds the `cells` table with isolation delta and string/bool line, from the recorded run: PASS (bytes independently corroborated at 200k; wall means taken as recorded per bench protocol).
- Reviewer verdict per item in `REVIEW.md`: PASS (this file). Applicator reasoning in `DECISION.md` and leaving changes uncommitted are outside the reviewer role; working tree was left uncommitted by this review.

## Findings (future work, not fixes — no source edits per cycle boundary)

1. `f64` formatting cost is the visible per-cell gap (~45–65 ns/cell above ints at scale, normalized floats +249–282 ns vs ints +192–243 ns). Any emitter-backend swap for `f64` formatting belongs to the cell-optimization cycle, which re-takes these numbers.
2. Single-block-write fusion (pending cycle 004-adjacent work) should use the normalized per-cell deltas here as its "to beat", not the literal equal-n deltas, given the 2-blank-column vs 1-formatted-column shape difference documented in the table.
3. Cross-run absolute times sit a few percent above the older floor baseline (producer-noted machine noise); later comparisons must use within-run `cells`-vs-`floor` ratios only, per STYLE rule 9.
4. String/bool body cells remain uncovered by writer, store, and reader; their benches wait on a feature cycle (R5 line). No action here.
