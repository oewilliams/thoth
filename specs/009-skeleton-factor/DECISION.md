# Cycle 009 applicator decision — skeleton factor

Applicator record, 2026-10-11. Applied exactly the reviewer-approved
SPEC to `src/emit.rs`, plus the `TESTING.md` Post-009 entry the SPEC
requires. Everything left uncommitted.

## Reviewer verdicts and dispositions

| # | Verdict | Disposition |
|---|---|---|
| R1 single-file, no per-kind split | APPROVE | Applied: helpers are private items in `src/emit.rs` beside the cells; no new module, no new `.rs` file, no `pub` widening; `i64_cell` / `f64_cell` keep `(out, col_letters, row, v)`. `git status` shows no new `.rs`. |
| R2 monomorphic loops, no per-cell dispatch | APPROVE + advisory (distinct entry points, no mode flag) | Applied per advisory: open / blank-close / value-wrap as distinct `#[inline]` fns, no flag parameter. |
| R3 byte-identity, suite green unmodified | APPROVE | Gate green on the factored tree; no test edited; bench-scale bytes re-probed identical to the 003 recipes. |
| R4 bench proof, no regression | APPROVE | All eleven benches re-taken and recorded in `TESTING.md` Post-009 with machine, profile, and commit; no regression at any scale. |
| R5 comment minimalism | APPROVE | One-line contract on the helper group; every pre-existing comment untouched. `cargo fmt --check` clean. |
| Whole SPEC | SHIPPABLE, no rejections | Shipped as approved; no deviations. |
| Per-kind file split | DEFERRED to the i32 cycle (user ruling, carried by SPEC + REVIEW) | No action: no module created, single-file layout kept. |

## R2 advisory — why four functions for three entry points

The SPEC asks for three entry points (open / blank-close /
value-open…value-close) and the reviewer requires distinct tiny
entry points with no mode flag. The value wrap needs two literal
spellings (`"><v>"` open, `"</v></c>"` close) that never occur
together in one push, so the minimal flag-free shape is four
one-line bodies in three entry groups: `cell_open`,
`blank_close`, `value_open`, `value_close`. This is not a
deviation: it is the advisory's preferred shape, and it adds no
branch anywhere.

Branch inventory (R2 test): each helper body is straight-line
`push_str` calls — zero branches. Both call sites keep exactly
their pre-existing null-test branches (`v == NULL_I64`,
`v.is_nan`) with unchanged arms; value spellings (`push_i64`,
`write!(out, "{v}")`) stay inline per kind; the infinity refusal
stays in `sheet_xml` ahead of `f64_cell`, untouched. No helper
takes a value, `Kind`, or `ColumnData`, so nothing it receives
can discriminate on type. `cargo clippy --locked --all-targets
-- -D warnings` clean (in gate).

## R4 notes (detail in `TESTING.md` Post-009)

Machine for every number below: Darwin 25.6.0 arm64, Apple M4,
bench profile (`--release`, `overflow-checks = true`,
`codegen-units = 1`), commit `77d2699` plus the uncommitted 009
work. Direct-binary runs with `--bench` and a single filter;
estimates mtimes confirmed moved before quoting.

Deltas vs recorded means (ints Post-005, floats Post-007):
ints +0.4% / +1.3% / −19.0%, floats −1.2% / −1.0% / −0.5%;
floor within −12.7%..+3.5% of recorded. The 200k-int −19.0%
is machine spread (same-binary 8.25–10.64ms across the
spell), claimed as within-noise only. A noisy spell mid-cycle
pushed tall reads up to +26% with 11–21% severe outliers;
a same-spell control (unmodified HEAD in a scratch worktree:
`tall_floats_20000` 1.2727ms tight vs recorded 1.257ms)
proved the spell environmental, and quiet-spell re-takes
(1.2447ms and 12.553ms, tight) closed the cycle.

## Scope proof

`git diff --stat`: `src/emit.rs` (helper + two call sites),
`TESTING.md` (Post-009 entry), `specs/009-skeleton-factor/`
(this record). `tests/`, `benches/`, `examples/`,
`src/xlsx.rs`, `src/contract.rs`, `src/zip.rs`, `STYLE.md`,
`DESIGN.md` untouched. Probes in `/tmp` only (`/tmp/bytes009`,
`/tmp/thoth-009-base` worktree); no new in-tree tests or
benches.

## Gate

`cargo fmt --check` clean;
`CARGO_HOME=/tmp/cargo-home sh check.sh` → "all gates green"
(lib units incl. `null_is_blank` / `int_spelling_matches` /
`letters_roll_over_z`, round-trip incl. verbatim-embed,
sealed locks, clippy, doc).
