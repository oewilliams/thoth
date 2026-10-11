# Cycle 006 review — f64 write-through

Reviewer verification, 2026-10-11. Producer diff inspected, gate
re-run, and probes run by the reviewer on this machine against the
uncommitted 006 tree (commit `1a3c0e5` + producer work). No source
file was edited by the reviewer; `src/`, `tests/`, `benches/`,
`examples/` are byte-identical before and after this review except
for the producer's own `src/emit.rs` change (verified with
`git diff --name-only`: only `TESTING.md` and `src/emit.rs` modified,
plus untracked `specs/006-f64-backend/`).

## Verdicts

- R1 (write-through, `format_f64` removed): PASS.
- R2 (`fmt::Result` arm): PASS.
- R3 (locks green, suite unmodified): PASS.
- R4 (numbers recorded, no regression): PASS.
- R5 (records, uncommitted): PASS for the reviewer half
  (this file); `DECISION.md` is applicator-owned. Nothing committed.

## Evidence per item

### R1 — write-through

- `src/emit.rs:137` formats with `write!(out, "{v}")` straight into
  `out`; the old `out.push_str(&format_f64(v))` temp-`String` spell
  is gone and `format_f64` itself is deleted (was `src/emit.rs:35-40`
  pre-change). `grep -rn format_f64 src/ tests/ benches/` returns zero
  hits: no dead code left behind.
- Same `Display` impl either way (`f64::to_string` is `Display`), so
  identical bytes by construction, not by reimplementation.
- Reviewer probe (`/tmp` scratch, `rustc -O`, not in the tree):
  `write!(s, "{v}")` vs `v.to_string()` compared over 2015 values —
  0.0, -0.0, 1/3, 0.1, 123.456, 1e21, 1e-7, 1e300, 5e-324,
  `f64::MAX`, min-subnormal, and ±1000 quarter-steps — zero
  mismatches, including inside the `<v>…</v>` cell wrapper.
- `src/xlsx.rs:281-294` untouched: NaN still goes blank inside
  `f64_cell` (`src/emit.rs:127`), the fused infinity refusal still
  sits ahead of it, so the writer domain stays finite non-NaN.
- Boundary kept: `f64_cell` keeps its `row: &str` signature
  (`src/emit.rs:126`); no new dependency (`use std::fmt::Write as _;`
  at `src/emit.rs:6`, itemized, no glob — STYLE rule 3 clean).

### R2 — `fmt::Result` arm

- `src/emit.rs:137-140`: the `is_err` arm carries
  `debug_assert!(false)` plus the one-line reason ("`String` never
  fails `fmt::Write`: unreachable by construction"). Matches the SPEC
  wording exactly.
- `grep -rn "unwrap\|expect" src/` shows no bare `unwrap`/`expect`
  in non-test code (only `unwrap_or` fallbacks and `unwrap_err` /
  `unwrap` inside `#[cfg(test)]` modules and pre-existing
  `xlsx.rs` test code): STYLE rules 7/8 hold. No new branches on the
  hot path beyond the unreachable arm.

### R3 — locks green, byte-identity

- `cargo fmt --check`: clean (exit 0).
- `CARGO_HOME=/tmp/cargo-home sh check.sh` re-run by the reviewer:
  all gates green — 18 + 1 + 4 tests pass, `clippy -D warnings`
  clean, `cargo doc` clean. That suite includes the round-trip,
  verbatim-embed, golden, and pin tests, so float bytes are pinned
  unchanged.
- `git diff --stat -- src tests benches examples` shows only
  `src/emit.rs` (9 insertions, 11 deletions); `tests/`, `benches/`,
  `examples/`, and `src/xlsx.rs` are untouched: same API, same
  bytes, no new tests, as scoped.

### R4 — numbers recorded

- `TESTING.md:345-372` Post-006 entry holds machine (Darwin 25.6.0
  arm64, `RELEASE_ARM64_T8132`), profile (bench), commit
  (`1a3c0e5` plus uncommitted work), criterion means, criterion's
  own change% (−5.56% at 1k, −35.0% at 20k, both p < 0.05), output
  bytes (unchanged 003 recipes), and the Post-005 comparison
  (−6.3% / −29.8% / −26.3% vs 0.200 / 1.886 / 17.07ms): improvement
  at every scale, no regression. The 200k float-vs-int gap narrows
  to ~1.2x (12.57 vs 10.18ms), consistent with removing the last
  per-cell allocation on the float path.
- Reviewer spot re-take (same tree, this machine):
  `cargo bench --bench fill_write -- cells/tall_floats_1000` reads
  185.6µs mean (CI [184.5, 187.1]µs) vs the recorded 187µs —
  consistent within noise; no regression.
- Method notes, both handled correctly in the entry: the discarded
  noisy 200k pass (19.95ms, CI [15.5, 25.6]ms, p = 0.34) is recorded
  alongside the tight superseding re-take (12.57ms, CI [12.44,
  12.74]ms), per the thin-margins-need-rounds lesson; and the
  criterion-0.5 substring-filter caveat (a `tall_floats_20000`
  filter also runs `tall_floats_200000`) is disclosed. Minor note:
  the 20k criterion change (−35.0%) is measured against its stored
  ~2.04ms baseline rather than the recorded 1.886ms Post-005 mean —
  both figures are stated in the entry, so no misrepresentation,
  but a future baseline reset would remove the ambiguity.

### R5 — records

- This `REVIEW.md` is the reviewer's record; `DECISION.md` remains
  applicator-owned. `git status --short` shows only `M TESTING.md`,
  `M src/emit.rs`, and untracked `specs/006-f64-backend/`: everything
  left uncommitted, reviewer included.

## Findings (none blocking)

1. The `debug_assert!(false)` arm is a debug-only tripwire: silent
   in release. That is exactly what the SPEC orders, and the arm is
   unreachable by construction, so this is confirmation, not a gap.
2. The removed `format_f64` doc comment carried the "shortest
   round-trip text" contract note; the new `f64_cell` doc comment
   (`src/emit.rs:120-124`) re-anchors the contract as "identical …
   by construction". The round-trip suite still pins the property
   itself. No doc loss.
3. Bench-protocol hygiene held: the reviewer confirmed the 1k
   re-take ran the scored harness (criterion analyzing, not
   test-only `Testing/Success` mode).
