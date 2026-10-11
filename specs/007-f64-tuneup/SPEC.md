# Cycle 007 — f64 tune-up to foundation standard

## Background

Committed cycle 006 (`1cbe856`) put the `f64` backend
write-through: `f64_cell` (`src/emit.rs:126-143`) formats via
`write!(out, "{v}")` straight into `out`, `format_f64` deleted,
same `Display` impl so identical bytes by construction. Review
found zero blockers and the Post-006 entry
(`TESTING.md:345-372`) records improvement at every scale
(−6.3% / −29.8% / −26.3% vs 0.200ms, 1.886ms, 17.07ms;
200k float-vs-int gap ~1.2x).

The 006 deliverable has not had the foundation pass that
`src/zip.rs` (cycle 004) and the `xlsx.rs` tail (cycle 005)
went through. Open tune-ups, all confined to the 006 surface
(imports, 006-touched comments, the value-format spelling,
record format):

- Import placement: `src/emit.rs:5-6` stacks
  `use crate::contract::NULL_I64;` above
  `use std::fmt::Write as _;` with no grouping, while
  `src/xlsx.rs:9-13` sets the house convention (std group
  first, blank line, crate group, everything itemized).
- Comment minimalism: the 006 delta added a three-line
  construction note on `f64_cell` (`src/emit.rs:121-123`)
  plus kept the one-line arm reason (`:137-139`). Only
  forced comments survive a foundation pass.
- Spelling speed: `write!(out, "{v}")` was picked by
  construction-identity, never measured against the other
  same-`Display` spellings.
- Record lesson: the 006 `REVIEW.md` R4 minor note (20k
  criterion change −35.0% read against its stored ~2.04ms
  baseline, not the recorded 1.886ms Post-005 mean) asks
  for a dual-baseline rule going forward.

Binding references: `STYLE.md`, `DESIGN.md`, `TESTING.md`
(proof before fix; measure, don't assume — including this
spec's own suspicions, which the proofs confirm or refute).

## Requirements (proof first; each item testable)

- **R1 — Comment minimalism on the 006 surface.** Only the
  006-touched comment lines are in scope (`f64_cell` doc,
  `src/emit.rs:120-124`, and the `is_err` arm reason,
  `:137-139`). Each surviving line must state something no
  compiler-checked artifact states (domain contract,
  unreachability reason, pinning suite). The arm keeps its
  one-line reason per 006 R2 — that comment is forced, not
  minimalism-optional. Test: reviewer lists every
  006-touched comment line with keep/remove and the check
  covering each removal; full suite green unmodified.
- **R2 — Import placement matches the house convention.**
  The `std::fmt::Write` import joins the `src/xlsx.rs:9-13`
  grouping (std group first, blank line, crate group),
  itemized, no glob (STYLE rule 3). Test: inspection plus
  `cargo fmt --check` clean and `clippy -D warnings` clean;
  suite green with identical bytes (imports are not
  behavior).
- **R3 — Fastest byte-identical spelling wins, identity
  first.** Candidates are confined to same-`Display`-impl
  spellings (e.g. `write!(out, "{v}")`,
  `write!(out, "{}", v)`, the old
  `out.push_str(&v.to_string())` as baseline). Each
  candidate proves zero mismatches over a documented value
  corpus at least as broad as the 006 reviewer's (signed
  zero, 1/3, 0.1, 123.456, 1e21, 1e-7, 1e300, 5e-324,
  `MAX`, min-subnormal, ±1000 quarter-steps, inside the
  `<v>…</v>` wrapper; NaN-blank and infinity-refusal paths
  pinned by the unmodified suite) in a `/tmp` scratch
  probe — before any timing. Only proven-identical
  candidates are benched. Numbers pick the winner; a
  tie or within-noise margin keeps the current spelling.
- **R4 — Numbers recorded, no regression.** Re-take
  `cells/tall_floats_1000`, `tall_floats_20000`,
  `tall_floats_200000` per the `TESTING.md` bench protocol
  (bench profile inheriting `--release` with
  `overflow-checks = true`, `codegen-units = 1`; criterion
  scored harness, not test-only mode; estimates mtime
  confirmed moved; direct-binary runs pass `--bench` with
  the single filter). The Post-007 entry holds machine,
  profile, and commit, criterion means with criterion's own
  change%, output bytes (unchanged 003 recipes), and both
  baselines for every change% (criterion's stored baseline
  and the recorded Post-006 means 0.187ms, 1.324ms,
  12.57ms). No regression vs Post-006 at any scale or the
  cycle does not ship — the only allowed delta is a faster
  R3 winner with identical bytes. Every perf claim states
  machine, profile, and commit or it is not a measurement
  (STYLE rule 9).
- **R5 — Record hygiene.** Reviewer writes `REVIEW.md`
  with a verdict per item above, each tied to an inspected
  file and line plus gate evidence (`cargo fmt --check`
  clean, `sh check.sh` with `CARGO_HOME=/tmp/cargo-home` —
  the suite's round-trip, verbatim-embed, golden, and pin
  tests are the byte-identity proof, unmodified).
  Applicator writes `DECISION.md` (what changed, what was
  rejected and why, measured numbers). No deferral unless
  tied to a named future pass with its owner — not vague
  later (e.g. ingress/signature shape belongs to the
  template cycle; anything beyond `Display` such as a
  direct `ryu`/`dtoa` backend belongs to a named cell
  cycle).

## Scoped files

`src/emit.rs` (006-touched lines only: the two `use`
lines, the `f64_cell` doc, the value-format spelling;
integer path, `col_letters`, `escape_into`,
`header_cell` untouched), `TESTING.md` (Post-007 entry
only), `specs/007-f64-tuneup/` records by role.
`tests/`, `benches/`, `examples/`, `src/xlsx.rs`,
`src/zip.rs`, `STYLE.md`, `DESIGN.md` untouched: same API,
same bytes — a winning R3 spelling still ships identical
bytes, pinned by the unmodified suite. Equivalence probes
live in `/tmp` scratch; no new in-tree tests or benches.

## Acceptance

- Gate green (`cargo fmt` clean, `sh check.sh` with
  `CARGO_HOME=/tmp/cargo-home`).
- Byte-identity: round-trip, verbatim-embed, golden, and
  pins green unmodified — float bytes unchanged or the
  cycle does not ship.
- Floats re-taken and recorded per R4 with machine,
  profile, and commit; no regression vs Post-006.
- Reviewer verdict per item in `REVIEW.md`; applicator
  reasoning in `DECISION.md`; everything left uncommitted.

## Boundaries (not this cycle)

- No API change (`f64_cell` keeps `row: &str`).
- No scientific notation: the positional ruling stands.
- No template work, no per-cell match or bounds-check
  changes, no new dependencies (`std::fmt` only).
- No touching the integer path or any module beyond the
  006 surface; no `STYLE.md` register change unless the
  reviewer rules one.
- Direct-binary bench runs pass `--bench` with the single
  filter; the estimates mtime must move before any number
  is quoted (protocol lesson, TESTING).

## Workflow (on `go`)

Producer, reviewer, applicator as three independent agents,
one role each, per `specs/README.md`: the producer gets
this SPEC plus `STYLE.md`, `DESIGN.md`, `TESTING.md`, and
nothing else; the reviewer reads code and spec, builds,
runs probes and tests, and creates only `REVIEW.md` (the
source tree stays byte-identical, verified by diff); the
applicator applies required changes and writes
`DECISION.md`, leaving everything uncommitted. No agent
reviews its own work.
