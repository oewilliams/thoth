# Cycle 009 — skeleton factor (one private tag helper)

## Background

`i64_cell` (`src/emit.rs:105-119`) and `f64_cell`
(`src/emit.rs:125-142`) share the same cell-tag skeleton
spelling: open (`<c r="` + letters + row), blank close
(`"/>`), value close (`"><v>` … `</v></c>`). Each function
repeats it in both arms (null-blank arm and value arm), so the
skeleton is spelled four times for two kinds. The value
spellings differ per kind (`push_i64` vs `write!(out, "{v}")`)
and the null arms differ per kind (`v == NULL_I64` vs
`v.is_nan()`); only the tag skeleton is common.

The next cell kind must review as spelling plus null-arm plus
tests only — no fourth and fifth copy of the tags. This cycle
factors the skeleton into one tiny private helper with three
entry points (open / blank-close / value-open…value-close, or
the equivalent minimal shape the reviewer accepts), called
from both existing cells. Mechanism only; bytes and API
unchanged.

Rulings carried in: single-file layout stays — no per-kind
module split (user ruling, deferred until the i32 cycle);
monomorphic-loop-per-kind stays (`DESIGN.md` Templates,
`src/contract.rs:9-12` `Kind`, `src/xlsx.rs:279-295`
per-column dispatch into `i64_cell` / `f64_cell` — the hot
loop never branches on type per cell); positional `f64`
spelling stays (cycle 006/007); `#[inline]` hints stay on the
hot path (cycle 005: forced `inline(always)` regressed 28%
and was reverted — hints only, bench decides).

Binding references: `STYLE.md`, `DESIGN.md`, `TESTING.md`
(proof before fix; measure, don't assume — including this
spec's own suspicion that the factoring is free, which the
benches confirm or refute).

## Requirements (each item testable)

- **R1 — Single-file layout stays, no per-kind split.** The
  helper lives in `src/emit.rs` as a private item beside
  `i64_cell` / `f64_cell`. No new module, no per-kind file,
  no `pub` widening of the helper. The per-kind split stays
  deferred to the i32 cycle per the user ruling. Test:
  inspection — `git status` shows no new `.rs` file, the
  helper is non-`pub`, `i64_cell` and `f64_cell` keep their
  exact public signatures.
- **R2 — Monomorphic loops preserved, no per-cell dispatch.**
  The helper takes only tag parts (`out`, `col_letters`,
  `row`) — never a value, never a `Kind`, never a
  `ColumnData`. No trait object, no `dyn`, no per-cell match
  on kind, no branch on type inside the helper or reintroduced
  at either call site. Per-kind value formatting
  (`push_i64`, `write!(out, "{v}")`) and per-kind null arms
  (`NULL_I64`, `is_nan`) stay inline in `i64_cell` /
  `f64_cell`; the infinity refusal stays in `sheet_xml`
  ahead of `f64_cell`, untouched. Test: reviewer lists every
  branch in the helper and both call sites with why none
  dispatches on type; `cargo clippy --locked --all-targets
  -- -D warnings` clean.
- **R3 — Byte-identity, suite green unmodified.** Full gate
  suite passes with no test edited: `tests/roundtrip.rs`
  `examples_round_trip_through_disk` (round-trip values plus
  the per-example stored-zip-verbatim-embed check, the floor
  oracle), `src/zip.rs`
  `golden_small_archive_is_byte_identical`, `src/xlsx.rs`
  `static_parts_are_pinned` and `sheet_cap_holds`, `src/emit.rs`
  `null_is_blank` / `int_spelling_matches` /
  `letters_roll_over_z`, and the `tests/sealed.rs` locks.
  No new in-tree tests: same skeleton spelling means
  identical bytes by construction, for reviewer confirmation.
  Any byte delta fails the cycle. Test: `sh check.sh` with
  `CARGO_HOME=/tmp/cargo-home` green, tests unmodified
  (`git diff --stat tests/ src/xlsx.rs src/zip.rs` empty
  outside `src/emit.rs`).
