# Cycle 001 — Applicator DECISION

## Reasoning (recorded before acting)

Review verdict (`specs/001-floor-review/REVIEW.md`): **PASS** on all six
SPEC requirements and all four acceptance items, on the tree with
`M src/xlsx.rs` + `M src/zip.rs` only. I inspected the cited code
(`src/zip.rs` guard + `ZipWriter::finish`, `src/xlsx.rs` guard placement
+ `TooLarge`) and agree with each verdict; no requirement asks for more
than what is already on the tree.

The review's single observation (`src/zip.rs:158`,
`out.len() as u32 - central_start` re-narrows the total) was considered
as the only candidate fix and **rejected for this cycle**:

- It is outside the SPEC's enumerated cast categories (part data
  lengths, local header offsets, central directory offset and length,
  end record counts). No `u32` field holds the archive total, so the
  sum-of-sections truncation is not a listed guard category.
- It is unreachable without ~4 GB of allocation (the fits-true
  `[(u32::MAX - 30, 0)]` size case totals `u32::MAX + 68`), so no
  in-range behavior changes either way.
- The review itself marks it non-blocking and defers it to "a later
  cycle touching `finish`". Fixing it now would expand the diff beyond
  what the review requires, against the SPEC's non-goals and the
  diff-scope acceptance gate.

Decision: apply nothing from the review; leave `src/zip.rs` and
`src/xlsx.rs` exactly as reviewed. This file is the only applicator
write.

## What changed

- No source edits. `src/zip.rs` and `src/xlsx.rs` (including their
  additive `#[cfg(test)]` modules) are untouched by the applicator.
- Created this file (`specs/001-floor-review/DECISION.md`) per the
  applicator contract.

## What was rejected and why

- Rejected: reworking `ZipWriter::finish:158` to compute the end-record
  central length from tracked section lengths instead of re-narrowing
  `out.len()`. Why: out of SPEC scope per above, explicitly deferred by
  the reviewer, unreachable without gigabyte allocation, and any edit
  there risks oracle churn for zero in-range benefit.

## Verification

- `export CARGO_HOME=/tmp/cargo-home; cargo fmt; sh check.sh`:
  green (`cargo fmt --check`, 10 unit + 1 integration green,
  clippy `-D warnings`, doc, `all gates green`).
- `cargo test --release --locked`: same 10 + 1 green.
- `git status --short`: ` M src/xlsx.rs`, ` M src/zip.rs` plus
  untracked `specs/001-floor-review/REVIEW.md` (reviewer) and this
  file; no other entries. `git diff --stat`: 2 files,
  155 insertions / 11 deletions, scoped to `src/zip.rs` + `src/xlsx.rs`.
- No commit; changes left uncommitted for review.
