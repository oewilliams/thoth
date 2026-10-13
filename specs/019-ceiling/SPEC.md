# Cycle 019 — ceiling verdict from witnessed lengths

The zip32 ceiling verdict in `build_workbook` is taken on
`sheet_cap`, the worst-case projection: every `f64` cell is
priced at 327 bytes. A real workbook from a work spec sheet
(three sheets, 8,880 rows × 1,298 columns, mostly `0.0`
buckets; ~311 MB of sheet XML per sheet, 110 MB deflated on
disk) projects to 12.2 GB and is refused with `TooLarge`
before a byte is emitted, while its actual stored length is
~933 MB — comfortably under the ceiling it is refused by. A
refusal must be a witnessed fact, not a projection artefact.
This cycle moves every sheet-dependent ceiling check to the
actual length at the point where the length is narrowed to
a zip field. Nothing about the emitted bytes changes.

Builds on the current working tree (cycle 018, uncommitted,
on HEAD `3c1c266`). Byte-identity in this cycle is measured
against the pre-cycle working tree, styles part included.

Depends on 014 (arena, backpatched headers, per-entry
`(name, crc, len, offset)` records) and the boundary framing:
refuse loudly, never truncate — that does not change. What
changes is which length the refusal is allowed to cite.

## Decisions pinned before the spec

- **`sheet_cap` stops being a refusal input.** It survives
  in its two honest jobs: the oracle's `String` capacity in
  `emit_sheet`, and the `sheet_cap_holds` proof that the
  actual never exceeds it. `build_workbook`'s ceiling verdict
  no longer consumes it. The reservation still comes from
  `sheet_hint`; correctness never depended on either, and
  after this cycle neither does the verdict.
- **Every narrowing cast gets its cover from a witnessed
  length, checked at the cast.** Sheet length is checked
  where it is patched into the local header; each entry's
  local offset and the central-directory start and length
  are checked where they are written. The checked narrowing
  is one named spelling (a helper that takes a `usize` and
  returns the `u32` or the refusal), so the boundary can be
  unit-tested with synthetic lengths and no allocation. No
  `as u32` on a sheet-dependent length survives without that
  spelling on the line.
- **What is known exactly before emission is still checked
  before emission.** Entry count, entry name lengths, and
  the small parts' actual lengths and totals are exact at
  the boundary and keep their pre-emission refusal through
  `archive_fits`. Only the sheet-dependent checks move.
- **No estimate may refuse.** Neither `sheet_cap` nor
  `sheet_hint` may veto a build after this cycle. The price
  is named: a workbook whose actual archive passes 4 GB is
  now refused only after its bytes are emitted, so the arena
  grows past the ceiling before the refusal lands. That is
  the standing resource-death taxonomy (STYLE rule 8), not a
  new guard: Excel's row/column limits still bound the
  shape, and no invented pre-emission estimate replaces the
  witnessed check.
- **`TooLarge.bytes` stays a projected finished length,
  computed from witnessed lengths.** At a sheet refusal it
  is `stored_archive_len` over the actual entries up to and
  including the refusing entry — a lower bound on the
  finished archive, and the doc comment on the variant must
  say so. The error variant, its fields, and its message
  shape do not change.

## Requirements (each item testable)

- **R1 — The spec-sheet shape builds.** A workbook of three
  sheets at 8,880 rows × 1,298 float columns (overwhelmingly
  `0.0`, the `external/workbook_spec_sheet.md` §1 shape,
  simplified trunk) builds successfully and its stored
  length is under the zip32 ceiling while its `sheet_cap`
  projection is over it. Proved by an integration test
  shaped to keep the gate sane (blank-heavy float cells
  spell a few bytes against the same 327-byte cap price:
  projection over the ceiling, actual a few hundred MB at
  most, asserted as projection > ceiling ≥ actual), and by
  the full-shape bench probe in R5. The test asserts the
  projection/actual relationship it relies on, so a future
  cap change that silently untests it fails loudly.
