# Cycle 007 review — f64 tune-up SPEC

Reviewer verification, 2026-10-11. No producer diff exists yet (prior
result 1 delivered only `SPEC.md`), so this review verifies each
requirement's premises against the actual code, re-runs the key
byte-identity and perf evidence, and rules the SPEC shippable or not.
Every cited body was opened and read. No source file was edited by the
reviewer: all nine tracked bodies hash-identical before and after
(hashes below); `git status --porcelain` shows only the untracked
`specs/007-f64-tuneup/` directory; `git diff HEAD --stat` is empty.

## Verdicts

- R1 (comment minimalism on the 006 surface): APPROVE, with a binding
  keep/remove list (§R1). Scope is complete; the removal is covered.
- R2 (import placement matches the house convention): APPROVE.
- R3 (fastest byte-identical spelling wins, identity first): APPROVE.
- R4 (numbers recorded, no regression): APPROVE.
- R5 (record hygiene): APPROVE for the reviewer half (this file);
  `DECISION.md` is applicator-owned. Nothing committed.
- The SPEC as a whole: SHIPPABLE. No rejections, one advisory signal
  under R3 (micro-bench parity — the applicator still measures the
  real cell path; a tie keeps the current spelling per the SPEC's own
  rule).

## Evidence per item

### R1 — comment minimalism

The 006 diff (`git show 1cbe856 -- src/emit.rs`, read in full)
confirms the SPEC's scope: the only 006-touched comment lines are the
`f64_cell` doc (`src/emit.rs:120-124`, three lines added plus a
reflow of the row-sharing sentence) and the `is_err` arm reason
(`src/emit.rs:137-139`, added). Line 120 itself is untouched context,
but the doc reads as one unit, so scoping `120-124` is correct, not
over-broad. The arm reason matches the 006 R2 wording exactly, and
the SPEC correctly exempts it as forced.

Per-line ruling (this list is the keep/remove the R1 test demands):

