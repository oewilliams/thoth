# Cycle 012 — phase bench (emit / CRC / assembly / write, separately)

Phase 2 of 3 (011 multi-sheet → 012 phase bench → 013 fused emit).
Harness and measurement only: no library byte changes, no library
behaviour changes. 012 exists so 013 removes only the passes the
numbers indict — STYLE rule 9 applied to the foundation question
itself ("below the threshold, bytes rule", and a claim about speed
that cannot point at fewer passes is aesthetics).

Depends on 011 applicated and green: the phase recipes include
the 3-sheet bench example 011 adds, so the fused design in 013 is
aimed at the multi-sheet shape, not the retired single-sheet one.

## Background

Today every scored number times one fused call:
`write_xlsx_file` = sheet emission (`sheet_xml` formatting cells
into a `String`, `src/xlsx.rs:236-301`) + CRC (`ZipWriter::add`
checksums each staged part, `src/zip.rs:192-210`) + assembly
(`ZipWriter::finish` copies every part into the archive buffer,
`src/zip.rs:215-282`) + the sealed file write
(`std::fs::write` + rename, `src/xlsx.rs:359-382`). The shipped
micro-opts each attacked one measured layer — cycle 004 replaced a
5.8 ns/B bitwise CRC with the 0.083 ns/B instruction spelling
(`src/zip.rs:96-107`, ~11 GB/s), cycle 005 removed per-cell row
formatting, cycle 006/007 settled the `f64` spelling — and the
recorded tall rates at HEAD (Post-010: floats ~0.93 GB/s, ints
~1.58 GB/s output) still sit well below both that CRC rate and
the disk, with no current measurement saying which remaining pass
owns the gap. Guessing is how a foundation gets rebuilt for no
gain; this cycle replaces the guess with a table.

The four phases are separately observable through existing public
spelling, no library change needed: `sheet_xml` alone is
emission; `build_workbook` minus `sheet_xml` time is CRC +
assembly (the parts are staged and copied inside it);
`write_workbook_file` minus `build_workbook` is the file write
plus seal checks. Two refinements keep the subtractions honest:
the `crc` criterion group already isolates CRC over fixed buffers
(`benches/fill_write.rs`, `crc32_4k`/`crc32_1m`), and the `floor`
group's `tall_null` recipes isolate skeleton emission (blank cells
skip value formatting). One known trap, named now so the reviewer
can check it: separate criterion benches subtract across processes
and allocator states, so phase shares are reported as ranges over
agreeing reads, never as single-point percentages.

Binding references: `STYLE.md` rules 4, 5, 9; `DESIGN.md` Floor
("a benchmark showing a faster way overrules this"); `TESTING.md`
protocol and the cycle-010 spell-agreement rule (two agreeing
quiet-spell reads, tight CIs, severe outliers ≤ 10%, box state at
200k).

## Requirements (each item testable)

- **R1 — Phase recipes beside the existing groups, same scales.**
  `benches/fill_write.rs` gains one group (name producer's,
  recorded in `REVIEW.md`) timing, per recipe: (a) emission alone
  — `sheet_xml` per sheet, summed over the workbook's sheets,
  result `black_box`ed, no file touched; (b) build alone —
  `build_workbook`, no file touched; (c) the existing end-to-end
  `write_xlsx_file`/`write_workbook_file` numbers, re-used from
  the `fill_write`/`cells` groups, never re-implemented. Recipes:
  `tall_ints`, `tall_floats`, `tall_null` (floor recipe) at
  20_000 and 200_000, plus the 011 3-sheet example at its bench
  scale. Setup (column construction, debris removal) stays out of
  the clock per cycle 010; emission recipes allocate exactly as
  the library does (the allocation is part of the phase cost and
  is named as such, not hidden). Test: reviewer lists every
  statement in each timed closure; `cargo bench` compiles clean;
  no recipe, scale, or existing group changed.