- **R2 — Genuine overflow still refuses, at the cast.**
  Each sheet-dependent narrowing (sheet length, local
  offset, central start, central length) refuses with
  `TooLarge` when its witnessed length passes `u32::MAX`,
  covered by unit tests over the checked-narrowing spelling
  with synthetic lengths at and past the boundary — no
  multi-GB allocation in the gate. Existing `archive_fits`
  synthetic tests keep passing for the checks that stay
  pre-emission (count, names, small-part totals, central
  length over known entries).
- **R3 — No byte changes.** Every existing recipe's archive
  is byte-identical to the pre-cycle tree (all parts,
  styles included), at the scales the standing probes run.
  The ceiling verdict is the only behavioural change: some
  previously refused workbooks now build, and genuinely
  oversized ones refuse later with the same error.
- **R4 — No other verdict moves.** Boundary validation
  (names, rectangularity, infinities, text checks, Excel
  row/column limits) is untouched and still runs before any
  emission. The arena reservation is still the hint (or
  block growth); a build that will ultimately refuse
  `TooLarge` reserves exactly what it reserves today.
- **R5 — Measured record.** Post-019 `TESTING.md` entry:
  the spec-sheet probe (three sheets, full §1 shape, data
  preparation outside the clock, `build_workbook` timed)
  recorded with cells, actual bytes, best time, MB/s, and
  peak RSS, beside the one-sheet reading; existing recipes
  shown unregressed on agreeing or interleaved reads;
  machine, profile, and the pre-cycle tree state (HEAD plus
  uncommitted 018) stated. No speed claim beyond the reads.
  The probe recipe lives in scratch, as before; whether a
  scaled spec recipe joins `benches/fill_write.rs` is the
  applicator's recorded call, not a silent addition (a
  stored 933 MB write per criterion iteration is not a
  bench, and saying so in the entry is a valid outcome).
- **R6 — Docs catch up.** `DESIGN.md`'s Templates section
  stops describing `sheet_cap` as part of the ceiling
  story: it is the oracle's capacity and a proof, the
  ceiling verdict is witnessed at the narrowing casts, and
  the Floor/Ingress wording that cites the projection is
  corrected to match. Cast-cover comments in `src/xlsx.rs`
  and `src/zip.rs` are rewritten to name the witnessed
  check each cast relies on — comments stay minimal and
  name no cycle or plan.

## Scoped files

`src/xlsx.rs` (build verdict, checked narrowings at the
emission points, cast-cover comments), `src/zip.rs`
(narrowing helper and header/central signatures if they
move), `src/contract.rs` (`sheet_cap` doc only — its
arithmetic and its oracle use do not change),
`src/xlsx/tests.rs` and `tests/` (synthetic narrowing
tests, the projection-vs-actual integration build),
`DESIGN.md`, `TESTING.md` (Post-019 entry),
`specs/019-ceiling/` records by role. No new dependencies.
No emitter, driver, macro, or styles changes.

## Boundaries (not this cycle)

- No deflate, no zip64, no streaming: the archive still
  lands whole in the arena and the ceiling is still zip32.
  A stored AssetCashflows at ~933 MB against 110 MB
  deflated is the deflate cycle's problem, separately.
- No change to `sheet_cap`'s arithmetic, to the hint, or
  to what the oracle allocates. Tightening the float cap
  price is a different idea and is not smuggled in here.
- Cycle 018 is not reviewed or committed by this cycle;
  it is the base tree, and this cycle's diff must be
  separable from it in the record (`REVIEW.md` states
  which hunks belong to which cycle where they share a
  file).

## Workflow (on `go`)

Producer, reviewer, applicator as three independent agents,
one role each, per `specs/README.md`. The producer gets this
SPEC plus `STYLE.md`, `DESIGN.md`, `TESTING.md`, and nothing
else. Everything left uncommitted.
