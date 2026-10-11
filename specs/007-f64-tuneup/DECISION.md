# Cycle 007 decision — f64 tune-up applicator

Applicator record, 2026-10-11. Tree at `1cbe856` plus the
uncommitted 007 work below. Everything left uncommitted: no
commit, push, amend, or tag.

## What changed

`src/emit.rs` only (R1 + R2; R3 ships no code change):

- R1: removed the `f64_cell` construction note (`:121-123`,
  "`Display` straight into `out` … by construction"). Kept
  `:120` (domain contract: NaN-blank vs caller-refused
  infinity), the row-sharing sentence reflowed onto the doc
  head to match `i64_cell` (`:100-102`), and the `:138` arm
  reason (forced per 006 R2). Net: five doc lines become
  three, matching the reviewer's binding keep/remove list
  exactly.
- R2: import regroup per the `src/xlsx.rs:9-13` convention —
  `use std::fmt::Write as _;` first, blank line, then
  `use crate::contract::NULL_I64;`. Itemized, no glob.

`TESTING.md`: Post-007 entry only (numbers below, dual
baselines, machine/profile/commit). No other file touched:
`git diff HEAD --stat` is `src/emit.rs` + `TESTING.md`;
`tests/`, `benches/`, `examples/`, `src/xlsx.rs`,
`src/zip.rs`, `STYLE.md`, `DESIGN.md` untouched.

## What was rejected and why

- R3 positional spelling (`write!(out, "{}", v)`): proven
  byte-identical (zero mismatches, see below) and benched on
  the real cell path — criterion 1k A/B reads inline
  187.33µs vs positional 188.82µs with overlapping CIs
  ([185.47, 190.11] vs [187.48, 190.88]µs), and scratch
  best-of-61 reads ~200µs vs ~198µs (~1.3%). Tie by the
  SPEC's own rule, so the current inline spelling stays.
  The temporary swap was reverted; the shipped `write!`
  line is byte-identical to the reviewed tree.
- Beyond-`Display` backends (`ryu`/`dtoa`): out of scope by
  SPEC boundary, owned by a named future cell cycle — not
  taken, not vague-later.
- `STYLE.md` register change: reviewer ruled none owed (R2
  applies existing rule 3) — none made.

## Reviewer disagreements and dispositions

The review (`specs/007-f64-tuneup/REVIEW.md`) rules the SPEC
SHIPPABLE with **zero rejections**. Disposition ledger:

1. R1 binding keep/remove list — applied verbatim (above).
2. R2 approval (inspection is load-bearing; fmt/clippy do
   not enforce grouping) — applied; inspection confirms the
   new order matches `xlsx.rs:9-13`.
3. R3 advisory signal (scratch parity predicts a tie; the
   real cell-path measurement decides) — measured as
   ordered; tie confirmed on both harnesses; current
   spelling kept per the tie rule. Advisory closed.
4. R4 approval — all three scales re-taken per protocol and
   recorded with both baselines (below). The reviewer's
   −6.3%-vs-−6.5% note is a confirmation, not a
   disagreement: no action beyond the dual-baseline rule,
   which the Post-007 entry follows.
5. R5 reviewer half done in `REVIEW.md`; this file is the
   applicator half. No deferrals taken on either side.
6. Findings 1–3 (scope fairness, baseline-vs-candidate
   assignment, named-future deferrals) — none blocking; the
   baseline (`push_str(&v.to_string())`) stayed a baseline
   and was not benched as a candidate, per finding 2.

No reviewer item was declined or modified.

## Measured numbers (shipped tree)

Machine Darwin 25.6.0 arm64 `RELEASE_ARM64_T8132`, bench
profile (`overflow-checks = true`, `codegen-units = 1`),
scored criterion harness, estimates mtimes confirmed moved
before quoting.

| bench | mean | vs Post-006 mean | criterion-own |
|---|---|---|---|
| `cells/tall_floats_1000` | 0.187ms ([185.47, 190.11]µs) | +0.2% vs 0.187ms | −0.62% (p = 0.45, no change) |
| `cells/tall_floats_20000` | 1.257ms ([1.2399, 1.2783]ms; corroborating run 1.2556ms) | −5.0% vs 1.324ms | −24.3% vs churned stored baseline |
| `cells/tall_floats_200000` | 12.62ms ([12.48, 12.79]ms) | +0.4% vs 12.57ms | −20.7% vs churned stored baseline |

No regression at any scale. The 20k −5.0% is
inter-session variance, not a code effect (comment/import
edits do not reach codegen); the criterion-own 20k/200k
figures compare against stored baselines churned by this
cycle's own noisy passes, so the recorded-mean column is
load-bearing. Superseded noisy passes (Post-006 precedent):
one 20k and four 200k wide-CI passes under evident machine
noise, replaced by the tight re-takes above.

R3 identity probe (`/tmp/f64probe-007/probe.rs`,
`rustc --edition 2021 -O`): 4024 values across three bare
spellings plus both `write!` spellings in the `<v>` wrapper
— zero mismatches; `"-0"` on both `write!` spellings.
NaN-blank and infinity-refusal paths pinned by the
unmodified suite (green below).

## Gate evidence

`cargo fmt --check` clean. `CARGO_HOME=/tmp/cargo-home sh
check.sh`: all gates green. `cargo test --locked`: 18 + 1 +
4 passed, zero failed. `cargo clippy --locked --all-targets
-- -D warnings` clean. Byte-identity proof on the shipped
tree: round-trip, verbatim-embed, golden
(`golden_small_archive_is_byte_identical`), and pins
(`static_parts_are_pinned`, `null_is_blank`,
`infinite_names_the_excel_row`) all green — float bytes
identical.