- **R4 — Bench proof the factoring is free, no regression.**
  Re-take the full `cells` group at all three scales for both
  kinds (`cells/tall_ints_1000`, `tall_ints_20000`,
  `tall_ints_200000`, `cells/tall_floats_1000`,
  `tall_floats_20000`, `tall_floats_200000`) plus the `floor`
  group (`floor/header_only`, `floor/wide_null`,
  `floor/tall_null_1000`, `floor/tall_null_20000`,
  `floor/tall_null_200000`) per the `TESTING.md` bench
  protocol (bench profile inheriting `--release` with
  `overflow-checks = true`, `codegen-units = 1`; criterion
  scored harness, not test-only mode; estimates mtime
  confirmed moved; direct-binary runs pass `--bench` with
  the single filter). The helper carries `#[inline]` like the
  cells it serves; no `inline(always)`, no new hint without a
  measured deficit it cures. The Post-009 entry holds
  machine, profile, and commit, criterion means with
  criterion's own change%, output bytes (unchanged 003
  recipes), and both baselines for every change% (criterion's
  stored baseline and the recorded means: floats 0.187ms,
  1.257ms, 12.62ms Post-007; ints 0.164ms, 0.870ms, 10.18ms
  Post-005, the last int re-take). No regression at any of
  the eleven benches or the cycle does not ship. Every perf
  claim states machine, profile, and commit or it is not a
  measurement (STYLE rule 9). Test: Post-009 entry present
  with all eleven means, both baselines, and bytes; every
  bench within noise of or faster than its recorded mean.
- **R5 — Comment minimalism per STYLE.** Only forced comments
  survive on touched lines: each surviving line must state
  something no compiler-checked artifact states (domain
  contract, unreachability reason, pinning suite). The helper
  needs at most its one-line contract (what it spells, what
  stays per-kind); construction narration is deleted. The
  existing forced lines stay (`f64_cell` `is_err` arm reason,
  `push_magnitude` UTF-8 construction note where touched).
  Test: reviewer lists every touched comment line with
  keep/remove and the check covering each removal; `cargo fmt
  --check` clean.

## Scoped files

`specs/009-skeleton-factor/SPEC.md` (this file) by the
producer now; on `go`: `src/emit.rs` (the helper plus the two
call sites only — value spellings, null arms, `col_letters`,
`escape_into`, `header_cell`, `push_i64`/`push_usize`/`DEC_PAIRS`
untouched), `TESTING.md` (Post-009 entry only),
`specs/009-skeleton-factor/` records by role. Untouched:
`src/xlsx.rs`, `src/contract.rs`, `src/zip.rs`,
`src/static_parts.rs`, `tests/`, `benches/`, `examples/`,
`STYLE.md`, `DESIGN.md` — same API, same bytes. Probes live
in `/tmp` scratch; no new in-tree tests or benches.

## Acceptance

- Gate green (`cargo fmt --check` clean, `sh check.sh` with
  `CARGO_HOME=/tmp/cargo-home`).
- Byte-identity: round-trip, verbatim-embed, golden, and pins
  green unmodified — one changed byte fails the cycle.
- All eleven benches re-taken and recorded per R4 with
  machine, profile, and commit; no regression at any scale.
- Reviewer verdict per item in `REVIEW.md`; applicator
  reasoning in `DECISION.md`; everything left uncommitted.

## Boundaries (not this cycle)

- No API change (`i64_cell` / `f64_cell` keep
  `(out, col_letters, row, v)`).
- No value-spelling change (positional `f64`, `push_i64`
  integer writer, null sentinels `NULL_I64` / NaN, infinity
  refusal in `sheet_xml`).
- No per-kind module split (deferred to the i32 cycle by user
  ruling); no trait objects, no per-cell kind dispatch, no
  `inline(always)`, no new dependencies.
- No `STYLE.md` register change unless the reviewer rules one.
- Direct-binary bench runs pass `--bench` with the single
  filter; the estimates mtime must move before any number is
  quoted (protocol lesson, TESTING).

## Workflow (on `go`)

Producer, reviewer, applicator as three independent agents,
one role each, per `specs/README.md`: the producer gets this
SPEC plus `STYLE.md`, `DESIGN.md`, `TESTING.md`, and nothing
else; the reviewer reads code and spec, builds, runs probes
and tests, and creates only `REVIEW.md` (the source tree stays
byte-identical, verified by diff); the applicator applies
required changes and writes `DECISION.md`, leaving everything
uncommitted. No agent reviews its own work.
