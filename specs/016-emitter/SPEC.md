# Cycle 016 — emitter (the spellings, measured one at a time)

Emission is now the only pass over the bytes: the arena (014)
removed the re-reads and copies, and the template contract (015)
locked who calls the emitters. This cycle tunes the emitter
spellings themselves in `src/emit.rs`, one candidate at a time,
each entering only with its measurement. The two emission
drivers (oracle and arena) stay exactly as they are — the human
reviewed them and they are not restructured here.

Depends on 015 committed. Post-015 baselines at 200k (build):
ints 3.83–3.92 ms, floats 8.90–8.93 ms, null 4.45–4.52 ms;
oracle emission: ints ~3.6 ms, floats ~9.3 ms, null ~4.2 ms.

## Already measured and rejected (do not retry without a new idea, named)

- Precomputed per-column cell prefix: 6–17% slower on every
  recipe (runtime-length appends lose to constant-length ones).
- Blank cell assembled in a 24-byte stack buffer, one append:
  ~25% slower on nulls, ints regressed (014 applicator).
- Branch-first blank/value split duplicating the cell open:
  nulls +23%, ints +5–8% (014 applicator).
- The `f64` spelling itself was settled in cycles 006/007
  (write-through vs backends, byte-identity plus bench, tie kept
  the incumbent). Reopening it needs a candidate those cycles
  did not measure, named as such.

## Candidates (each independently gated)

- **C1 — Float append shape.** `f64_cell_bytes` writes through
  `fmt::Write` over the arena, and `Display` delivers the value
  in several `write_str` pieces, each an arena append with its
  capacity check. Candidate: format into a fixed stack scratch,
  then one append of the finished spelling. Bytes must match
  `Display` exactly over the cycle-007 probe set (specials, both
  zeros, subnormal boundary, quarter-steps, 301-digit extremes),
  pinned by extending `byte_sink_matches_string_oracle`.
- **C2 — Blank path.** The recorded null deficit (~1–3% at
  emitter level, accepted in 014 at build level) — reduce the
  blank cell's append count without the rejected stack-assembly
  or branch-first shapes. A candidate that does not beat the
  deficit's noise band is recorded and reverted, not carried.
- **C3 — Digit path on the oracle.** The byte digit spelling
  already appends once per value; the `String` oracle still
  pushes digit-by-digit as chars. If the oracle is to stay a
  meaningful parity benchmark for 016's successors, respell its
  magnitude push as one slice append with identical bytes
  (`int_spelling_matches` sweep unchanged). Build-path bytes
  cannot change.
- **C4 — `unsafe` cursor, only behind a number.** If, after
  C1–C3, a remaining candidate is writing digits or fragments
  into the arena's spare capacity without per-append checks, it
  enters only with a measurement naming the nanoseconds it buys
  over the best safe spelling, per the cycle-004 precedent. No
  measurement, no `unsafe` — and the spec records the refusal
  either way in `DECISION.md`.

## Requirements (each item testable)

- **R1 — Byte-identity is absolute.** Every candidate keeps the
  archive dumps at 200k identical to HEAD on ints, floats, null,
  mixed, and three-sheet recipes (`cmp` against `git archive`
  dumps as in Post-014/015), and the emitter unit sweeps pass
  unmodified in what they assert.
- **R2 — One candidate, one measurement.** Each candidate ships
  only if it beats the incumbent spelling on interleaved A/B
  (rotating order, medians, per the 014/015 method finding;
  sequential Criterion reads on this box are not evidence for
  close calls) at 200k on the recipe it targets, without
  regressing the others beyond noise. Losers are reverted and
  the revert recorded with its numbers — a measured no-op
  spelling change does not ship (STYLE rule 9).
- **R3 — Drivers untouched.** `emit_sheet` and
  `emit_sheet_arena` in `src/contract.rs` keep their structure,
  loop shape, and CRC chunking; call sites change only if a
  shipped spelling's signature forces it, and each such change
  is listed in `DECISION.md`. No template, arena, ceiling, or
  error-semantics change.
- **R4 — Post-016 record.** `TESTING.md` entry with per-candidate
  verdicts (shipped/reverted with numbers), final build reads
  against the Post-015 baselines on agreeing or interleaved
  reads, machine, profile, and commit stated. Speed is claimed
  per candidate or not at all.

## Scoped files

`src/emit.rs`, `src/contract.rs` only for forced call-site
changes under R3, `TESTING.md` (Post-016 entry),
`specs/016-emitter/` records by role. Untouched:
`src/xlsx.rs`, `src/workbook_parts.rs`, `src/zip.rs`,
`src/macros.rs`, `src/static_parts.rs`, examples, manifests.
No new dependencies. Public emitter signatures change only if
recorded in `DECISION.md` with in-tree callers migrated.

## Workflow (on `go`)

Producer, reviewer, applicator as three independent agents, one
role each, per `specs/README.md`. The producer gets this SPEC
plus `STYLE.md`, `DESIGN.md`, `TESTING.md`, and nothing else.
Everything left uncommitted.
