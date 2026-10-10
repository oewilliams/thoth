# DECISION — Cycle 004 applicator (2026-10-10)

## Reasoning (recorded before acting)

Read `SPEC.md` and `specs/004-zip-tighten/REVIEW.md` in full, then
spot-checked the tree against the review's load-bearing claims:

- `src/zip.rs` build path carries only `debug_assert!`s (lines 193–201);
  no `assert!`/`panic!`/`unwrap()`/`expect()` on the build path.
- `stored_archive_fits` is gone from `src/zip.rs`; `archive_fits` lives
  in `src/xlsx.rs:186`, called from `build_xlsx` at `:309`.
- `TESTING.md` Post-004 section and the additive `crc32_4k` bench group
  exist as reviewed.
- `git status --short` / `git diff --stat HEAD` match the review
  baseline exactly (4 files, 418 insertions, 118 deletions; HEAD
  `bb86154` presumed — verified stat, not rehashed).

Review verdict is PASS on every requirement (R1–R6), cast-to-check
mapping COMPLETE, assembler citations PASS, gate PASS (reviewer-ran).
Findings list no defects; the two evidence limits (golden-byte
provenance producer-attested; bench deltas consistency-checked, not
re-run) and the `u32::MAX`-total observation are explicitly
non-blocking and require no code action.

I agree with all three non-blocking notes as stated and see no reason
to dispute any verdict: re-running the criterion benches would cost
more than it could change (percentages recompute exactly; `crc` group
compiles under `clippy --all-targets`), and adding a `u32::MAX` grand-
total bound would reintroduce exactly the over-validation R3 removed.

## What changed

Nothing. The review requires zero source edits, so no file in the
scoped set (`src/zip.rs`, `src/xlsx.rs`, `benches/fill_write.rs`,
`TESTING.md`) was touched by the applicator. This `DECISION.md` is the
applicator's only write.

## What was rejected and why

No review item was rejected. There was nothing to reject: with no
defects found and every acceptance item PASS, the candidate action set
(apply fixes) was empty. The only conceivable discretionary actions —
re-running benches to harden R6 evidence, or bounding the grand total
at `u32::MAX` — were considered and declined for the reasons above
(cost without decision value; contradicts R3's sound permissiveness).
