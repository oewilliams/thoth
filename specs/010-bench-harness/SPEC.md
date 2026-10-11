# Cycle 010 — bench harness harden (setup out of the clock, spell rule, box state)

## Background

Post-009 diagnosed the 200k-scale variance, not a code effect. Same-binary
reads across the cycle-009 spell spanned 8.25–10.64ms (`cells/tall_ints_200000`,
recorded mean 8.248ms, −19.0% vs Post-005 — claimed as machine spread, no
speedup claimed). Steady-state spread is 5–10%. A scratch split-probe in
`/tmp/varprobe` showed two contaminations: cold-start effects plus transient
busy spells on a 42-day-uptime box. And the harness itself leaks setup into
the clock: `write_iter` (`benches/fill_write.rs:26-34`) runs debris removal
(both `remove_file` calls) and column construction (`example.writer_columns()`
— per-column `name.clone()` plus the `Vec` build, `tests/common/mod.rs`)
inside the timed closure, so per-iteration filesystem and allocator work
rides the score. `examples/bench.rs:13-28` has the same shape: debris removal
and `writer_columns()` sit between path build and the clock stop.

This cycle hardens the harness only. No library change, no byte change, no
new dependency. The number it produces is a trustworthy 200k re-take under
the hardened harness, with the spread demonstrably narrowed.

Binding references: `STYLE.md` (rules 8, 9 — `unwrap` lives only in
benches/examples/tests; every perf claim states machine, profile, commit),
`DESIGN.md`, `TESTING.md` (Post-006/007/009 documentation standard for
superseded noisy passes).

## Requirements (each item testable)

- **R1 — Setup leaves the timed section in both harnesses.** In
  `benches/fill_write.rs`, `write_iter` moves per-iteration debris removal
  (both `remove_file` calls) and column construction (`writer_columns()`)
  into criterion setup (`iter_with_setup` or the equivalent the reviewer
  accepts), so the clock covers exactly the `write_xlsx_file` call —
  fill-plus-write only. Post-iteration debris removal becomes teardown
  outside the clock. In `examples/bench.rs`, debris removal and
  `writer_columns()` move above `Instant::now()`; the clock covers
  `writer_columns()`-plus-`write_xlsx_file` only if the producer shows
  `writer_columns()` is fill, otherwise the `write_xlsx_file` call alone —
  the reviewer confirms which, and the timed lines are named in `REVIEW.md`.
  No other bench-group, recipe, or scale change: same groups (`fill_write`,
  `floor`, `cells`, `crc`), same examples, same counts. Test: reviewer lists
  every statement inside each timed closure/linespan and shows none touches
  the filesystem or constructs columns; `cargo bench` compiles clean.
- **R2 — Spell-agreement rule formalized and followed.** A number is
  recorded only after two agreeing quiet-spell reads: same binary, same
  spell, means within 3% of each other, each with a tight interval (CI half
  width under 5% of the mean) and severe-outlier share at or under 10%.
  Noisy first passes (wide CIs, outlier share above 10%, means drifting
  across spells) are discarded and documented to the Post-006/007/009
  standard — discarded means, intervals, and outlier shares quoted in the
  `TESTING.md` entry, never silently dropped. The rule itself is written
  into `TESTING.md` (benchmarks section) so cycle 011 inherits it as
  protocol, not lore. Test: Post-010 entry quotes two agreeing reads per
  recorded bench; every discarded pass appears with figures and a supersede
  note; the standing rule text is present in `TESTING.md`.
- **R3 — Box state recorded alongside the 200k numbers.** Every 200k
  re-take in the Post-010 entry carries the box state at bench time:
  system uptime (days, from `uptime` or `kern.boottime`), 1-minute load
  average, and free pages (`vm_stat` free plus inactive, or
  `/proc/meminfo` `MemAvailable` on Linux). Small scales need no box state.
  Test: Post-010 entry shows uptime, load, and free pages beside each 200k
  figure; reviewer re-runs the stated commands and confirms the shape.
