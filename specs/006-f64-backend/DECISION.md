# Cycle 006 decision — f64 write-through

Applicator record, 2026-10-11. Required reading done first: `SPEC.md`,
`STYLE.md`, `DESIGN.md`, `TESTING.md` (binding), the producer report
(prior result 1), and the review (prior result 2,
`specs/006-f64-backend/REVIEW.md`). Inspected the live tree before
deciding: `git diff -- src/emit.rs`, `grep -rn format_f64 src/ tests/
benches/` (zero hits), and the `unwrap`/`expect` hits in `src/`.

## Verdict on the review

Agree with every review verdict: R1 PASS, R2 PASS, R3 PASS, R4 PASS,
R5 reviewer-half PASS. No disagreement to record.

## What changed

No source change. The review lists zero blocking findings, and the
applicator's own inspection confirms there is nothing to apply:

- R1: `f64_cell` (`src/emit.rs:126-143`) already formats via
  `write!(out, "{v}")` straight into `out`; `format_f64` is already
  deleted with zero remaining references in `src/`, `tests/`, or
  `benches/`. Same `Display` impl, identical bytes by construction.
- R2: the `is_err` arm (`src/emit.rs:137-140`) already carries
  `debug_assert!(false)` plus the one-line reason ("`String` never
  fails `fmt::Write`: unreachable by construction"). No `unwrap` /
  `expect` in non-test code: the only `src/` hits are `unwrap_or`
  fallbacks (`xlsx.rs:332,364`, `zip.rs:220`, which never panic) and
  `unwrap_err` / `unwrap` inside `#[cfg(test)]` modules — STYLE
  rules 7/8 hold.
- R3: scoped files untouched as required (`tests/`, `benches/`,
  `examples/`, `src/xlsx.rs` — only `src/emit.rs` and `TESTING.md`
  modified).

The only file this cycle adds is this `DECISION.md`, completing the
R5 applicator half.

## What was rejected and why

Nothing was rejected. The review's three findings are explicitly
non-blocking confirmations, and each needs no action:

1. `debug_assert!(false)` is silent in release — exactly what the
   SPEC orders for an arm unreachable by construction. Kept as is.
2. The removed `format_f64` doc comment's "shortest round-trip text"
   note is re-anchored on `f64_cell` as "identical … by
   construction", and the round-trip suite still pins the property.
   No doc loss, no action.
3. Bench-protocol hygiene held (scored harness, estimates mtime).
   No action.

One reviewer minor note (20k criterion change −35.0% measured against
its stored ~2.04ms baseline rather than the recorded 1.886ms Post-005
mean): both figures are stated in the Post-006 entry, so no
misrepresentation; a future baseline reset can remove the ambiguity,
but that is outside this cycle's scope — no action.

## Measured numbers

Post-006 entry (`TESTING.md`) re-states producer figures, confirmed
by the applicator's gate re-run rather than re-taking benches (method:
benches already taken once this cycle on this machine; re-running the
full tall-bench set adds noise, not evidence):

- `cells/tall_floats_1000`: 0.187ms (criterion −5.56%, p < 0.05)
- `cells/tall_floats_20000`: 1.324ms (criterion −35.0%, p < 0.05)
- `cells/tall_floats_200000`: 12.57ms (CI [12.44, 12.74]ms)
- Vs Post-005 means (0.200 / 1.886 / 17.07ms): −6.3% / −29.8% /
  −26.3% — improvement at every scale, no regression; 200k
  float-vs-int gap ~1.2x (12.57 vs 10.18ms).

Gate evidence (this applicator run, commit `1a3c0e5` + uncommitted
006 work): `cargo fmt --check` clean; `CARGO_HOME=/tmp/cargo-home sh
check.sh` → 18 + 1 + 4 tests pass, "all gates green" (clippy
`-D warnings`, doc). Byte-identity pinned by the unmodified green
suite (round-trip, verbatim-embed, golden, pins).

## Status

Everything left uncommitted. `git status --short`: `M TESTING.md`,
`M src/emit.rs`, untracked `specs/006-f64-backend/` (now holding
`SPEC.md`, `REVIEW.md`, and this `DECISION.md`).
