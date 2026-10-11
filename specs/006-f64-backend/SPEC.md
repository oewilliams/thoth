# Cycle 006 — f64 write-through (positional, identical bytes)

## Background

Float cells cost ~2.4x int cells at 200k (17.07 vs 10.18ms
fill+write, TESTING Post-005): every float cell buys one temp
`String` plus a second copy into the sheet. Row sharing already
removed the row `String`. Rulings carried in: the spelling stays
positional, byte-identical to current output; backend first,
template untouched; Delos framing (types are impositions on
bytes — writing a number moves digits once, straight into the
sheet, with no owned intermediate). Mechanism: `f64_cell`
formats through `std::fmt::Write` into `out` directly — the same
`Display` impl, so identical bytes by construction rather than
by reimplementation. NaN goes blank inside `f64_cell` and the
fused infinity refusal stays ahead of it, so the writer's domain
stays finite non-NaN, exactly as today.

## Requirements

- R1 — Write-through: `f64_cell` formats the value straight
  into `out`; `format_f64` is removed (no dead code left
  behind).
- R2 — `fmt::Result` arm: `String` never fails `fmt::Write`,
  so the `is_err` arm carries `debug_assert!(false)` and the
  one-line reason it exists. Never `unwrap`/`expect`
  (STYLE rule 7).
- R3 — Locks green unchanged: full suite passes unmodified —
  the round-trip verbatim-embed check pins float bytes, the
  `infinite` path pins refusal. No new tests: same impl means
  identical bytes by construction, for reviewer confirmation.
- R4 — Numbers recorded: `cells/tall_floats_1000`,
  `tall_floats_20000`, `tall_floats_200000` re-taken on the
  same machine/profile/commit; TESTING Post-006 entry with
  means and criterion change%; no regression vs 0.200ms,
  1.886ms, 17.07ms.
- R5 — Records: reviewer writes `REVIEW.md`, applicator writes
  `DECISION.md`; everything left uncommitted.

## Scoped files

`src/emit.rs`, `TESTING.md`, `specs/006-f64-backend/` records.
`tests/`, `benches/`, `src/xlsx.rs` untouched: same API, same
bytes.

## Acceptance

- Gate green (`cargo fmt` clean, `sh check.sh` with
  `CARGO_HOME=/tmp/cargo-home`).
- Byte-identity: round-trip, verbatim-embed, golden, and pins
  green — float bytes unchanged or the cycle does not ship.
- Floats re-taken, recorded, no regression; TESTING Post-006
  holds machine, profile, and commit.
- Reviewer verdict per item in `REVIEW.md`; applicator
  reasoning in `DECISION.md`; changes left uncommitted.

## Boundaries (not this cycle)

- No API change (`f64_cell` keeps `row: &str`).
- No scientific notation: the positional ruling stands.
- No template work, no per-cell match or bounds-check changes,
  no new dependencies (`std::fmt` only).
- Direct-binary bench runs pass `--bench` with the single
  filter; the estimates mtime must move before any number is
  quoted (protocol lesson, TESTING).

## Workflow (on `go`)

Producer, reviewer, applicator as three independent agents,
one role each, per `specs/README.md`: the producer gets this
SPEC plus `STYLE.md`, `DESIGN.md`, `TESTING.md`, and nothing
else; the reviewer reads code and spec, builds, runs probes
and tests, and creates only `REVIEW.md` (the source tree
stays byte-identical, verified by diff); the applicator
applies required changes and writes `DECISION.md`, leaving
everything uncommitted. No agent reviews its own work.