- **R2 — Phase table with shares as ranges.** The Post-012
  `TESTING.md` entry holds, per recipe: emission ms, build-minus-
  emission ms (CRC + assembly), write-minus-build ms (file +
  seal), end-to-end ms — each as two agreeing reads with CIs,
  machine, profile, commit, and 200k box state per protocol —
  plus the implied per-phase output rates (MB/s over the known
  output bytes). Cross-checked against the two independent
  anchors: the `crc` group rate applied to the sheet byte count
  predicts the CRC share of (build − emission), and the
  `tall_null` floor rate bounds skeleton emission; where a
  subtraction disagrees with its anchor beyond the CIs, the entry
  says so and trusts the anchor (direct measurement beats
  subtraction). Test: entry present, arithmetic shown, every
  share a range, anchors quoted beside the subtractions.
- **R3 — The 013 decision, stated as a gate.** The entry closes
  with the phase ranking and an explicit go/no-go per candidate
  fusion 013 may perform: (i) emit-into-archive (removes the
  sheet→archive copy and the sheet allocation), (ii) incremental
  CRC during emission (removes the post-emission CRC pass),
  (iii) neither, if emission itself owns the time and the next
  target is the emitter (digit/float spelling, per-push cost).
  A fusion is `go` only if its phase share is ≥ 15% of end-to-end
  on at least two recipes at 200k on agreeing reads; below that
  it is recorded `no-go` and 013's scope shrinks accordingly (the
  applicator of 013 records the shrink in its `DECISION.md` —
  shipping a measured no-op foundation change fails STYLE rule 9
  even if the code is prettier). The 15% line is this spec's
  ruling, argued here once: cycle 004's CRC pass was ~75% of the
  tall time and cycle 005's row formatting ~35% — every shipped
  foundation win was a large share; small-share fusions have
  never paid for their risk on this codebase. Test: entry states
  go/no-go per fusion with the share arithmetic; `REVIEW.md`
  confirms the shares support the verdicts.
- **R4 — Peak-memory observation, one probe.** A `/tmp` scratch
  probe (never in-tree) reports peak RSS (or allocator high-water,
  method named) for `build_workbook` vs end-to-end write on the
  200k 3-sheet recipe, quantifying the sheet-plus-archive double
  residency 013 would remove. Scratch only; the figure is
  recorded as a probe result with its method, not as a bench.
  Test: Post-012 entry quotes the probe figure and method;
  reviewer re-runs the probe shape from its description.
- **R5 — Byte-identity, library untouched.** `src/` diff empty
  (comments included), existing tests green unmodified, output
  bytes unchanged (003 recipe byte-size probe still matches).
  Test: `sh check.sh` green; `git diff --stat src/ tests/`
  empty.

## Scoped files

`specs/012-phase-bench/SPEC.md` (this file) now; on `go`:
`benches/fill_write.rs` (one new group only), `TESTING.md`
(Post-012 entry only), `specs/012-phase-bench/` records by role.
Untouched: `src/`, `tests/`, `examples/` (unless the reviewer
rules the phase timing belongs in `examples/bench.rs` instead —
one home, not both), `Cargo.toml`, `Cargo.lock`, `STYLE.md`,
`DESIGN.md`, `README.md`. Probes in `/tmp` scratch. No new
dependencies, no new in-tree tests.

## Acceptance

- Gate green (`cargo fmt --check`, `sh check.sh`).
- Phase table complete per R2 with agreeing reads and anchors;
  library diff empty.
- Explicit go/no-go per 013 fusion with share arithmetic (R3).
- Reviewer verdict per item in `REVIEW.md`; applicator reasoning
  in `DECISION.md`; everything left uncommitted.

## Boundaries (not this cycle)

- No library change of any kind — not the sink, not the CRC, not
  a comment. If a phase cannot be observed without one, the
  reviewer records that as a finding for 013's spec, and 012
  still ships its table from the observable phases.
- No new bench recipes or scales beyond R1; no criterion upgrade.
- No conclusion stated without its machine, profile, and commit;
  no cross-run ratio without its spread (STYLE rule 9).

## Workflow (on `go`)

Producer, reviewer, applicator as three independent agents, one
role each, per `specs/README.md`: the producer gets this SPEC plus
`STYLE.md`, `DESIGN.md`, `TESTING.md`, and nothing else; the
reviewer reads code and spec, builds, runs probes and tests, and
creates only `REVIEW.md` (the tree byte-identical before and
after outside scoped files, verified by diff); the applicator
applies required changes and writes `DECISION.md`, leaving
everything uncommitted. No agent reviews its own work. 013 does
not start until 012's R3 gate is recorded.
