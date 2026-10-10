# Cycle 004 — Zip internals to machine tightness

## Background

Review verdict (user, 2026-10-10): `src/zip.rs` is decent Rust, not
good enough for this project. `crc32` runs a bitwise loop with a
fixed 8-iteration inner loop over borrowed bytes — prime vectorizer
miss on an accumulator over every output byte. `stored_archive_len`
copies tuples through a multi-line loop for single-expression
arithmetic. `stored_archive_fits` keeps constants in function scope
and validates at a level that is not an ingress interface. `ZipWriter`
asserts, grows its entry list, and calls small helpers with visible
call overhead. This cycle rewrites the zip internals for
instruction-count tightness, with assembler and bench evidence for
every claim.

Binding references: `DESIGN.md`, `TESTING.md`, `STYLE.md` (machine-level
minimalism; measure, don't assume — including this spec's own
suspicions, which the assembler confirms or refutes).

## Requirements

- **R1 — `crc32` tight and vectorized.** Replace the bitwise loop.
  Candidates (table-driven, target hardware CRC) are measured
  head-to-head on a micro-bench and the `floor` group; the winner is
  picked by numbers, not by assumption. Any target-specific path is
  compile-time `cfg` only — no runtime feature detection, per the
  monomorphization-only branch rule. The IEEE test vector
  (`"123456789"` → `0xCBF4_3926`) stays. Assembler cited in review.
- **R2 — `stored_archive_len` as single-expression arithmetic.**
  No tuple copies, no statement-per-step loop. Assembler evidence of
  the instruction drop versus the current spelling.
- **R3 — Validation moves to the ingress boundary.** The zip module
  performs no validation: name shape, uniqueness, count, and every
  size check move to the ingress-facing build path (`build_xlsx`),
  which already knows the entry set. The reviewer matches every
  narrowing cast in `finish` to its covering ingress check,
  cast for cast. Incidental: hoist the constants to module scope.
- **R4 — `ZipWriter` assumes valid input.** No asserts on the build
  path (invalid states unrepresentable by construction, or
  debug-only). Entry count known at construction: sized once, never
  grown. No residual call overhead on the byte pushes — assembler
  evidence, however achieved.
- **R5 — Byte-identity lock.** Before rewriting, capture a golden
  test from pre-change output (exact bytes of a small fixed archive).
  It passes unchanged after. Round-trip suite green: same files out.
- **R6 — Numbers recorded.** Re-run `floor` (and any new `crc`
  micro-bench, additive in `benches/fill_write.rs`) `--release` on
  this machine; record deltas in `TESTING.md` from the actual runs.

## Scoped files

`src/zip.rs`, `src/xlsx.rs`, `benches/fill_write.rs` (additive
only), `TESTING.md` (plus `specs/004-zip-tighten/REVIEW.md`,
`specs/004-zip-tighten/DECISION.md` by role).

## Acceptance

- Gate green (`cargo fmt` clean, `sh check.sh` with
  `CARGO_HOME=/tmp/cargo-home`).
- Golden byte-identity test plus full round-trip green.
- `TESTING.md` holds the `floor`/`crc` deltas and the assembler
  notes, all from recorded runs.
- Reviewer verdict per item in `REVIEW.md` with cast-to-check
  mapping and assembler citations; applicator reasoning in
  `DECISION.md`; changes left uncommitted.

## Boundaries (not this cycle)

- No ingress API change (`ColumnData`, `Column`, error types keep
  their shape; validation moves, callers don't).
- No deflate, no cell/emitter changes, no run-off, no `writev`.
- No runtime dispatch of any kind; the assembler is evidence, and
  where it refutes a suspicion in this spec, the assembler wins.
