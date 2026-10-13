# Cycle 020 — arena access patterns (layout round two)

Cycle 014 chose the arena; cycles since have tuned spellings
inside it. This round asks whether the layout itself is
getting everything the hardware offers: how the arena is
touched, in what order, at what working-set size, and how
much work each instruction carries. It runs before any
number-format work (the last spec-sheet feature family) and
before deflate (absolutely last), so its results are
measured against the stored floor while the bytes are still
diffable. Every candidate must produce byte-identical
output; a layout that cannot is not a layout, it is a
different writer.

Base tree: HEAD `a0fddc9` (cycles 018–019 committed).

## Standing rules for this round

- **Experiments live in scratch first** (`/tmp`, as in 014
  and 019), as switchable variants inside one probe binary
  wherever the candidate allows it. The 017 lesson stands:
  cross-binary comparisons cannot separate layout drift
  from cost, so a claim needs interleaved or within-binary
  reads — two agreeing reads per the standing spell
  protocol — before anything lands in tree.
- **Access patterns are evidenced, not asserted.** Each
  shipped change names its instruction-level claim and
  backs it with the emitted assembly for the loop it
  touches (bytes per load/store, wide/NEON use, call
  traffic), plus the interleaved timing. Neither half
  stands alone: vectorised is not fast (the 004 lesson),
  and fast without a machine-level why is not understood.
- **Tunable sizes are measured on this target, never
  inherited.** Tile widths, staging sizes, and fold
  granularities are swept; the shipped value is the sweep's
  winner with its reads recorded (STYLE rule 5).
- **`unsafe` only with a measured figure naming what it
  buys** (standing rule from 016). Safe SWAR and wide-copy
  spellings are exhausted first.
- **Past the boundary, payload is bits.** No candidate may
  classify values to shortcut their spelling (the float
  fast paths refused in 016 stay refused). Layout changes
  move bytes; they do not inspect what the bytes mean.

## Candidates (each accepted or rejected on its own reads)

- **E1 — Witnessed reservation.** The boundary scans
  already walk every string (validity, cap) and every
  float (infinities). Accumulate the exact escaped byte
  length of string payload during the scan that already
  visits those chars, and feed the reservation a witnessed
  size in place of the per-kind expected widths where a
  witness exists. Measure retained capacity and peak RSS
  per recipe, and prove the scan does not slow (it must
  ride the existing pass, never add one).
- **E2 — Row working set.** Today each row is formatted
  straight into the growing arena and CRC-folded over the
  arena range. Candidates: (a) stage one row in a small
  reused buffer, append it once, fold over the staged
  bytes while they are L1-hot; (b) the status quo. Sweep
  on narrow (tall) and wide (1,298-column) shapes — the
  winner may differ by shape, and a shape-dependent winner
  must be chosen by a declared template fact (column
  count), never discovered mid-emission. The extra copy in
  (a) is presumed guilty; it ships only if the reads clear
  it.
- **E3 — Wide-sheet gather.** The row loop strides across
  every column array per row. With byte order fixed
  (complete rows, in order), the levers are prefetch
  distance and the order columns are first touched.
  Safe-Rust prefetch spellings only; measure on the
  spec-sheet bucket shape and the liability shape.
  Already measured and not re-run without a new
  hypothesis: per-column prefix slices (016, slower —
  runtime-length appends lose to constant-length ones).
- **E4 — Escape scan width.** String and header escaping
  walks char by char. Candidate: SWAR scanning of the
  byte payload (eight bytes per word, safe Rust) for the
  five escapable characters, falling back to the current
  per-char spelling around hits, with the same treatment
  for the boundary validity scan if the reads support it.
  Measure on long-string (multi-KB `CF:`-like), short
  string, and escapable-heavy recipes.
- **E5 — CRC fold granularity.** The fold is per emitted
  row. Sweep folding per K rows (and per byte threshold)
  against per row, on narrow and wide shapes: the fold
  must still touch bytes while they are hot, and the CRC
  result is identical by construction. The shipped
  granularity is the sweep's winner, stated with the row
  size it was tuned on.

## Recipes (the measuring set)

Existing in-tree benches (tall ints/floats/null at 200k,
mixed, strings/bools, styled/twin, three-sheet, floor)
plus the scratch spec-sheet shapes from
`external/workbook_spec_sheet.md`: buckets (8,880 × 1,298,
one and three sheets), ByAsset-like (8,880 × 104 with
multi-KB string columns), liability-like (567 × 1,918).
Data preparation is always outside the clock.

## Requirements (each item testable)

- **R1 — Byte identity.** Every recipe, in-tree and spec
  shape, byte-identical to the base tree, part by part.
- **R2 — Per-candidate verdicts.** Each of E1–E5 is
  recorded shipped or rejected with its interleaved reads
  (spreads stated), its assembly claim where it touches a
  hot loop, and the recipes it was swept on. A candidate
  that wins on one shape and loses on another ships only
  behind a declared template fact that selects it, or not
  at all. Rejected candidates are recorded with their
  numbers, as in 016 — a measured no is a result.
- **R3 — No regression by silence.** Existing in-tree
  recipes are shown unregressed on interleaved reads
  against the base tree; any recipe that moves past its
  spread is named, not averaged away.
- **R4 — Measured record.** Post-020 `TESTING.md` entry:
  per-candidate verdicts and reads, retained capacity and
  peak RSS where E1 lands, machine, profile, and commit
  stated. No speed claim beyond the reads.

## Scoped files

Expected: `src/contract.rs` (drivers, reservation,
witness accumulation), `src/emit.rs` (escape and sink
spellings), `src/zip.rs` (fold granularity if E5 lands),
`TESTING.md`, `specs/020-layout/` records by role. Only
what a shipped candidate touches: a rejected candidate
leaves no code behind. No new dependencies, no format
features, no deflate, no boundary-semantics changes.

## Boundaries (not this cycle)

- No number formats, date formats, freeze panes, header
  variants, per-cell or mixed types, or PASS/FAIL fills:
  formatting is the last spec feature family and rides on
  whatever layout this round proves.
- No deflate. That remains absolutely last.

## Workflow (on `go`)

Producer, reviewer, applicator as three independent agents,
one role each, per `specs/README.md`. The producer gets this
SPEC plus `STYLE.md`, `DESIGN.md`, `TESTING.md`, and nothing
else. Everything left uncommitted.