- `:120` ("One numeric f64 cell. NaN -> blank; infinite -> caller
  refuses."): KEEP. States the domain contract — the division of
  responsibility between `f64_cell` (blanks NaN) and its caller (the
  fused infinity refusal in `src/xlsx.rs:281-294`, untouched) — which
  no compiler-checked artifact states in words. Matches the
  `i64_cell` house style (`src/emit.rs:100-102`).
- `:121-123` (construction note: "`Display` straight into `out` …
  identical to the old temp-`String` spelling by construction"):
  REMOVE. The mechanism is visible on its own line (`:137`,
  `write!(out, "{v}")`), so the note restates the code; and unlike
  the `push_i64` doc (`src/emit.rs:45-46`), which cites its pinning
  suite by name (`int_spelling_matches`), this note names no pin.
  Covering check for the removal: the byte-identity property stays
  pinned by the unmodified suite — round-trip
  (`examples_round_trip_through_disk`), the verbatim-embed assertion
  (`tests/roundtrip.rs:36`), the golden test, and
  `static_parts_are_pinned` — all green in this review's gate re-run
  (18 + 1 + 4 tests, see §Gate); a comment deletion cannot alter
  bytes, and the green suite proves it.
- `:123-tail–:124` (row-sharing sentence): KEEP. States the
  cross-cell sharing contract, matching `i64_cell` (`:101-102`).
  The applicator reflows it onto the doc head after removing
  `:121-123`.
- `:138` (arm reason): KEEP, forced per 006 R2 — states the
  unreachability reason ("`String` never fails `fmt::Write`") that
  `debug_assert!(false)` alone does not carry in release.

### R2 — import placement

Premises verified by inspection. `src/emit.rs:5-6` stacks
`use crate::contract::NULL_I64;` above
`use std::fmt::Write as _;` with no grouping — the 006 diff shows
the `std` line was appended below the crate line. `src/xlsx.rs:9-13`
shows the house convention exactly as the SPEC describes: the `std`
group first (`use std::path::{Path, PathBuf};`), a blank line, then
the crate group, everything itemized, no glob — satisfying STYLE
rule 3 ("No glob imports") and the Rust chapter ("Imports
item-by-item"). One honest note, already honest in the SPEC: `cargo
fmt --check` and `clippy -D warnings` do not enforce import grouping
— inspection is the load-bearing check, and the stated test leads
with "inspection plus". Both gates are clean anyway (see §Gate), so
the regroup is behavior-only as claimed.

### R3 — fastest byte-identical spelling, identity first

Premises verified. The 006 `SPEC.md`/`REVIEW.md` confirm `write!(out,
"{v}")` was picked by construction-identity and never measured
against sibling spellings: the 006 reviewer's probe (`2015` values)
compared `write!(s, "{v}")` against `v.to_string()` — the old
allocating baseline — but the inline-args vs positional-args
comparison (`"{v}"` vs `"{}", v`) was never run. The SPEC's corpus
floor (signed zero, 1/3, 0.1, 123.456, 1e21, 1e-7, 1e300, 5e-324,
`MAX`, min-subnormal, ±1000 quarter-steps, `<v>` wrapper, suite-pinned
NaN/infinity paths) faithfully reproduces the 006 probe's breadth.

Reviewer re-run (`/tmp/f64probe/probe.rs`, `rustc --edition 2021
-O`, this machine Darwin 25.6.0 arm64 `RELEASE_ARM64_T8132`,
uncommitted tree at `1cbe856`): 2017 values (16 specials including
both zeros, min-normal, max-subnormal boundary `2.4703…e-324`, and
2001 ±1000 quarter-steps) across all three spellings
(`write!(s, "{v}")`, `write!(s, "{}", v)`, `push_str(&v.to_string())`)
bare, plus both `write!` spellings inside the `<v>…</v>` wrapper —
**zero mismatches**. Signed-zero bytes confirmed `"-0"` on both
`write!` spellings. NaN-blank and infinity-refusal paths were not
re-probed in scratch: they are pinned by the unmodified suite
(`null_is_blank`, `infinite_names_the_excel_row`), green in this
review's gate run — exactly the division the SPEC orders.

Advisory signal, not a verdict: the same probe's best-of-61 timing
head-to-head over the 2017-value corpus reads at parity — three
consecutive runs agree at ~43.7µs both spellings (within 0.5%; a
first cold run read 118µs vs 105µs, discarded as warmup noise per the
thin-margins-need-rounds lesson). This suggests the SPEC's tie rule
("a tie or within-noise margin keeps the current spelling") is the
likely outcome — but a scratch micro-bench is not the cell path, so
the applicator's criterion measurement on the real benches decides.

### R4 — numbers recorded, no regression

Premises verified against `TESTING.md:345-372`. Recorded Post-006
means 0.187ms / 1.324ms / 12.57ms (`:355-357`) and Post-005 means
0.200ms / 1.886ms / 17.07ms (`:359`) check out; the deltas −29.8% /
−26.3% recompute exactly from the recorded means, and −6.3% matches
to rounding (unrounded criterion means; the rounded pair reads
−6.5% — see note below, not a disagreement). The 200k float-vs-int
gap 12.57 vs 10.18ms (`:332`) is ~1.2x as claimed. The dual-baseline
hazard is real and stated on both sides: criterion's own calls
(−5.56% at 1k, −35.0% at 20k, `:364-365`) sit beside the recorded
Post-005 comparison, and the 20k stored baseline (~2.04ms, per the
006 `REVIEW.md` minor note, re-confirmed there at lines 89-93)
differs from the recorded 1.886ms mean — so the SPEC's both-baselines
rule resolves a genuine ambiguity. Bench protocol (`TESTING.md:60-65`:
one filter, `--bench` flag, estimates-mtime check), profile
(`Cargo.toml:9-13`: `overflow-checks = true`, `codegen-units = 1`),
and STYLE rule 9 (machine/profile/commit or gossip) all read as
cited.

Reviewer spot re-take (same tree, this machine, bench profile):
`cargo bench --bench fill_write -- cells/tall_floats_1000` in the
scored harness (criterion analyzing, not test-only mode) reads
**188.12µs, CI [186.43, 190.38]µs**, criterion reporting "No change
in performance detected" (p = 0.60); estimates mtime moved
(1791733196 → 1791734012) before quoting, per protocol. Consistent
with the recorded 187µs within noise — no drift, no regression at
the re-taken scale.

Note (confirmation, not a disagreement): the rounded 1k means give
−6.5% while the entry states −6.3%; the entry's figure comes from
unrounded criterion means and both the criterion-own (−5.56%) and
recorded-mean comparisons are printed side by side, so there is no
misrepresentation for R4's dual-baseline rule to fix — it is the
case that motivated the rule.

### R5 — record hygiene

Reviewer half complete with this file: one verdict per R1–R5, each
tied to an inspected file and line above, plus gate evidence below.
Applicator half (`DECISION.md`: what changed, what was rejected and
why, measured numbers) is applicator-owned by role and correctly
out of reviewer scope. No deferrals are taken in this review. On the
SPEC's register clause ("no `STYLE.md` register change unless the
reviewer rules one"): ruled — none needed. R2 applies existing
STYLE rule 3 and the Rust-chapter itemized-import convention; no new
convention is created, so no register entry is owed.

## Gate evidence

`cargo fmt --check`: clean (exit 0). `CARGO_HOME=/tmp/cargo-home sh
check.sh`: "all gates green" — lib unit tests 18 passed
(`src/emit.rs:3`, `src/xlsx.rs:12`, `src/zip.rs:3`), integration
`tests/roundtrip.rs:1` (`examples_round_trip_through_disk`, incl.
the verbatim-embed assertion at `:36`), `tests/sealed.rs:4`, zero
failures; `clippy --locked --all-targets -- -D warnings` clean;
`cargo doc --locked --no-deps` clean. The suite is unmodified
(hash proof below), so the round-trip, verbatim-embed, golden, and
pin tests are the byte-identity proof for the current tree, as R5
requires.

## Zero-edit hash proof

`git hash-object` after all probes and runs, identical to the
pre-review baseline at commit `1cbe856`:

- `src/emit.rs` `b939cf3ee0bef5284b91809f3ca9854039d30191`
- `src/xlsx.rs` `bde3abd077e1a03e3956a587cd6f8c655b78a4d5`
- `src/zip.rs` `6914af825864e14dd9daf6973a282e839156e219`
- `TESTING.md` `bdd782f7b464e8552e4ca4becedc0dd816403c37`
- `STYLE.md` `df5d3676fa3ff5f65b9ba4e5f94489af93b07d00`
- `DESIGN.md` `5b3d562521bbac28c353b18a2f14ca3cf064519c`
- `Cargo.toml` `08e5540084d9fe529e2ff7062c5d794ecad98c67`
- `check.sh` `ce400d11eb8dfd2c08f6b84e14c345d89aec1d0b`
- `benches/fill_write.rs`
  `b6e9c66a8e8798e5c230d2fd3606e594c4655d23`

`git status --porcelain`: only `?? specs/007-f64-tuneup/`.
`git diff HEAD --stat`: empty. The only filesystem writes outside
the new spec directory are `/tmp` scratch (`/tmp/f64probe/`,
criterion output under `target/`, which is git-ignored) — no new
in-tree tests or benches, per the SPEC's scoping.

## Findings (none blocking)

1. The SPEC's Background states the 006 deliverable "has not had the
   foundation pass that `src/zip.rs` (cycle 004) and the `xlsx.rs`
   tail (cycle 005) went through." Verified as fair: the 006 diff
   touches only `src/emit.rs` plus the `TESTING.md` entry, and the
   stacked-import / construction-note residue is visible at
   `src/emit.rs:5-6,121-123`.
2. The SPEC confines R3 candidates to same-`Display`-impl spellings
   with the old `push_str(&v.to_string())` as baseline, not
   candidate — correct, since the baseline allocates and cannot win
   on the mechanism the cycle exists to remove.
3. Beyond-`Display` backends (`ryu`/`dtoa`) and ingress/signature
   shape are assigned to named future passes (a cell cycle, the
   template cycle), not vague later — satisfying R5's deferral rule
   by construction. Nothing further to defer from review.
