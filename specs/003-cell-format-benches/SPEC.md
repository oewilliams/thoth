# Cycle 003 — Cell-format benches

## Background

The floor (`floor` group: headers, blanks) is measured and recorded;
the formatted-cell paths are not. `fill_write` benches the shared
`tall` example at one scale (20k), which mixes one `I64` and one `F64`
column, so no per-type format cost can be isolated and no linearity
check exists for formatted cells. The pending fusion work (single
block write) and any emitter-backend swap (`f64` formatting) need
per-type, multi-scale deltas against the floor. This cycle measures
only; it changes no emitter.

Binding references: `DESIGN.md`, `TESTING.md`, `STYLE.md`.

## Requirements

- **R1 — Per-type tall examples in the shared store.** Add
  `tall_ints(n)` (`I64` column `0..n`, name `"t"`) and
  `tall_floats(n)` (`F64` column `i as f64 + 0.5`, name `"px"`) to
  `tests/common/mod.rs`, with round-trip `expected` in the existing
  `Option<f64>` shape, wired into `all()` in stable order after
  `tall`. Values stay deterministic and exactly round-trippable
  (same recipe as `tall`). Tests keep small `n`; nothing in
  `tests/roundtrip.rs` changes shape.
- **R2 — `cells` criterion group.** In `benches/fill_write.rs`, add a
  `cells` group benching `tall_ints` and `tall_floats` at
  `[1_000, 20_000, 200_000]` through the existing `write_iter`
  (fill plus write to temp, no read-back). Register the group in
  `criterion_group!`. No other bench changes.
- **R3 — Recorded numbers.** Run the `cells` group `--release` on
  this machine and record in `TESTING.md` beside the floor baseline:
  per-bench wall time, output bytes, MB/s, per-cell ns, plus the
  format-cost isolation (`cells` minus `floor` `tall_null` at equal
  scale, per cell and per byte). Numbers from the actual run only. Label the table pre-cell-optimization:
cell writes are unoptimized at this point, so these numbers verify
this layer (linearity, per-type isolation against the floor) and are
provisional — the cell-optimization cycle re-takes them, it does not
inherit them as a baseline.
- **R4 — Null behavior diff.** No `src/` changes. `sh check.sh`
  green; round-trip suite passes including the new examples at test
  scale; existing expectations unmodified.
- **R5 — String/bool gap on record.** Writer, store, and store
  reader all lack string/bool body cells (store covers `I64`/`F64`
  only). Add one status line in `TESTING.md` stating their benches
  wait on a feature cycle. No types added here.

## Scoped files

`tests/common/mod.rs`, `benches/fill_write.rs`, `TESTING.md`
(plus `specs/003-cell-format-benches/REVIEW.md`,
`specs/003-cell-format-benches/DECISION.md` by role).

## Acceptance

- Gate green (`cargo fmt` clean, `sh check.sh` with
  `CARGO_HOME=/tmp/cargo-home`).
- New examples round-trip; `git status` shows scoped files only.
- `TESTING.md` holds the `cells` table with the isolation delta and
  the string/bool status line, all from the recorded run.
- Reviewer verdict per item in `REVIEW.md`; applicator reasoning in
  `DECISION.md`; changes left uncommitted.

## Boundaries (not this cycle)

- No emitter, zip, or writer changes, however hot the profile looks;
  hotspots go in the review as future-work notes.
- No run-off, no `writev`, no third-party reader: cycle 004.