- **R4 — Byte-identity, suite green unmodified.** Harness changes must not
  move a single output byte. Full gate suite passes with no test edited:
  `tests/roundtrip.rs` `examples_round_trip_through_disk` (round-trip
  values plus the per-example stored-zip-verbatim-embed check, the floor
  oracle), `src/zip.rs` `golden_small_archive_is_byte_identical`,
  `src/xlsx.rs` `static_parts_are_pinned` and `sheet_cap_holds`,
  `src/emit.rs` spelling/null/letter tests, and the `tests/sealed.rs`
  locks. The byte-size probe of the bench-scale recipes still matches the
  003 figures byte for byte (48001 / 50002, 990005 / 1030006,
  10470009 / 10870010). Test: `sh check.sh` with
  `CARGO_HOME=/tmp/cargo-home` green, tests unmodified (`git diff --stat
  tests/ src/` empty).
- **R5 — 200k re-take under the hardened harness, spread narrowed.**
  Re-take `cells/tall_ints_200000`, `cells/tall_floats_200000`, and the
  `floor/tall_null_200000` control under the R1 harness per the
  `TESTING.md` bench protocol (bench profile inheriting `--release` with
  `overflow-checks = true`, `codegen-units = 1`; criterion scored harness,
  not test-only mode; estimates mtime confirmed moved; direct-binary runs
  pass `--bench` with the single filter). The Post-010 entry shows both
  agreeing reads per bench in absolute figures (ms means, intervals,
  output bytes, MB/s) with machine, profile, and commit, and the spread
  narrowed against the Post-009 baseline: the two 200k-int reads must span
  under 1.0ms absolute (Post-009 same-binary span was 2.39ms, 8.25–10.64ms)
  with tight intervals. No speedup is claimed unless the agreeing reads
  sit outside the old interval — within noise, the result is "same speed,
  tighter clock". Test: Post-010 entry present with all figures, box
  state, both baselines (recorded means: ints 10.18ms Post-005 / 8.248ms
  Post-009; floats 12.553ms Post-009; floor 10.062/10.108ms Post-009),
  and the narrowed span arithmetic shown.

## Scoped files

`specs/010-bench-harness/SPEC.md` (this file) by the producer now; on `go`:
`benches/fill_write.rs` (R1 harness shape only — groups, recipes, scales
untouched), `examples/bench.rs` (R1 clock lines only), `TESTING.md`
(spell-agreement rule text plus the Post-010 entry only),
`specs/010-bench-harness/` records by role. Untouched: `src/` (the whole
library — same API, same bytes), `tests/`, `Cargo.toml`, `Cargo.lock`,
`STYLE.md`, `DESIGN.md`. Probes live in `/tmp` scratch; no new in-tree
tests or benches; no new dependencies.

## Acceptance

- Gate green (`cargo fmt --check` clean, `sh check.sh` with
  `CARGO_HOME=/tmp/cargo-home`).
- Byte-identity: round-trip, verbatim-embed, golden, and pins green
  unmodified, byte-size probe matching 003 figures — one changed byte
  fails the cycle.
- Timed sections contain exactly the fill-plus-write call (reviewer-listed).
- Post-010 entry holds two agreeing reads per 200k bench with absolute
  figures, box state, machine/profile/commit, discarded passes documented,
  span narrowed vs Post-009.
- Reviewer verdict per item in `REVIEW.md`; applicator reasoning in
  `DECISION.md`; everything left uncommitted.

## Boundaries (not this cycle)

- No library change of any kind (`src/` untouched — not even comments).
- No bench recipe, scale, group, or criterion-version change; no new
  harness dependency.
- No `#[inline]` or codegen touch; no `STYLE.md` register change unless
  the reviewer rules one.
- No speedup claimed from the harness change alone; the 200k-int −19.0%
  stays labeled machine spread until agreeing reads say otherwise.
- Direct-binary bench runs pass `--bench` with the single filter; the
  estimates mtime must move before any number is quoted (protocol lesson,
  TESTING).

## Workflow (on `go`)

Producer, reviewer, applicator as three independent agents, one role each,
per `specs/README.md`: the producer gets this SPEC plus `STYLE.md`,
`DESIGN.md`, `TESTING.md`, and nothing else; the reviewer reads code and
spec, builds, runs probes and tests, and creates only `REVIEW.md` (the
source tree stays byte-identical outside the scoped harness files,
verified by diff); the applicator applies required changes and writes
`DECISION.md`, leaving everything uncommitted. No agent reviews its own
work.
